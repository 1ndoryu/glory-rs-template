use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{CreateSolicitudRequest, OPERACIONES, TIPOS};
use crate::repositories::{ClienteRepository, NuevaVisita, VisitaRepository};
use crate::services::SolicitudService;
use crate::services::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar_outbox_idem, Encolado,
};
use glory_agent::errors::AgentError;
use glory_agent::session::ChatHub;
use glory_agent::tools::{ToolCtx, ToolDefinition, ToolExecutor};

/* [169A-4] Tools de la inmobiliaria (las ejecuta el loop F6 del núcleo).
 * Salidas compactas: cada token cuenta para la ventana de 30k. La IA nunca
 * inventa datos: precios/direcciones salen de `buscar/detalle`, el teléfono
 * de `datos_contacto` y el humano de `escalar_a_humano`. */

/// Definiciones para el provider (schemas cortos, en español).
/// Una fn por tool: el `vec!` monolítico superó el tope de líneas.
pub fn definiciones() -> Vec<ToolDefinition> {
    vec![
        def_buscar(),
        def_detalle(),
        def_registrar_contacto(),
        def_enviar_fotos(),
        def_datos_contacto(),
        def_escalar(),
        def_consultar(),
        def_captacion(),
        def_visita(),
    ]
}

fn def_buscar() -> ToolDefinition {
    ToolDefinition::new(
        "buscar_inmuebles",
        "Busca inmuebles publicados y disponibles. Usala siempre antes de hablar de oferta concreta. \
         Pasa `habitaciones` (numero exacto) y `zona` siempre que el visitante los mencione: \
         son filtros exactos en BD, nunca filtres a ojo lo devuelto. \
         Las primeras tarjetas YA se envian solas como mensajes separados (mira `tarjetas_enviadas`): \
         no las repitas ni las listes en tu respuesta, solo intro de una linea + cierre breve. \
         Si llamas de nuevo con el mismo filtro, las tarjetas ya enviadas NO se reenvian \
         (mira `repetidas`): no las anuncies otra vez. \
         Si `total` es 0, dilo claro y pide otro filtro (no inventes oferta).",
        json!({
            "type": "object",
            "properties": {
                "texto": {"type": "string", "description": "Palabra en titulo o ubicacion"},
                "tipo": {"type": "string", "enum": ["apartamento", "casa", "local", "terreno", "townhouse"]},
                "operacion": {"type": "string", "enum": ["venta", "alquiler"]},
                "precio_max": {"type": "number"},
                "habitaciones": {"type": "integer", "description": "Numero exacto de habitaciones"},
                "zona": {"type": "string", "description": "Zona o sector (filtra por ubicacion)"},
                "limite": {"type": "integer", "default": 5}
            }
        }),
    )
}

fn def_detalle() -> ToolDefinition {
    ToolDefinition::new(
        "detalle_inmueble",
        "Ficha completa de un inmueble por su id (sale de buscar_inmuebles). \
         Trae `extras` con lo respondido en /ask (internet, agua, amoblado...) \
         y `margen_negociable` (bool, sin cifras: el minimo nunca se dice).",
        json!({
            "type": "object",
            "properties": {"id": {"type": "string", "format": "uuid"}},
            "required": ["id"]
        }),
    )
}

fn def_registrar_contacto() -> ToolDefinition {
    ToolDefinition::new(
        "registrar_contacto",
        "Guarda nombre y telefono del visitante cuando los da (ficha comercial; \
         no cambia el hilo actual). Solo di que quedo registrado si ESTA llamada \
         respondio exito en este turno: prohibido afirmarlo sin haberla llamado.",
        json!({
            "type": "object",
            "properties": {
                "nombre": {"type": "string"},
                "telefono": {"type": "string"}
            },
            "required": ["nombre", "telefono"]
        }),
    )
}

fn def_enviar_fotos() -> ToolDefinition {
    ToolDefinition::new(
        "enviar_fotos_inmueble",
        "Envia hasta 3 fotos del catalogo al visitante por WhatsApp (con el titulo como pie). \
         Usala cuando el visitante pida fotos de un inmueble o cuando ofrezcas enviarselas y acepte. \
         El id sale de buscar_inmuebles/detalle_inmueble. Tras llamarla, confirma en tu respuesta \
         que ya se las enviaste; no pegues URLs de fotos en el texto. \
         Las fotos ya enviadas en este hilo NO se reenvian (mira `repetidas`): \
         no las anuncies otra vez.",
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "string", "format": "uuid"},
                "max": {"type": "integer", "minimum": 1, "maximum": 3, "default": 3}
            },
            "required": ["id"]
        }),
    )
}

