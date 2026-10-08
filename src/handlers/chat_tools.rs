use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{OPERACIONES, TIPOS};
use crate::repositories::ClienteRepository;
use crate::services::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar_outbox_idem, Encolado,
};
use glory_agent::errors::AgentError;
use glory_agent::session::ChatHub;
use glory_agent::tools::{ToolCtx, ToolExecutor};

/* [169A-4] Tools de la inmobiliaria (las ejecuta el loop F6 del núcleo).
 * Salidas compactas: cada token cuenta para la ventana de 30k. La IA nunca
 * inventa datos: precios/direcciones salen de `buscar/detalle`, el teléfono
 * de `datos_contacto` y el humano de `escalar_a_humano`. */

/* [08AA-6] Los schemas del provider viven en `chat_tools_definiciones`
 * (split god-object); se re-exporta `definiciones` para no romper `chat.rs`. */
pub(crate) use super::chat_tools_definiciones::definiciones;

/// Executor con acceso a BD y al contacto por defecto (`AGENTE_CONTACTO`).
pub struct Herramientas {
    pool: PgPool,
    contacto_defecto: String,
    /* [299A-4] Hub opcional para espejar en el hilo lo enviado al visitante
     * (tarjeta/foto): `ToolCtx` no trae hub y el núcleo es dependencia
     * externa, así que viaja en el executor que sí construimos nosotros. */
    hub: Option<ChatHub>,
}

impl Herramientas {
    pub fn new(pool: PgPool, contacto_defecto: String) -> Self {
        Self {
            pool,
            contacto_defecto,
            hub: None,
        }
    }

    /// Hub para el espejo 299A-4. Sin hub (tests) se encola igual pero no
    /// se espeja: el envío al visitante nunca depende del espejo.
    pub fn with_hub(mut self, hub: ChatHub) -> Self {
        self.hub = Some(hub);
        self
    }

    fn pool(&self, ctx: &ToolCtx) -> PgPool {
        ctx.pool.clone().unwrap_or_else(|| self.pool.clone())
    }

    async fn ejecutar(&self, name: &str, args: &Value, ctx: &ToolCtx) -> Result<Value, AgentError> {
        let inicio = std::time::Instant::now();
        let pool = self.pool(ctx);
        if deshabilitada(&pool, name).await {
            tracing::warn!(
                "tool {name} sesion={} bloqueada por admin ({} ms)",
                ctx.session_id,
                inicio.elapsed().as_millis()
            );
            return Ok(json!({"error": "herramienta deshabilitada por el administrador"}));
        }
        let salida = match name {
            "buscar_inmuebles" => buscar(&pool, ctx.session_id, args, self.hub.as_ref()).await,
            "detalle_inmueble" => detalle(&pool, args).await,
            "registrar_contacto" => registrar(&pool, ctx.session_id, args).await,
            "enviar_fotos_inmueble" => {
                enviar_fotos(&pool, ctx.session_id, args, self.hub.as_ref()).await
            }
            "registrar_captacion" => captar(&pool, ctx.session_id, args).await,
            "agendar_visita" => agendar(&pool, ctx.session_id, args).await,
            "datos_contacto" => contacto_publico(&pool, &self.contacto_defecto).await,
            "escalar_a_humano" => escalar(&pool, ctx.session_id, args).await,
            "consultar_agente" => consultar(&pool, ctx.session_id, args).await,
            otro => Ok(json!({"error": format!("tool desconocida: {otro}")})),
        };
        match &salida {
            Ok(v) => tracing::info!(
                "tool {name} sesion={} ok ({} ms, {} chars)",
                ctx.session_id,
                inicio.elapsed().as_millis(),
                v.to_string().len()
            ),
            Err(e) => tracing::warn!(
                "tool {name} sesion={} ERROR tras {} ms: {e}",
                ctx.session_id,
                inicio.elapsed().as_millis()
            ),
        }
        salida
    }
}

impl ToolExecutor for Herramientas {
    fn execute<'a>(
        &'a self,
        name: &'a str,
        args: &'a Value,
        ctx: &'a ToolCtx,
    ) -> Pin<Box<dyn Future<Output = Result<Value, AgentError>> + Send + 'a>> {
        Box::pin(async move { self.ejecutar(name, args, ctx).await })
    }
}

/// ¿La apagó el admin en `tools_deshabilitadas` (csv)? Sin config = todas on.
async fn deshabilitada(pool: &PgPool, name: &str) -> bool {
    let csv = glory_agent::persistence::get_config(pool, "tools_deshabilitadas")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    csv.split(',').map(str::trim).any(|t| t == name)
}

