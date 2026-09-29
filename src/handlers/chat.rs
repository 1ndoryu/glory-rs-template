/* [169A-1] Chat del visitante con IA (F5): cablea el crate agnostico
 * `glory-agent` (regla 17: lo reutilizable vive en el nucleo) con la
 * identidad del producto.
 * [169A-4] F7: registra las 5 tools (`chat_tools`), comparte el `ChatHub`
 * con las rutas staff (el humano responde por el mismo WS) y expone
 * `/api/agent/info` + `/api/agent/sesiones/:id/contacto` sin auth.
 * Gotcha: `transport::routes()` usa `AgentState` propio, no `AppState`;
 * por eso se anida con `with_state` (Router<()>) en vez de `merge`.
 * Sin `OPENCODE_GO_API_KEY` el chat persiste + reenvia en realtime pero
 * la IA no responde (degradado verificado en glory-agent F2). */

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::chat_tools;
use super::whatsapp;
use crate::repositories::ClienteRepository;
use glory_agent::errors::AgentError;
use glory_agent::session::ChatHub;

fn contacto_defecto() -> String {
    std::env::var("AGENTE_CONTACTO")
        .unwrap_or_else(|_| "el WhatsApp de la inmobiliaria".to_string())
}

/// Identidad del producto: la IA se presenta como tal, consulta inmuebles
/// reales con sus tools (nunca inventa precios/direcciones), capta nombre
/// y teléfono, da el teléfono oficial (vía `datos_contacto`, nunca de
/// memoria) y escala a humano cuando toca.
/// [289A-2] Tono comercial cálido (decisión usuaria 2026-09-28): saludo
/// según la hora de Venezuela, máximo 3 opciones relevantes descritas con
/// palabras propias (nunca el título tal cual ni listas largas) y cierre
/// ofreciendo fotos o más información.
/// [289A-10] Texto plano para `WhatsApp` (decisión usuaria 2026-09-28): sin
/// markdown (nada de `**`, ni encabezados ni tablas), listas 1. 2. 3.,
/// emojis sí.
/// [299A-1 E12] Notas de voz: si el último mensaje del visitante es
/// `(nota de voz)` (audio que aún no se transcribe), no inventes su
/// contenido: di que todavía no puedes escuchar audios, pide que lo escriba
/// o resume por texto, y ofrece seguimiento por el teléfono oficial.
fn prompt_config() -> glory_agent::prompts::PromptConfig {
    let contacto = contacto_defecto();
    glory_agent::prompts::PromptConfig::new(
        "Asistente de IA de MN Inmobiliaria",
        "Eres el Asistente de IA de MN Inmobiliaria. Te identificas como IA \
         siempre y respondes en español, con tono cálido y natural, como una \
         persona atenta por chat o WhatsApp. La primera vez que hablas en la \
         conversación saludas según la hora de Venezuela (buenos días de \
         mañana, buenas tardes de tarde, buenas noches de noche).",
        "Ante cualquier pregunta sobre oferta concreta usa `buscar_inmuebles` \
         (y `detalle_inmueble` para la ficha) antes de responder: solo hablas \
         de inmuebles que la tool devuelva, y si cumplen lo pedido los \
         presentas con seguridad, sin decir que no ves nada exacto. Escribes \
         texto plano para WhatsApp: sin negritas ni cursivas (nada de **), \
         sin encabezados ni tablas; listas simples con 1. 2. 3. y emojis \
         moderados si ayudan. Ofrece \
         como máximo 3 opciones relevantes: describe cada una con tus \
         palabras (zona, tipo, operación, precio y un detalle), sin copiar \
         el título tal cual ni pasar listas largas. Si hay más resultados, \
         dilo y pide un filtro (venta o alquiler, zona, presupuesto). La \
         ficha trae `extras` con lo respondido en /ask (internet, agua, \
         amoblado...; `no_se` significa que aun no se sabe): usalos al \
         describir. Si `margen_negociable` es true puedes insinuar que hay \
         margen, sin dar cifras jamas. Cierra ofreciendo fotos o más \
         información y quedando atento. Si el visitante manda una nota de \
         voz (su mensaje dice `(nota de voz)`), no inventes lo que dijo: \
         dile que aún no puedes escuchar audios, pídele que lo escriba y \
         sigue ayudando por texto. Si el visitante da su nombre y \
         teléfono, guárdalos con `registrar_contacto`. Si pide un número de \
         contacto, llama a `datos_contacto` y dalo exacto.",
        &format!(
            "Si el visitante pide un humano o das 2 respuestas sin resolver, \
             llama a `escalar_a_humano` con el motivo y ofrece seguimiento por \
             {contacto}."
        ),
    )
}