fn def_datos_contacto() -> ToolDefinition {
    ToolDefinition::new(
        "datos_contacto",
        "Telefono y WhatsApp oficiales de la inmobiliaria. Llamala antes de dar un numero.",
        json!({"type": "object", "properties": {}}),
    )
}

/* Esquema compartido `motivo+resumen` de `escalar_a_humano`/`consultar_agente`. */
fn esquema_motivo() -> Value {
    json!({
        "type": "object",
        "properties": {
            "motivo": {"type": "string"},
            "resumen": {"type": "string", "description": "Resumen breve para la ficha del aviso (1..500)"}
        },
        "required": ["motivo"]
    })
}

fn def_escalar() -> ToolDefinition {
    ToolDefinition::new(
        "escalar_a_humano",
        "Deriva la conversacion a un humano: avisa por WhatsApp al admin y frena a la IA. Usala si el visitante pide un humano o das 2 respuestas sin resolver.",
        esquema_motivo(),
    )
}

fn def_consultar() -> ToolDefinition {
    ToolDefinition::new(
        "consultar_agente",
        "Duda puntual: pregunta a un humano sin delegar del todo. Congela la IA (retoma al responder) y avisa por WhatsApp con la ficha.",
        esquema_motivo(),
    )
}

fn def_captacion() -> ToolDefinition {
    ToolDefinition::new(
        "registrar_captacion",
        "El visitante quiere VENDER o ALQUILAR su propiedad: registra la captacion (queda `pendiente` para el captador) y le avisa por WhatsApp. Pide antes nombre, telefono, operacion, ubicacion y detalles; llama solo con esos datos.",
        json!({
            "type": "object",
            "properties": {
                "nombre": {"type": "string"},
                "telefono": {"type": "string"},
                "operacion": {"type": "string", "enum": ["venta", "alquiler"]},
                "ubicacion": {"type": "string"},
                "descripcion": {"type": "string", "description": "Detalles de la propiedad (1..2000)"},
                "puestos": {"type": "integer"},
                "residencia": {"type": "string"},
                "precio_estimado": {"type": "number"},
                "email": {"type": "string"}
            },
            "required": ["nombre", "telefono", "ubicacion", "descripcion"]
        }),
    )
}

fn def_visita() -> ToolDefinition {
    ToolDefinition::new(
        "agendar_visita",
        "El visitante quiere VISITAR un inmueble del catalogo: agenda la visita (queda `pendiente`: el agente confirma dia y hora) y congela la IA hasta que el humano confirme. Pide antes nombre, telefono y cuando quiere ir (texto libre, ej. 'el sabado en la manana'); el id sale de buscar_inmuebles/detalle_inmueble; llama solo con esos datos.",
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "string", "format": "uuid"},
                "nombre": {"type": "string"},
                "telefono": {"type": "string"},
                "cuando": {"type": "string", "description": "Lo que dijo el visitante sobre cuando ir (1..200)"},
                "fecha": {"type": "string", "description": "Dia exacto YYYY-MM-DD, solo si lo dio el visitante"}
            },
            "required": ["id", "nombre", "telefono", "cuando"]
        }),
    )
}

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
/// a mano o todo ya se había enviado.
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
/// con `20260915000002_inmuebles.up.sql` (todo NOT NULL salvo `copy_corta`).
/* [279A-8] La IA ve todo lo rellenable: `extras` (respuestas /ask, tal cual,
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

async fn registrar(pool: &PgPool, session_id: Uuid, args: &Value) -> Result<Value, AgentError> {
    let nombre = args
        .get("nombre")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let telefono = args
        .get("telefono")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if nombre.is_empty() || nombre.len() > 80 {
        return Ok(json!({"error": "nombre requerido (1..80)"}));
    }
    if !telefono_valido(telefono) {
        return Ok(json!({"error": "telefono invalido"}));
    }
    glory_agent::persistence::set_session_contact(pool, session_id, Some(nombre), Some(telefono))
        .await?;
    /* [279A-2 F1] El contacto también vive en `clientes` (una fila por
     * teléfono, upsert idempotente): sin este paso la IA captaría datos
     * que nadie puede consultar. Si falla se propaga (nada silencioso).
     * [Fase3-H1] Ficha SIN re-vincular: el hilo pertenece al remitente real
     * y un número dictado no lo re-clavea (antes partía el hilo: segunda
     * vuelta sin historial). */
    ClienteRepository::registrar_sin_vincular(pool, Some(nombre), telefono)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    Ok(json!({"ok": true}))
}