async fn buscar(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
    hub: Option<&ChatHub>,
) -> Result<Value, AgentError> {
    let texto = args
        .get("texto")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let tipo = args
        .get("tipo")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(t) = tipo {
        if !TIPOS.contains(&t) {
            return Ok(json!({"error": "tipo invalido"}));
        }
    }
    let operacion = args
        .get("operacion")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(o) = operacion {
        if !OPERACIONES.contains(&o) {
            return Ok(json!({"error": "operacion invalida"}));
        }
    }
    let precio_max = args
        .get("precio_max")
        .and_then(Value::as_f64)
        .filter(|p| *p > 0.0);
    /* [Fase3-H3] Habitaciones exactas en BD (antes el modelo filtraba a ojo
     * y colaba 3 hab cuando pedían 2). Solo enteros positivos; NULL/0 = sin
     * filtro. Ojo: filas con `habitaciones` NULL quedan fuera si se filtra. */
    let habitaciones = args
        .get("habitaciones")
        .and_then(Value::as_i64)
        .filter(|h| *h > 0);
    /* [Fase3-H4] Zona aparte de `texto`: el ILIKE sobre `ubicacion` no cubre
     * zonas que no aparecen literales ("norte" vs "Guayana Country Club").
     * Filtra lo que sí coincide y el prompt ordena reconocer el vacío. */
    let zona = args
        .get("zona")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let limite = args
        .get("limite")
        .and_then(Value::as_i64)
        .unwrap_or(5)
        .clamp(1, 10);
    /* [229A-1] Struct en vez de tupla de 9 (mismo patrón que `Ficha`):
     * legible y evita el lint de tipos complejos.
     * [Fase3-v2] `texto`/`zona` pasan por `sencilla()` (migración
     * `20260929000018`: minúsculas sin tildes): el visitante escribe
     * "Caroni" y la BD guarda "Caroní" (el ILIKE directo daba 0 filas y
     * la IA negaba oferta existente). */
    let filas: Vec<Tarjeta> =
        sqlx::query_as(
            "SELECT id, titulo, tipo, operacion, precio, ubicacion, slug, puestos, residencia, habitaciones \
             FROM inmuebles \
             WHERE publicado AND estado = 'disponible' \
             AND ($1::TEXT IS NULL OR sencilla(titulo) LIKE '%' || sencilla($1) || '%' OR sencilla(ubicacion) LIKE '%' || sencilla($1) || '%') \
             AND ($2::TEXT IS NULL OR tipo = $2) \
             AND ($3::TEXT IS NULL OR operacion = $3) \
             AND ($4::FLOAT8 IS NULL OR precio <= $4) \
             AND ($6::BIGINT IS NULL OR habitaciones = $6) \
             AND ($7::TEXT IS NULL OR sencilla(ubicacion) LIKE '%' || sencilla($7) || '%') \
             ORDER BY updated_at DESC LIMIT $5",
        )
        .bind(texto)
        .bind(tipo)
        .bind(operacion)
        .bind(precio_max)
        .bind(limite)
        .bind(habitaciones)
        .bind(zona)
        .fetch_all(pool)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let total = filas.len();
    /* [E-fluido F2] Tarjetas 1-propiedad-por-mensaje: el modelo tiende a soltar
     * la lista entera en un bloque (molesto de leer en WhatsApp), asi que el
     * backend encola hasta `MAX_TARJETAS` con formato fijo y devuelve
     * `tarjetas_enviadas` para que NO las repita: solo intro + cierre. Sin
     * telefono o sin canal WhatsApp no hay a donde enviarlas: `0` y el modelo
     * lista como antes (widget web).
     * Ojo: `filas` se consume abajo para `items`; las tarjetas van primero. */
    let (enviadas, repetidas) = encolar_tarjetas(pool, session_id, &filas, hub).await;
    let items: Vec<Value> = filas
        .into_iter()
        .map(|t| {
            json!({"id": t.id, "titulo": t.titulo, "tipo": t.tipo, "operacion": t.operacion,
                   "precio": t.precio, "ubicacion": t.ubicacion, "slug": t.slug,
                   "puestos": t.puestos, "residencia": t.residencia, "habitaciones": t.habitaciones})
        })
        .collect();
    Ok(
        json!({"inmuebles": items, "total": total, "tarjetas_enviadas": enviadas, "repetidas": repetidas}),
    )
}

/// Tope de tarjetas por turno (regla usuaria: con mas de 5 resultados se
/// muestran 5 + cierre "tengo N mas").
const MAX_TARJETAS: usize = 5;

/// Tarjeta breve de texto (una propiedad por mensaje, menos de 300 chars).
fn tarjeta_texto(t: &Tarjeta) -> String {
    let titulo: String = t.titulo.trim().chars().take(120).collect();
    let ubicacion: String = t.ubicacion.trim().chars().take(80).collect();
    format!(
        "🏠 {titulo}\n{} en {} · {}\n📍 {ubicacion}",
        t.tipo,
        t.operacion,
        formato_precio(t.precio)
    )
}

/// Precio en dolares con miles (`$150.000`): legible en una tarjeta breve.
/// Sin casts (el redondeo va por formato): `abs` + `{:.0}` + agrupar.
fn formato_precio(precio: f64) -> String {
    let entero: String = format!("{:.0}", precio.abs())
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    let entero = if entero.is_empty() {
        "0".to_string()
    } else {
        entero
    };
    let digitos: Vec<char> = entero.chars().collect();
    let mut grupos: Vec<String> = Vec::new();
    let mut resto = digitos.as_slice();
    while resto.len() > 3 {
        let (cabeza, cola) = resto.split_at(resto.len() - 3);
        grupos.push(cola.iter().collect());
        resto = cabeza;
    }
    grupos.push(resto.iter().collect());
    grupos.reverse();
    format!("${}", grupos.join("."))
}

