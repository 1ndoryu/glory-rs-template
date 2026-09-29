use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{CreateSolicitudRequest, OPERACIONES, TIPOS};
use crate::repositories::{ClienteRepository, NuevaVisita, VisitaRepository};
use crate::services::SolicitudService;
use glory_agent::errors::AgentError;
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
        "Busca inmuebles publicados y disponibles. Usala siempre antes de hablar de oferta concreta.",
        json!({
            "type": "object",
            "properties": {
                "texto": {"type": "string", "description": "Palabra en titulo o ubicacion"},
                "tipo": {"type": "string", "enum": ["apartamento", "casa", "local", "terreno", "townhouse"]},
                "operacion": {"type": "string", "enum": ["venta", "alquiler"]},
                "precio_max": {"type": "number"},
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
        "Guarda nombre y telefono del visitante cuando los da.",
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
         que ya se las enviaste; no pegues URLs de fotos en el texto.",
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
}

impl Herramientas {
    pub fn new(pool: PgPool, contacto_defecto: String) -> Self {
        Self {
            pool,
            contacto_defecto,
        }
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
            "buscar_inmuebles" => buscar(&pool, args).await,
            "detalle_inmueble" => detalle(&pool, args).await,
            "registrar_contacto" => registrar(&pool, ctx.session_id, args).await,
            "enviar_fotos_inmueble" => enviar_fotos(&pool, ctx.session_id, args).await,
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

async fn buscar(pool: &PgPool, args: &Value) -> Result<Value, AgentError> {
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
    let limite = args
        .get("limite")
        .and_then(Value::as_i64)
        .unwrap_or(5)
        .clamp(1, 10);
    /* [229A-1] Struct en vez de tupla de 9 (mismo patrón que `Ficha`):
     * legible y evita el lint de tipos complejos. */
    let filas: Vec<Tarjeta> =
        sqlx::query_as(
            "SELECT id, titulo, tipo, operacion, precio, ubicacion, slug, puestos, residencia \
             FROM inmuebles \
             WHERE publicado AND estado = 'disponible' \
             AND ($1::TEXT IS NULL OR titulo ILIKE '%' || $1 || '%' OR ubicacion ILIKE '%' || $1 || '%') \
             AND ($2::TEXT IS NULL OR tipo = $2) \
             AND ($3::TEXT IS NULL OR operacion = $3) \
             AND ($4::FLOAT8 IS NULL OR precio <= $4) \
             ORDER BY updated_at DESC LIMIT $5",
        )
        .bind(texto)
        .bind(tipo)
        .bind(operacion)
        .bind(precio_max)
        .bind(limite)
        .fetch_all(pool)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let items: Vec<Value> = filas
        .into_iter()
        .map(|t| {
            json!({"id": t.id, "titulo": t.titulo, "tipo": t.tipo, "operacion": t.operacion,
                   "precio": t.precio, "ubicacion": t.ubicacion, "slug": t.slug,
                   "puestos": t.puestos, "residencia": t.residencia})
        })
        .collect();
    let total = items.len();
    Ok(json!({"inmuebles": items, "total": total}))
}

/// Tarjeta breve de un inmueble para `buscar_inmuebles` (struct en vez de
/// tupla de 9: legible y evita el lint de tipos complejos). Incluye
/// `puestos` y `residencia` para que el agente responda con esos datos.
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
async fn enviar_fotos(pool: &PgPool, session_id: Uuid, args: &Value) -> Result<Value, AgentError> {
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
    for (i, clave) in claves.iter().enumerate() {
        let pie = if total > 1 {
            format!("{} ({}/{})", titulo.trim(), i + 1, total)
        } else {
            titulo.trim().to_string()
        };
        let aviso = json!({
            "session_id": session_id.to_string(),
            "destino": telefono,
            "texto": pie,
            "media_url": format!("{base}/uploads/{clave}"),
            "via": via,
            "motivo": "ia_foto",
        });
        glory_agent::persistence::enqueue_outbox(pool, "whatsapp", aviso).await?;
    }
    Ok(json!({"ok": true, "enviadas": total, "titulo": titulo}))
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
     * que nadie puede consultar. Si falla se propaga (nada silencioso). */
    ClienteRepository::registrar_y_vincular(pool, session_id, Some(nombre), telefono)
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
    glory_agent::persistence::enqueue_outbox(pool, "whatsapp", aviso).await?;
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