/// Teléfono oficial: config `contacto_telefono` o `AGENTE_CONTACTO`.
/// `whatsapp_url` listo para el widget (`wa.me`, solo dígitos).
/// Pública para reutilizar en `GET /api/agent/info` (misma fuente que la tool).
pub async fn contacto_publico(pool: &PgPool, defecto: &str) -> Result<Value, AgentError> {
    let telefono = glory_agent::persistence::get_config(pool, "contacto_telefono")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| defecto.to_string());
    let admin = glory_agent::persistence::get_config(pool, "whatsapp_admin")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let digitos: String = admin.chars().filter(char::is_ascii_digit).collect();
    let url = if digitos.len() >= 6 {
        format!("https://wa.me/{digitos}")
    } else {
        String::new()
    };
    Ok(json!({"telefono": telefono, "whatsapp": admin, "whatsapp_url": url}))
}

/* [279A-2 F3] Delegación con congelamiento (máquina `atencion_sesiones`):
 * `consultar` = duda puntual (consultando + `ai_enabled=false` + ciclo
 * `waiting`; el humano devuelve con nota y la IA retoma); `escalar` =
 * delegación total (delegada + triple freno `escalated`+ciclo+`ai_enabled`;
 * la IA no vuelve sola). Ambas encolan `whatsapp` con ficha (el worker la
 * arma vía `ficha_para_aviso`) y `destino` explícito cuando hay
 * `whatsapp_admin` (si no, el worker usa el fallback y queda `pending`). */

async fn destino_humano(pool: &PgPool) -> Option<String> {
    glory_agent::persistence::get_config(pool, "whatsapp_admin")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn resumen_breve(args: &Value) -> String {
    args.get("resumen")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| "-".to_string(), |s| s.chars().take(500).collect())
}

/* [E14] Aviso al humano compartido por `consultar`/`escalar`/`captar`:
 * `via` = canal de la sesión (el gateway responde por el mismo número
 * que escribió el cliente; sin vínculo cae a `wa_a`) y `destino`
 * explícito cuando hay `whatsapp_admin` (si no, el worker usa el
 * fallback y queda `pending`: aviso parcial antes que ninguno). */
async fn aviso_humano(
    pool: &PgPool,
    session_id: Uuid,
    motivo: &str,
    resumen: &str,
) -> Result<(), AgentError> {
    let mut aviso = json!({
        "session_id": session_id.to_string(),
        "motivo": motivo,
        "resumen": resumen,
    });
    if let Ok(canal) = ClienteRepository::canal_de(pool, session_id).await {
        aviso["via"] = json!(canal.as_deref().unwrap_or("wa_a"));
    }
    if let Some(destino) = destino_humano(pool).await {
        aviso["destino"] = json!(destino);
    }
    /* [011A-5 Fase1] Clave bajo corte (motivos consultar/escalar/captar…);
     * `manual` queda excluido por `debe_usar_clave`. */
    let canal_aviso = aviso.get("via").and_then(|v| v.as_str()).unwrap_or("wa_a");
    let clave_aviso;
    let clave_aviso_ref = if debe_usar_clave(motivo) && corte_cubre(pool, canal_aviso).await {
        clave_aviso = clave_idempotencia(&session_id.to_string(), motivo, resumen);
        Some(clave_aviso.as_str())
    } else {
        None
    };
    /* [011A-5 Fase3] `Revivido` también envía (gemelo `failed` que vuelve
     * a `pending`): solo `Duplicado` se reporta como tragado. */
    if matches!(
        encolar_outbox_idem(pool, "whatsapp", aviso, clave_aviso_ref).await?,
        Encolado::Duplicado
    ) {
        tracing::info!(
            "aviso humano sesion={session_id} motivo={motivo}: duplicado tragado por idempotency_key"
        );
    }
    Ok(())
}

async fn consultar(pool: &PgPool, session_id: Uuid, args: &Value) -> Result<Value, AgentError> {
    let motivo = args
        .get("motivo")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if motivo.is_empty() || motivo.len() > 300 {
        return Ok(json!({"error": "motivo requerido (1..300)"}));
    }
    let resumen = resumen_breve(args);
    ClienteRepository::marcar_atencion(pool, session_id, "consultando")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    glory_agent::persistence::set_session_ai(pool, session_id, false).await?;
    glory_agent::persistence::upsert_response_cycle(pool, session_id, "waiting").await?;
    aviso_humano(pool, session_id, motivo, &resumen).await?;
    Ok(json!({"ok": true}))
}