/// Encola una tarjeta por propiedad (hasta `MAX_TARJETAS`) en outbox
/// `whatsapp`. Devuelve `(nuevas, repetidas)`: `0` nuevas = el modelo lista
/// a mano o ya estaban encoladas.
/// [309A-1] Dedup entre turnos: el modelo re-llama `buscar` en el turno
/// siguiente ("mándame las fotos") y antes re-encolaba la misma tarjeta
/// (el visitante la recibía 2 veces). Se compara el texto exacto contra los
/// últimos 30 mensajes `ai` del hilo (incluye los espejos 299A-4): lo ya
/// enviado se salta y se cuenta en `repetidas`. Best-effort: si la lectura
/// falla se envía como antes (fail-open, nunca se bloquea un envío por un
/// fallo de lectura) y se avisa con WARN.
async fn encolar_tarjetas(
    pool: &PgPool,
    session_id: Uuid,
    filas: &[Tarjeta],
    hub: Option<&ChatHub>,
) -> (usize, usize) {
    if filas.is_empty() {
        return (0, 0);
    }
    let telefono = match ClienteRepository::ficha_para_aviso(pool, session_id).await {
        Ok(f) => f
            .telefono
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty()),
        Err(e) => {
            tracing::warn!("tarjetas sesion={session_id}: sin ficha ({e}), no se encolan");
            return (0, 0);
        }
    };
    let Some(destino) = telefono else {
        return (0, 0);
    };
    let via = match ClienteRepository::canal_de(pool, session_id).await {
        Ok(Some(v)) if v == "wa_a" || v == "wa_b" => v,
        _ => return (0, 0),
    };
    let mut ya_enviadas = cuerpos_ai_recientes(pool, session_id).await;
    let mut enviadas = 0;
    let mut repetidas = 0;
    for t in filas.iter().take(MAX_TARJETAS) {
        let texto = tarjeta_texto(t);
        if ya_enviadas.contains(&texto) {
            repetidas += 1;
            continue;
        }
        /* [011A-5 Fase1] Clave idempotente bajo corte: un reintento con el
         * mismo texto no duplica (el gemelo en vuelo ya espejó en hilo).
         * Se calcula antes del `json!` porque este mueve `via`. */
        let clave_tarjeta;
        let clave_tarjeta_ref = if debe_usar_clave("tarjeta") && corte_cubre(pool, &via).await {
            clave_tarjeta = clave_idempotencia(&session_id.to_string(), "tarjeta", &texto);
            Some(clave_tarjeta.as_str())
        } else {
            None
        };
        let tarjeta = json!({
            "session_id": session_id.to_string(),
            "destino": destino,
            "texto": texto.clone(),
            "via": via,
            "motivo": "tarjeta",
        });
        match encolar_outbox_idem(pool, "whatsapp", tarjeta, clave_tarjeta_ref).await {
            Ok(Encolado::Nuevo(_)) => {
                enviadas += 1;
                /* Se registra en el set para que dos filas con el mismo
                 * texto en el mismo lote tampoco se dupliquen. */
                ya_enviadas.insert(texto.clone());
                espejar_en_hilo(pool, hub, session_id, &texto).await;
            }
            /* [011A-5 Fase3] Revivido = el gemelo estaba `failed` y vuelve
             * a `pending` con este payload: cuenta como enviada y entra al
             * set del lote, pero NO se espeja (el primer encolado ya dejó
             * el texto en el hilo). */
            Ok(Encolado::Revivido(_)) => {
                enviadas += 1;
                ya_enviadas.insert(texto.clone());
            }
            Ok(Encolado::Duplicado) => {
                repetidas += 1;
            }
            Err(e) => {
                tracing::warn!("tarjetas sesion={session_id}: no se pudo encolar ({e})");
                break;
            }
        }
    }
    (enviadas, repetidas)
}

/// Cuerpos de los últimos mensajes `ai` del hilo (ventana de 30): base del
/// dedup 309A-1 (tarjetas y fotos ya enviadas). Vacío + WARN si falla.
/* [011A-1] Foto F5-Paso0: la ventana de 30 queda en const con nombre para
 * que la sombra detecte si cambia (era literal suelto). */
const VENTANA_DEDUP_TARJETAS: i64 = 30;
async fn cuerpos_ai_recientes(pool: &PgPool, session_id: Uuid) -> HashSet<String> {
    match glory_agent::persistence::list_messages(pool, session_id, VENTANA_DEDUP_TARJETAS).await {
        Ok(msgs) => msgs
            .into_iter()
            .filter(|m| m.sender == "ai")
            .map(|m| m.body)
            .collect(),
        Err(e) => {
            tracing::warn!(
                "dedup sesion={session_id}: sin historial reciente ({e}), se envía todo"
            );
            HashSet::new()
        }
    }
}

/// [299A-4] Espejo de envíos `WhatsApp` en el hilo: `agent_outbox` es cola
/// transitoria (el worker BORRA la fila tras enviar) y `historial` solo lee
/// `agent_messages`, así que tarjeta/foto nunca aparecían en /admin aunque
/// el visitante sí las recibía. Tras cada enqueue al visitante se persiste
/// el mismo contenido como `ai` (la foto en marca `[foto] url — se ve: pie`,
/// que `MessageMedia` ya renderiza). Si el espejo falla se avisa y se sigue:
/// el envío ya está encolado y tumbar la tool duplicaría el envío al
/// reintentar el modelo. Sin hub no hay espejo (tests y widget web).
async fn espejar_en_hilo(pool: &PgPool, hub: Option<&ChatHub>, session_id: Uuid, body: &str) {
    let Some(hub) = hub else { return };
    if let Err(e) =
        glory_agent::persistence::insert_message_seq(pool, hub, session_id, "ai", body, None, None)
            .await
    {
        tracing::warn!("espejo sesion={session_id}: envío sin reflejar en el hilo ({e})");
    }
}

/// Tarjeta breve de un inmueble para `buscar_inmuebles` (struct en vez de
/// tupla de 9: legible y evita el lint de tipos complejos). Incluye
/// `puestos`, `residencia` y `habitaciones` para que el agente responda con
/// esos datos exactos (Fase3-H3: sin este campo filtraba a ojo).
#[derive(Debug, sqlx::FromRow)]
struct Tarjeta {
    id: Uuid,
    titulo: String,
    tipo: String,
    operacion: String,
    precio: f64,
    ubicacion: String,
    slug: String,
    puestos: i32,
    residencia: String,
    habitaciones: i32,
}