/// Router del chat con estado propio (`AgentState`), listo para anidar.
/// Comparte `hub` con las rutas staff para el realtime humano→visitante.
pub fn agent_router(pool: sqlx::PgPool, hub: ChatHub) -> Router<()> {
    let provider = glory_agent::providers::ProviderConfig::opencode_go(
        std::env::var("OPENCODE_GO_API_KEY").unwrap_or_default(),
    );
    let mut registro = glory_agent::tools::ToolRegistry::new();
    for def in chat_tools::definiciones() {
        registro.register(def);
    }
    let executor: Arc<dyn glory_agent::tools::ToolExecutor> = Arc::new(
        chat_tools::Herramientas::new(pool.clone(), contacto_defecto()),
    );
    let mut state = glory_agent::transport::AgentState::new(provider, prompt_config())
        .with_pool(pool)
        .with_hub(hub);
    state.tools = Arc::new(registro);
    state.executor = Some(executor);
    glory_agent::transport::routes()
        .route("/agent/info", get(info))
        .route("/agent/sesiones/:id/contacto", post(guardar_contacto))
        .merge(whatsapp::whatsapp_routes())
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct InfoPublica {
    #[serde(rename = "contactoTelefono")]
    contacto_telefono: String,
    #[serde(rename = "whatsappUrl")]
    whatsapp_url: String,
    #[serde(rename = "iaHabilitada")]
    ia_habilitada: bool,
}

/// Datos públicos del chat para el widget (teléfono, `WhatsApp`, IA on/off).
async fn info(
    State(state): State<glory_agent::transport::AgentState>,
) -> Result<Json<InfoPublica>, AgentError> {
    let pool = state
        .pool
        .clone()
        .ok_or_else(|| AgentError::Internal("sin BD".to_string()))?;
    let contacto = chat_tools::contacto_publico(&pool, &contacto_defecto()).await?;
    let global = glory_agent::persistence::get_config(&pool, "ai_enabled_global").await?;
    Ok(Json(InfoPublica {
        contacto_telefono: contacto
            .get("telefono")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        whatsapp_url: contacto
            .get("whatsapp_url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        ia_habilitada: global.as_deref() != Some("off"),
    }))
}

#[derive(Debug, Deserialize)]
struct ContactoVisitante {
    nombre: String,
    telefono: String,
}

/// El visitante deja nombre+teléfono (widget o tool). Validado en boundary.
async fn guardar_contacto(
    State(state): State<glory_agent::transport::AgentState>,
    Path(id): Path<Uuid>,
    Json(input): Json<ContactoVisitante>,
) -> Result<Json<serde_json::Value>, AgentError> {
    let pool = state
        .pool
        .clone()
        .ok_or_else(|| AgentError::Internal("sin BD".to_string()))?;
    let nombre = input.nombre.trim();
    let telefono = input.telefono.trim();
    if nombre.is_empty() || nombre.len() > 80 {
        return Err(AgentError::BadRequest(
            "nombre requerido (1..80)".to_string(),
        ));
    }
    if !chat_tools::telefono_valido(telefono) {
        return Err(AgentError::BadRequest("telefono invalido".to_string()));
    }
    glory_agent::persistence::ensure_session(&pool, id).await?;
    glory_agent::persistence::set_session_contact(&pool, id, Some(nombre), Some(telefono)).await?;
    /* [279A-2 F1] Igual que la tool: el widget también crea `clientes`
     * (una fila por teléfono, upsert idempotente). Si falla se propaga. */
    ClienteRepository::registrar_y_vincular(&pool, id, Some(nombre), telefono)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}