async fn escalar(pool: &PgPool, session_id: Uuid, args: &Value) -> Result<Value, AgentError> {
    let motivo = args
        .get("motivo")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if motivo.is_empty() || motivo.len() > 300 {
        return Ok(json!({"error": "motivo requerido (1..300)"}));
    }
    let resumen = resumen_breve(args);
    glory_agent::persistence::set_session_status(pool, session_id, "escalated").await?;
    glory_agent::persistence::upsert_response_cycle(pool, session_id, "escalated").await?;
    glory_agent::persistence::set_session_ai(pool, session_id, false).await?;
    ClienteRepository::marcar_atencion(pool, session_id, "delegada")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    aviso_humano(pool, session_id, motivo, &resumen).await?;
    let telefono = glory_agent::persistence::get_config(pool, "contacto_telefono")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty());
    Ok(json!({"ok": true, "telefono": telefono.unwrap_or_default()}))
}

/* [E14] Captación: el visitante quiere vender/alquilar SU propiedad.
 * Crea la `solicitud` (origen `whatsapp`, siempre `pendiente`: la crea
 * el visitante, nunca la IA) con el mismo servicio del formulario web,
 * marca `captacion` (la IA confirma y sigue disponible; el captador
 * llama por teléfono fuera del chat) y avisa al humano con la ficha. */