/// Ficha completa de un inmueble para `detalle_inmueble` (struct en vez de
/// tupla de 12: legible y evita el lint de tipos complejos). Tipos alineados
/// con `20260915000002_inmuebles.up.sql` (NOT NULL salvo `copy_corta`).
/* [279A-8] La IA ve cada campo rellenable: `extras` (respuestas /ask, tal cual,
 * incluidos `no_se`/`a_veces`: saber lo que falta también informa) y si el
 * precio tiene margen (`margen_negociable`, calculado en SQL). La cifra del
 * mínimo jamás sale (frontera 279A-3: la IA insinúa sin cifras). */
#[derive(Debug, sqlx::FromRow)]
struct Ficha {
    titulo: String,
    descripcion: String,
    ubicacion: String,
    puestos: i32,
    residencia: String,
    precio: f64,
    tipo: String,
    operacion: String,
    habitaciones: i32,
    banos: i32,
    metros: f64,
    metros_terreno: f64,
    estado: String,
    copy_corta: Option<String>,
    extras: Value,
    margen_negociable: bool,
}

async fn detalle(pool: &PgPool, args: &Value) -> Result<Value, AgentError> {
    let id: Uuid = args
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .parse()
        .map_err(|_| AgentError::BadRequest("id de inmueble invalido".to_string()))?;
    let fila: Option<Ficha> = sqlx::query_as(
        "SELECT titulo, descripcion, ubicacion, puestos, residencia, precio, tipo, operacion, \
              habitaciones, banos, metros, metros_terreno, estado, copy_corta, extras, \
              (precio_minimo IS NOT NULL AND precio_minimo > 0) AS margen_negociable \
              FROM inmuebles WHERE id = $1 AND publicado",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(f) = fila else {
        return Ok(json!({"error": "inmueble no disponible"}));
    };
    let descripcion: String = f.descripcion.chars().take(600).collect();
    Ok(
        json!({"id": id, "titulo": f.titulo, "descripcion": descripcion, "ubicacion": f.ubicacion,
              "puestos": f.puestos, "residencia": f.residencia,
              "precio": f.precio, "tipo": f.tipo, "operacion": f.operacion, "habitaciones": f.habitaciones,
              "banos": f.banos, "metros": f.metros, "metros_terreno": f.metros_terreno, "estado": f.estado,
              "resumen": f.copy_corta.unwrap_or_default(),
              "extras": f.extras, "margen_negociable": f.margen_negociable}),
    )
}

/* [299A-1 E13] La IA envía fotos del catálogo por WhatsApp: hasta 3
 * `image+caption` (pie = título) vía outbox `whatsapp` con `media_url`
 * absoluta (`/uploads/<storage_key>` bajo `PUBLIC_BASE_URL`). `destino` =
 * teléfono del visitante (ficha) y `via` = canal de la sesión, igual que
 * `consultar`. Sin fotos o sin teléfono responde `error` (la IA lo dice). */
async fn enviar_fotos(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
    hub: Option<&ChatHub>,
) -> Result<Value, AgentError> {
    let id: Uuid = args
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .parse()
        .map_err(|_| AgentError::BadRequest("id de inmueble invalido".to_string()))?;
    let max = args
        .get("max")
        .and_then(Value::as_i64)
        .unwrap_or(3)
        .clamp(1, 3);
    let titulo: Option<String> =
        sqlx::query_scalar("SELECT titulo FROM inmuebles WHERE id = $1 AND publicado")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(titulo) = titulo.filter(|t| !t.trim().is_empty()) else {
        return Ok(json!({"error": "inmueble no disponible"}));
    };
    let claves: Vec<String> = sqlx::query_scalar(
        "SELECT storage_key FROM fotos WHERE inmueble_id = $1 ORDER BY orden LIMIT $2",
    )
    .bind(id)
    .bind(max)
    .fetch_all(pool)
    .await
    .map_err(|e| AgentError::Db(e.to_string()))?;
    if claves.is_empty() {
        return Ok(json!({"error": "ese inmueble aún no tiene fotos"}));
    }
    let ficha = ClienteRepository::ficha_para_aviso(pool, session_id)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(telefono) = ficha
        .telefono
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
    else {
        return Ok(json!({"error": "sin teléfono del visitante"}));
    };
    let via = ClienteRepository::canal_de(pool, session_id)
        .await
        .ok()
        .flatten()
        .filter(|v| v == "wa_a" || v == "wa_b")
        .unwrap_or_else(|| "wa_a".to_string());
    /* Base pública para `media_url`: en local el gateway descarga de este
     * mismo backend; en producción `PUBLIC_BASE_URL` lleva el dominio
     * (los servidores de WhatsApp deben alcanzarla). */
    let base = std::env::var("PUBLIC_BASE_URL")
        .ok()
        .map(|b| b.trim().trim_end_matches('/').to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string());
    let total = claves.len();
    /* [309A-1] Dedup de fotos entre turnos (misma causa que las tarjetas:
     * re-llamar con el mismo id re-enviaba las 3 fotos). Se compara la URL
     * contra los espejos `[foto] url ...` ya presentes en el hilo. */
    let ya_enviadas = cuerpos_ai_recientes(pool, session_id).await;
    let mut enviadas = 0;
    let mut repetidas = 0;
    for (i, clave) in claves.iter().enumerate() {
        let pie = if total > 1 {
            format!("{} ({}/{})", titulo.trim(), i + 1, total)
        } else {
            titulo.trim().to_string()
        };
        /* [299A-4] La misma foto que viaja por WhatsApp queda en el hilo
         * (`[foto] url — se ve: pie` la renderiza `MessageMedia`). */
        let url = format!("{base}/uploads/{clave}");
        if ya_enviadas.iter().any(|b| b.contains(&url)) {
            repetidas += 1;
            continue;
        }
        let espejo = format!("[foto] {url} — se ve: {}", pie.trim());
        /* [011A-5 Fase1] Igual que tarjetas: clave bajo corte, duplicado
         * en vuelo cuenta como repetida (el gemelo ya espejó). Se calcula
         * antes del `json!` porque este mueve `pie` y `via`. */
        let clave_foto;
        let clave_foto_ref = if debe_usar_clave("ia_foto") && corte_cubre(pool, &via).await {
            clave_foto = clave_idempotencia(&session_id.to_string(), "ia_foto", &pie);
            Some(clave_foto.as_str())
        } else {
            None
        };
        let aviso = json!({
            "session_id": session_id.to_string(),
            "destino": telefono,
            "texto": pie,
            "media_url": url,
            "via": via,
            "motivo": "ia_foto",
        });
        match encolar_outbox_idem(pool, "whatsapp", aviso, clave_foto_ref).await {
            Ok(Encolado::Duplicado) => {
                repetidas += 1;
                continue;
            }
            /* [011A-5 Fase3] Revivido: el gemelo `failed` vuelve a
             * `pending`; no se espeja (el primer encolado ya dejó la
             * foto en el hilo). */
            Ok(Encolado::Revivido(_)) => {}
            Err(e) => return Err(e.into()),
            Ok(Encolado::Nuevo(_)) => {
                espejar_en_hilo(pool, hub, session_id, &espejo).await;
            }
        }
        enviadas += 1;
    }
    Ok(json!({"ok": true, "enviadas": enviadas, "repetidas": repetidas, "titulo": titulo}))
}

/// Teléfono 6..24 chars de `+0123456789 ()-.` con al menos 6 dígitos.
pub fn telefono_valido(tel: &str) -> bool {
    let t = tel.trim();
    (6..=24).contains(&t.len())
        && t.chars()
            .all(|c| c.is_ascii_digit() || "+ ()-.".contains(c))
        && t.chars().filter(char::is_ascii_digit).count() >= 6
}

/* [08AA-8] Captación/contacto/escalado a humano viven en
 * `chat_tools_captacion.rs` (split god-object); se re-exportan para el
 * dispatcher `Herramientas::ejecutar` y los tests (`super::*`). */
pub use super::chat_tools_captacion::contacto_publico;
/* Solo lo que usan el dispatcher y los tests (`super::*`); los helpers
 * internos (`arg_texto`, `aviso_humano`…) quedan en el módulo hijo. */
pub(super) use super::chat_tools_captacion::{agendar, captar, consultar, escalar, registrar};

/* [08AA-8] `destino_humano`→`agendar` movidos a `chat_tools_captacion.rs`
 * (re-export arriba para dispatcher y tests). */

/* [169A-4] Las consultas SQL no usan macros verificadas en compilación:
 * estos tests las ejecutan contra la BD real de rama (`DATABASE_URL`).
 * Sin `DATABASE_URL` se omiten (gate local sin BD sigue verde). */
#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::repositories::VisitaRepository;
    use glory_agent::tools::{ToolCtx, ToolExecutor};

    fn pool_si_hay() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    #[test]
    fn telefono_acepta_formatos_reales() {
        assert!(telefono_valido("+34 600 123 456"));
        assert!(telefono_valido("600123456"));
        assert!(telefono_valido("+1 (555) 123-4567"));
        assert!(!telefono_valido("abc"));
        assert!(!telefono_valido("12345"));
        assert!(!telefono_valido(""));
    }

    /* [011A-1] Foto F5-Paso0: el dedup de tarjetas mira los últimos 30
     * mensajes `ai` del hilo. */
    #[test]
    fn foto_dedup_mira_ultimos_30() {
        assert_eq!(VENTANA_DEDUP_TARJETAS, 30);
    }

    /* [E-fluido F2] Tarjeta breve: una propiedad por mensaje, menos de 300
     * chars, precio con miles. */
    #[test]
    fn tarjeta_breve_con_precio_legible() {
        assert_eq!(formato_precio(150_000.0), "$150.000");
        assert_eq!(formato_precio(2_500.5), "$2.500");
        assert_eq!(formato_precio(900.0), "$900");
        let t = Tarjeta {
            id: Uuid::new_v4(),
            titulo: "Apartamento Residencias Caroní Plaza".to_string(),
            tipo: "apartamento".to_string(),
            operacion: "venta".to_string(),
            precio: 85_000.0,
            ubicacion: "Puerto Ordaz".to_string(),
            slug: "x".to_string(),
            puestos: 3,
            residencia: "2".to_string(),
            habitaciones: 2,
        };
        let texto = tarjeta_texto(&t);
        assert!(texto.contains("Apartamento Residencias Caroní Plaza"));
        assert!(texto.contains("apartamento en venta"));
        assert!(texto.contains("$85.000"));
        assert!(texto.chars().count() < 300);
        let larga = Tarjeta {
            titulo: "x".repeat(500),
            ubicacion: "y".repeat(500),
            ..Tarjeta {
                id: Uuid::new_v4(),
                titulo: String::new(),
                tipo: "casa".to_string(),
                operacion: "alquiler".to_string(),
                precio: 1_200_000.0,
                ubicacion: String::new(),
                slug: String::new(),
                puestos: 1,
                residencia: String::new(),
                habitaciones: 1,
            }
        };
        assert!(tarjeta_texto(&larga).chars().count() < 300);
    }

    /* [E-fluido F2] Sin teléfono del visitante no hay a donde enviar tarjetas:
     * `tarjetas_enviadas` 0, nada en outbox y el modelo lista a mano. */
    #[tokio::test]
    async fn buscar_sin_telefono_no_encola_tarjetas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(
            salida.get("tarjetas_enviadas").and_then(Value::as_u64),
            Some(0)
        );
        let encoladas: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(encoladas, 0);
        limpiar_sesion(&pool, sesion).await;
    }

    /* [Fase3-H3] El filtro de habitaciones es exacto en BD: pedir 2 nunca
     * trae 3 (antes el modelo filtraba a ojo y colaba de más). */
    #[tokio::test]
    async fn buscar_filtra_habitaciones_exactas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute("buscar_inmuebles", &json!({"habitaciones": 2}), &ctx)
            .await
            .unwrap();
        let items = salida
            .get("inmuebles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!items.is_empty());
        assert!(items
            .iter()
            .all(|i| i.get("habitaciones").and_then(Value::as_i64) == Some(2)));
        limpiar_sesion(&pool, sesion).await;
    }

    /* [Fase3-v2] Sin tildes también encuentra: "caroni" localiza el local
     * de "Riberas del Caroní" (antes 0 filas y la IA negaba la oferta). */
    #[tokio::test]
    async fn buscar_sin_tilde_encuentra_igual() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute("buscar_inmuebles", &json!({"zona": "caroni"}), &ctx)
            .await
            .unwrap();
        let items = salida
            .get("inmuebles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| i
            .get("ubicacion")
            .and_then(Value::as_str)
            .is_some_and(|u| u.contains("Caron"))));
        limpiar_sesion(&pool, sesion).await;
    }

    /* [Fase3-H1] Un número dictado no re-clavea el hilo: la sesión sigue
     * atada al remitente real y el número nuevo queda como ficha en
     * `clientes` (antes partía el hilo y la segunda vuelta perdía el
     * historial). */
    #[tokio::test]
    async fn registrar_no_reclavea_hilo() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let dueno = ClienteRepository::registrar(&pool, Some("Dueno Hilo"), "34111111111")
            .await
            .unwrap();
        ClienteRepository::vincular_canal(
            &pool,
            sesion,
            dueno.id,
            "34111111111",
            "wa_b",
            "inicial",
        )
        .await
        .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute(
                "registrar_contacto",
                &json!({"nombre": "Otro Numero", "telefono": "34222222222"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(salida.get("ok").and_then(Value::as_bool), Some(true));
        let canal: (String, Uuid) =
            sqlx::query_as("SELECT telefono, cliente_id FROM canal_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(canal.0, "34111111111");
        assert_eq!(canal.1, dueno.id);
        let ficha: (String,) =
            sqlx::query_as("SELECT nombre FROM clientes WHERE telefono = '34222222222'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(ficha.0, "Otro Numero");
        sqlx::query("DELETE FROM clientes WHERE telefono IN ('34111111111','34222222222')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        limpiar_sesion(&pool, sesion).await;
    }

    async fn limpiar_sesion(pool: &PgPool, sesion: Uuid) {
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn tools_consultan_esquema_real() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let lista = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert!(lista.get("inmuebles").and_then(Value::as_array).is_some());
        let mala = h
            .execute("buscar_inmuebles", &json!({"tipo": "castillo"}), &ctx)
            .await
            .unwrap();
        assert!(mala.get("error").is_some());
        let ausente = h
            .execute(
                "detalle_inmueble",
                &json!({"id": Uuid::new_v4().to_string()}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(ausente.get("error").is_some());

        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn contacto_y_escalado_persisten() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let r = h
            .execute(
                "registrar_contacto",
                &json!({"nombre": "Humo Test", "telefono": "+34611111111"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        /* [279A-2 F1] La tool también deja la fila en `clientes`. */
        let cliente: String =
            sqlx::query_scalar("SELECT telefono FROM clientes WHERE telefono = '34611111111'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cliente, "34611111111");
        let datos = contacto_publico(&pool, "Test 600111222").await.unwrap();
        assert!(datos.get("whatsapp_url").and_then(Value::as_str).is_some());
        let esc = h
            .execute("escalar_a_humano", &json!({"motivo": "prueba humo"}), &ctx)
            .await
            .unwrap();
        assert_eq!(esc.get("ok").and_then(Value::as_bool), Some(true));
        let estado = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap()
            .status;
        assert_eq!(estado, "escalated");
        let pendientes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(pendientes, 1);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34611111111'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [279A-2 F3] Matriz mínima: consultar congela (`consultando` +
     * `ai_enabled=false` + ciclo `waiting` + aviso con resumen) y escalar
     * delega (`delegada` + triple freno). */
    #[tokio::test]
    async fn consultar_congela_y_escalar_delega() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let c = h
            .execute(
                "consultar_agente",
                &json!({"motivo": "duda precio", "resumen": "pregunta margen"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(c.get("ok").and_then(Value::as_bool), Some(true));
        let fila = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap();
        assert!(!fila.ai_enabled);
        let estado_at: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado_at, "consultando");
        let ciclo: String =
            sqlx::query_scalar("SELECT status FROM agent_response_cycles WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(ciclo, "waiting");

        let e = h
            .execute(
                "escalar_a_humano",
                &json!({"motivo": "pide humano", "resumen": "quiere visita"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(e.get("ok").and_then(Value::as_bool), Some(true));
        let fila2 = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fila2.status, "escalated");
        assert!(!fila2.ai_enabled);
        let estado_at2: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado_at2, "delegada");

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [299A-1 E13] Enviar fotos encola un outbox `whatsapp` con `media_url`
     * por foto (tope `max`) y rechaza id inválido/ausente sin tocar BD. */
    #[tokio::test]
    async fn enviar_fotos_encola_media_url() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let mala = h
            .execute("enviar_fotos_inmueble", &json!({"id": "no-uuid"}), &ctx)
            .await;
        assert!(mala.is_err());
        let ausente = h
            .execute(
                "enviar_fotos_inmueble",
                &json!({"id": Uuid::new_v4().to_string()}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(ausente.get("error").is_some());

        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Foto Test", "telefono": "+34622222222"}),
            &ctx,
        )
        .await
        .unwrap();
        /* Inmueble semilla con fotos (solo lectura; no se toca). */
        let id: String = sqlx::query_scalar(
            "SELECT id::TEXT FROM inmuebles WHERE publicado \
             AND (SELECT COUNT(*) FROM fotos WHERE inmueble_id = inmuebles.id) > 0 LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let r = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(r.get("enviadas").and_then(Value::as_i64), Some(2));
        let fotos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1 \
             AND payload->>'motivo' = 'ia_foto' AND payload ? 'media_url'",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(fotos, 2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34622222222'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [299A-4] Con hub, cada `ia_foto` persiste su espejo `ai [foto] ...`
     * en el hilo (sin hub no hay espejo: el resto de tests lo confirma). */
    #[tokio::test]
    async fn enviar_fotos_espeja_hilo_con_hub() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string())
            .with_hub(ChatHub::default());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Espejo Test", "telefono": "+34633333333"}),
            &ctx,
        )
        .await
        .unwrap();
        let id: String = sqlx::query_scalar(
            "SELECT id::TEXT FROM inmuebles WHERE publicado \
             AND (SELECT COUNT(*) FROM fotos WHERE inmueble_id = inmuebles.id) > 0 LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let r = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(r.get("enviadas").and_then(Value::as_i64), Some(2));
        let espejos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_messages WHERE session_id = $1 \
             AND sender = 'ai' AND body LIKE '[foto] %'",
        )
        .bind(sesion)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(espejos, 2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34633333333'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [309A-1] Segunda vuelta de `buscar` con el mismo filtro no re-encola
     * tarjetas ya enviadas (antes el visitante recibía la tarjeta 2 veces:
     * turno 1 `buscar` + turno 2 `buscar` de nuevo al pedir fotos). */
    #[tokio::test]
    async fn buscar_no_repite_tarjetas_ya_enviadas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string())
            .with_hub(ChatHub::default());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Dedup Test", "telefono": "+34644444444"}),
            &ctx,
        )
        .await
        .unwrap();
        /* `registrar` no vincula canal (Fase3-H1): se fija `wa_b` directo
         * para que `encolar_tarjetas` tenga vía de salida. */
        sqlx::query(
            "INSERT INTO canal_sesiones (session_id, cliente_id, canal, telefono, modo) \
             VALUES ($1, NULL, 'wa_b', '34644444444', 'completo')",
        )
        .bind(sesion)
        .execute(&pool)
        .await
        .unwrap();

        let primera = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert!(
            primera
                .get("tarjetas_enviadas")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        );
        let outbox_1: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        let segunda = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(
            segunda.get("tarjetas_enviadas").and_then(Value::as_u64),
            Some(0)
        );
        assert!(
            segunda
                .get("repetidas")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        );
        let outbox_2: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(outbox_1, outbox_2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34644444444'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [309A-1] Re-llamar `enviar_fotos` con el mismo id no reenvía las fotos
     * ya presentes en el hilo (mismo dedup que las tarjetas). */
    #[tokio::test]
    async fn enviar_fotos_no_repite_fotos_ya_enviadas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string())
            .with_hub(ChatHub::default());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Dedup Foto", "telefono": "+34655555555"}),
            &ctx,
        )
        .await
        .unwrap();
        let id: String = sqlx::query_scalar(
            "SELECT id::TEXT FROM inmuebles WHERE publicado \
             AND (SELECT COUNT(*) FROM fotos WHERE inmueble_id = inmuebles.id) > 0 LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let primera = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(primera.get("enviadas").and_then(Value::as_i64), Some(2));
        let segunda = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(segunda.get("enviadas").and_then(Value::as_i64), Some(0));
        assert_eq!(segunda.get("repetidas").and_then(Value::as_i64), Some(2));
        let espejos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_messages WHERE session_id = $1 \
             AND sender = 'ai' AND body LIKE '[foto] %'",
        )
        .bind(sesion)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(espejos, 2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34655555555'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [E14] `registrar_captacion`: valida args, crea la solicitud con el
     * servicio web (origen `whatsapp`, `pendiente`), marca `captacion`
     * y encola el aviso al captador. */
    #[tokio::test]
    async fn captacion_registra_y_avisa() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let mala = h
            .execute(
                "registrar_captacion",
                &json!({"nombre": "Vende Casas", "telefono": "abc"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(mala.get("error").is_some());

        let r = h
            .execute(
                "registrar_captacion",
                &json!({
                    "nombre": "Vende Casas",
                    "telefono": "+34633333333",
                    "operacion": "venta",
                    "ubicacion": "Chacao",
                    "descripcion": "Apartamento 80m2, 2 hab",
                    "precio_estimado": 95000.0
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        let sid = r
            .get("solicitud_id")
            .and_then(Value::as_str)
            .expect("solicitud_id");
        let fila: (String, String) =
            sqlx::query_as("SELECT estado, origen_contacto FROM solicitudes WHERE id = $1::UUID")
                .bind(sid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(fila, ("pendiente".to_string(), "whatsapp".to_string()));
        let estado: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado, "captacion");
        let avisos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1 \
             AND payload->>'motivo' = 'captacion'",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(avisos, 1);

        sqlx::query("DELETE FROM solicitudes WHERE id = $1::UUID")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [E15] Matriz mínima de visitas: rechazo de args malos sin tocar
     * la BD; agenda feliz (fila `pendiente` + congelar como `consultar`
     * + aviso `visita`) y confirmación del admin con fecha. */
    /* [E15] Limpieza de los rastros de sesión que dejan estos tests
     * (outbox, mensajes, canal, atención, ciclo y sesión sintética). */
    async fn limpiar_rastros(pool: &PgPool, sesion: Uuid) {
        for (sql, texto) in [
            (
                "DELETE FROM agent_outbox WHERE payload->>'session_id' = $1",
                true,
            ),
            ("DELETE FROM agent_messages WHERE session_id = $1", false),
            ("DELETE FROM canal_sesiones WHERE session_id = $1", false),
            ("DELETE FROM atencion_sesiones WHERE session_id = $1", false),
            (
                "DELETE FROM agent_response_cycles WHERE session_id = $1",
                false,
            ),
            ("DELETE FROM agent_sessions WHERE id = $1", false),
        ] {
            if texto {
                sqlx::query(sql)
                    .bind(sesion.to_string())
                    .execute(pool)
                    .await
                    .unwrap();
            } else {
                sqlx::query(sql).bind(sesion).execute(pool).await.unwrap();
            }
        }
    }

    /* [E15] `agendar_visita` rechaza args malos sin tocar la BD (salvo el
     * uuid inexistente, que solo lee): la IA se corrige en el turno. */
    #[tokio::test]
    async fn visita_rechaza_args_malos() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let id = Uuid::new_v4().to_string();

        let mala = h
            .execute("agendar_visita", &json!({"id": "no-uuid"}), &ctx)
            .await;
        assert!(mala.is_err());
        for args in [
            json!({"id": id, "telefono": "+34633333333", "cuando": "hoy"}),
            json!({"id": id, "nombre": "T", "telefono": "abc", "cuando": "hoy"}),
            json!({"id": id, "nombre": "T", "telefono": "+34633333333"}),
            json!({"id": id, "nombre": "T", "telefono": "+34633333333", "cuando": "hoy", "fecha": "ayer"}),
        ] {
            let r = h.execute("agendar_visita", &args, &ctx).await.unwrap();
            assert!(r.get("error").is_some(), "args: {args}");
        }
        let ausente = h
            .execute(
                "agendar_visita",
                &json!({
                    "id": id,
                    "nombre": "Nadie",
                    "telefono": "+34633333333",
                    "cuando": "mañana"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(ausente.get("error").is_some());

        limpiar_rastros(&pool, sesion).await;
    }

    /* [E15] Matriz mínima feliz: abre la fila en `pendiente`, congela
     * como `consultar` y avisa con motivo `visita`; el admin confirma
     * con fecha desde el repo. */
    #[tokio::test]
    async fn visita_agenda_y_avisa() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        /* Inmueble semilla publicado (solo lectura; no se toca). */
        let Some(id): Option<String> =
            sqlx::query_scalar("SELECT id::TEXT FROM inmuebles WHERE publicado LIMIT 1")
                .fetch_optional(&pool)
                .await
                .unwrap()
        else {
            limpiar_rastros(&pool, sesion).await;
            return;
        };

        let r = h
            .execute(
                "agendar_visita",
                &json!({
                    "id": id,
                    "nombre": "Visita Test",
                    "telefono": "+34633333333",
                    "cuando": "el sábado en la mañana"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        let vid = r
            .get("visita_id")
            .and_then(Value::as_str)
            .expect("visita_id");
        let fila: (String, String, Option<chrono::NaiveDate>) =
            sqlx::query_as("SELECT estado, cuando, fecha FROM visitas WHERE id = $1::UUID")
                .bind(vid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(fila.0, "pendiente".to_string());
        assert_eq!(fila.1, "el sábado en la mañana".to_string());
        assert_eq!(fila.2, None);
        let estado: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado, "consultando");
        let fila_sesion = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap();
        assert!(!fila_sesion.ai_enabled);
        let avisos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1 \
             AND payload->>'motivo' = 'visita'",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(avisos, 1);
        /* El admin confirma con fecha: cierra el ciclo de la cita. */
        let conf = VisitaRepository::cambiar_estado(
            &pool,
            vid.parse().unwrap(),
            "confirmada",
            Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap()),
        )
        .await
        .unwrap()
        .expect("visita confirmada");
        assert_eq!(conf.estado, "confirmada");
        assert_eq!(
            conf.fecha,
            Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap())
        );

        sqlx::query("DELETE FROM visitas WHERE id = $1::UUID")
            .bind(vid)
            .execute(&pool)
            .await
            .unwrap();
        limpiar_rastros(&pool, sesion).await;
    }
}
