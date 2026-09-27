use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{OPERACIONES, TIPOS};
use crate::repositories::ClienteRepository;
use glory_agent::errors::AgentError;
use glory_agent::tools::{ToolCtx, ToolDefinition, ToolExecutor};

/* [169A-4] Tools de la inmobiliaria (las ejecuta el loop F6 del núcleo).
 * Salidas compactas: cada token cuenta para la ventana de 30k. La IA nunca
 * inventa datos: precios/direcciones salen de `buscar/detalle`, el teléfono
 * de `datos_contacto` y el humano de `escalar_a_humano`. */

/// Definiciones para el provider (schemas cortos, en español).
pub fn definiciones() -> Vec<ToolDefinition> {
    vec![
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
        ),
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
        ),
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
        ),
        ToolDefinition::new(
            "datos_contacto",
            "Telefono y WhatsApp oficiales de la inmobiliaria. Llamala antes de dar un numero.",
            json!({"type": "object", "properties": {}}),
        ),
        ToolDefinition::new(
            "escalar_a_humano",
            "Deriva la conversacion a un humano: avisa por WhatsApp al admin y frena a la IA. Usala si el visitante pide un humano o das 2 respuestas sin resolver.",
            json!({
                "type": "object",
                "properties": {"motivo": {"type": "string"}},
                "required": ["motivo"]
            }),
        ),
    ]
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
        let pool = self.pool(ctx);
        if deshabilitada(&pool, name).await {
            return Ok(json!({"error": "herramienta deshabilitada por el administrador"}));
        }
        match name {
            "buscar_inmuebles" => buscar(&pool, args).await,
            "detalle_inmueble" => detalle(&pool, args).await,
            "registrar_contacto" => registrar(&pool, ctx.session_id, args).await,
            "datos_contacto" => contacto_publico(&pool, &self.contacto_defecto).await,
            "escalar_a_humano" => escalar(&pool, ctx.session_id, args).await,
            otro => Ok(json!({"error": format!("tool desconocida: {otro}")})),
        }
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

async fn escalar(pool: &PgPool, session_id: Uuid, args: &Value) -> Result<Value, AgentError> {
    let motivo = args
        .get("motivo")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if motivo.is_empty() || motivo.len() > 300 {
        return Ok(json!({"error": "motivo requerido (1..300)"}));
    }
    glory_agent::persistence::set_session_status(pool, session_id, "escalated").await?;
    glory_agent::persistence::upsert_response_cycle(pool, session_id, "escalated").await?;
    glory_agent::persistence::enqueue_outbox(
        pool,
        "whatsapp",
        json!({"session_id": session_id.to_string(), "motivo": motivo}),
    )
    .await?;
    let telefono = glory_agent::persistence::get_config(pool, "contacto_telefono")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty());
    Ok(json!({"ok": true, "telefono": telefono.unwrap_or_default()}))
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
}