fn arg_texto(args: &Value, clave: &str) -> String {
    args.get(clave)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn mapear_error_solicitud(e: AppError) -> AgentError {
    match e {
        AppError::Validation(m) | AppError::BadRequest(m) => AgentError::BadRequest(m),
        AppError::NotFound(m) => AgentError::Internal(format!("solicitud no encontrada: {m}")),
        AppError::Conflict(m) | AppError::Forbidden(m) => {
            AgentError::Internal(format!("rechazada: {m}"))
        }
        AppError::Unauthorized => AgentError::Internal("no autorizado".to_string()),
        AppError::PayloadMuyGrande => AgentError::Internal("carga demasiado grande".to_string()),
        AppError::Internal(m) => AgentError::Internal(m),
        AppError::Database(e) => AgentError::Db(e.to_string()),
        AppError::Io(e) => AgentError::Internal(e.to_string()),
    }
}

/* Valida los args de `registrar_captacion` y arma el pedido para el
 * servicio web. `Err` = `{"error": ...}` para que la IA se corrija. */
fn pedido_captacion(args: &Value) -> Result<(CreateSolicitudRequest, String, String), Value> {
    let nombre = arg_texto(args, "nombre");
    let telefono = arg_texto(args, "telefono");
    let ubicacion = arg_texto(args, "ubicacion");
    let descripcion = arg_texto(args, "descripcion");
    if nombre.is_empty() || nombre.len() > 200 {
        return Err(json!({"error": "nombre requerido (1..200)"}));
    }
    if !telefono_valido(&telefono) {
        return Err(json!({"error": "telefono invalido"}));
    }
    if ubicacion.is_empty() || ubicacion.len() > 300 {
        return Err(json!({"error": "ubicacion requerida (1..300)"}));
    }
    if descripcion.is_empty() || descripcion.len() > 2000 {
        return Err(json!({"error": "descripcion requerida (1..2000)"}));
    }
    let operacion = arg_texto(args, "operacion");
    let operacion = if operacion.is_empty() {
        "venta".to_string()
    } else {
        operacion
    };
    if !OPERACIONES.contains(&operacion.as_str()) {
        return Err(json!({"error": "operacion invalida (venta|alquiler)"}));
    }
    let req = CreateSolicitudRequest {
        nombre: nombre.clone(),
        telefono,
        email: {
            let e = arg_texto(args, "email");
            if e.is_empty() {
                None
            } else {
                Some(e)
            }
        },
        descripcion,
        ubicacion,
        puestos: args
            .get("puestos")
            .and_then(Value::as_i64)
            .and_then(|p| i32::try_from(p).ok())
            .unwrap_or(0)
            .max(0),
        residencia: arg_texto(args, "residencia"),
        precio_estimado: args.get("precio_estimado").and_then(Value::as_f64),
        operacion: operacion.clone(),
        fotos: vec![],
        origen_contacto: Some("whatsapp".to_string()),
    };
    Ok((req, operacion, nombre))
}

async fn captar(pool: &PgPool, session_id: Uuid, args: &Value) -> Result<Value, AgentError> {
    let (req, operacion, nombre) = match pedido_captacion(args) {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let telefono = req.telefono.clone();
    let ubicacion = req.ubicacion.clone();
    let descripcion = req.descripcion.clone();
    let solicitud = match SolicitudService::create(
        pool,
        req,
        None,
        Some("glory-ia/whatsapp".to_string()),
    )
    .await
    {
        Ok(s) => s,
        /* Fallo de validación del servicio = la IA pasó algo mal:
         * se devuelve como `error` para que se corrija en el turno. */
        Err(AppError::Validation(m) | AppError::BadRequest(m)) => {
            return Ok(json!({"error": m}));
        }
        Err(e) => return Err(mapear_error_solicitud(e)),
    };
    ClienteRepository::marcar_atencion(pool, session_id, "captacion")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let mut resumen =
        format!("Captación {operacion} en {ubicacion} — {nombre} {telefono}: {descripcion}");
    if let Some(p) = solicitud.precio_estimado {
        resumen = format!("{resumen} (estima {p})");
    }
    let resumen: String = resumen.chars().take(500).collect();
    aviso_humano(pool, session_id, "captacion", &resumen).await?;
    Ok(json!({"ok": true, "solicitud_id": solicitud.id}))
}

/* [E15] Visita: el visitante quiere ver un inmueble del catálogo. Valida
 * que el inmueble exista y esté publicado, abre la fila en `pendiente`
 * (el agente confirma día/hora) y congela la IA como `consultar` (el
 * humano confirma con nota y la IA retoma): la cita sin día cerrado no
 * es una delegación total. `fecha` solo viaja si el visitante dio un
 * día exacto; si no, el humano la fija al confirmar. */
async fn agendar(pool: &PgPool, session_id: Uuid, args: &Value) -> Result<Value, AgentError> {
    let id: Uuid = args
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .parse()
        .map_err(|_| AgentError::BadRequest("id de inmueble invalido".to_string()))?;
    /* Barato primero: sin nombre/teléfono/cuándo/fecha válidos no se toca
     * la BD (la IA se corrige en el turno sin costo de consulta). */
    let nombre = arg_texto(args, "nombre");
    let telefono = arg_texto(args, "telefono");
    let cuando = arg_texto(args, "cuando");
    if nombre.is_empty() || nombre.len() > 200 {
        return Ok(json!({"error": "nombre requerido (1..200)"}));
    }
    if !telefono_valido(&telefono) {
        return Ok(json!({"error": "telefono invalido"}));
    }
    if cuando.is_empty() || cuando.len() > 200 {
        return Ok(json!({"error": "cuando requerido (1..200)"}));
    }
    let fecha = match arg_texto(args, "fecha") {
        f if f.is_empty() => None,
        f => match chrono::NaiveDate::parse_from_str(&f, "%Y-%m-%d") {
            Ok(d) => Some(d),
            Err(_) => return Ok(json!({"error": "fecha invalida (YYYY-MM-DD)"})),
        },
    };
    let titulo: Option<String> =
        sqlx::query_scalar("SELECT titulo FROM inmuebles WHERE id = $1 AND publicado")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(titulo) = titulo.filter(|t| !t.trim().is_empty()) else {
        return Ok(json!({"error": "inmueble no disponible"}));
    };
    let visita = VisitaRepository::crear(
        pool,
        NuevaVisita {
            inmueble_id: id,
            session_id,
            nombre: nombre.clone(),
            telefono: telefono.clone(),
            cuando: cuando.clone(),
            fecha,
        },
    )
    .await
    .map_err(|e| AgentError::Db(e.to_string()))?;
    /* Congelar como `consultar`: el humano confirma día/hora y la IA
     * retoma con la nota (una cita no es delegación total). */
    ClienteRepository::marcar_atencion(pool, session_id, "consultando")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    glory_agent::persistence::set_session_ai(pool, session_id, false).await?;
    glory_agent::persistence::upsert_response_cycle(pool, session_id, "waiting").await?;
    let resumen: String = format!(
        "Visita {} — {cuando} — {nombre} {telefono} (cita {})",
        titulo.trim(),
        visita.id
    )
    .chars()
    .take(500)
    .collect();
    aviso_humano(pool, session_id, "visita", &resumen).await?;
    Ok(json!({"ok": true, "visita_id": visita.id}))
}

/* [169A-4] Las consultas SQL no usan macros verificadas en compilación:
 * estos tests las ejecutan contra la BD real de rama (`DATABASE_URL`).
 * Sin `DATABASE_URL` se omiten (gate local sin BD sigue verde). */
#[cfg(test)]
mod pruebas {
    use super::*;
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
