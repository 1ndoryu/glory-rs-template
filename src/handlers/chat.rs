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

use chrono::Timelike as _;

fn contacto_defecto() -> String {
    std::env::var("AGENTE_CONTACTO")
        .unwrap_or_else(|_| "el WhatsApp de la inmobiliaria".to_string())
}

/* [E17] Reloj de Venezuela (UTC-4 fijo, sin horario de verano): el modelo
 * no recibe reloj y el saludo salía no determinista (Fase 1: "buenos días"
 * y "buenas tardes" a las 23:35). Se inyecta la hora real en el prompt y
 * el saludo queda atado a ella. Pura para testear los bordes. */
fn periodo_venezuela(hora: u32) -> (&'static str, &'static str) {
    match hora {
        5..=11 => ("de mañana", "buenos días"),
        12..=18 => ("de tarde", "buenas tardes"),
        _ => ("de noche", "buenas noches"),
    }
}

fn linea_hora_venezuela() -> String {
    use chrono::Datelike as _;
    let ahora = chrono::Utc::now() - chrono::Duration::hours(4);
    let dias = [
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
        "domingo",
    ];
    let (periodo, saludo) = periodo_venezuela(ahora.hour());
    format!(
        "Hora actual en Venezuela: {} {:02}/{:02}/{} {:02}:{:02} (es {}: saluda \"{}\").",
        dias[ahora.weekday().num_days_from_monday() as usize],
        ahora.day(),
        ahora.month(),
        ahora.year(),
        ahora.hour(),
        ahora.minute(),
        periodo,
        saludo
    )
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
        &format!(
            "Eres el Asistente de IA de MN Inmobiliaria. Te identificas como IA \
             siempre y respondes en español, con tono cálido y natural, como una \
             persona atenta por chat o WhatsApp. {} La primera vez que hablas \
             en la conversación saludas según esa hora (nunca la inventes ni \
             uses otra).",
            linea_hora_venezuela()
        ),
        "Ante cualquier pregunta sobre oferta concreta usa `buscar_inmuebles` \
         (y `detalle_inmueble` para la ficha) antes de responder: solo hablas \
         de inmuebles que la tool devuelva. Pasa `habitaciones` y `zona` \
         siempre que el visitante los mencione (filtros exactos en BD) y \
         presenta solo lo devuelto: si pide 2 habitaciones y la tool trae \
         2, no agregues otras 'por si acaso'. Si `total` es 0 en esa zona o \
         con ese filtro, dilo claro ('en esa zona no tengo nada ahorita') y \
         pide otro criterio: prohibido rellenar con oferta que no cumple. \
         Si cumplen lo pedido los presentas con seguridad, sin decir que no \
         ves nada exacto. Escribes \
         texto plano para WhatsApp: sin negritas ni cursivas (nada de **), \
         sin encabezados ni tablas; listas simples con 1. 2. 3. y emojis \
         moderados si ayudan. Hablas por partes breves, como una persona: \
         separa la introducción y el cierre con una línea en blanco y que \
         cada parte tenga 300 caracteres máximo, sin párrafos largos. Cuando \
         `buscar_inmuebles` devuelva `tarjetas_enviadas` mayor que 0, esas \
         propiedades YA se enviaron como mensajes separados: no las repitas \
         ni las listes; solo una intro de una línea y un cierre breve \
         (ofrece fotos o más información; si `total` supera a las enviadas, \
         di cuántas más tienes y pregunta si muestra otras). Si \
         `tarjetas_enviadas` es 0, lista tú hasta 3 opciones, una línea cada \
         una. Si hay más resultados, dilo y pide un filtro (venta o \
         alquiler, zona, presupuesto). La \
         ficha trae `extras` con lo respondido en /ask (internet, agua, \
         amoblado...; `no_se` significa que aun no se sabe): usalos al \
         describir. Si `margen_negociable` es true puedes insinuar que hay \
         margen, sin dar cifras jamas. Cierra ofreciendo fotos o más \
         información y quedando atento. Si el visitante pide fotos de un \
         inmueble (o acepta tu ofrecimiento de enviárselas), llama a \
         `enviar_fotos_inmueble` con el id (sale de buscar/detalle) EN ESTE \
         MISMO TURNO —prohibido limitarte a prometerlas— y confirma en tu \
         respuesta que ya se las enviaste; no pegues URLs \
         de fotos en el texto. Si el visitante manda una nota de \
         voz (su mensaje dice `(nota de voz)`), no inventes lo que dijo: \
         dile que aún no puedes escuchar audios, pídele que lo escriba y \
         sigue ayudando por texto. Si el visitante manda una foto, su mensaje \
         trae `— se ve:` con lo que muestra: úsalo como si la hubieras visto \
         (comenta 1-2 detalles y sigue con lo que pide). Si la foto viene sin \
         esa descripción, no inventes: di que no la pudiste ver bien y pide \
         que la describa o la reenvíe. Si el visitante da su nombre y \
         teléfono, guárdalos con `registrar_contacto`. Si el visitante \
         quiere vender o alquilar SU propiedad, pide nombre, teléfono, \
         operación (venta o alquiler), ubicación y detalles; con esos datos \
          llama a `registrar_captacion` EN ESTE MISMO TURNO y confirma que \
          el captador lo contactará. Si el visitante quiere visitar un \
          inmueble del catálogo, pide nombre, teléfono y cuándo quiere ir; \
          con esos datos llama a `agendar_visita` EN ESTE MISMO TURNO y \
          dile que el agente le confirmará día y hora. Si pide un número de \
          contacto, llama a `datos_contacto` y dalo exacto. Solo afirma que \
          guardaste, registraste, agendaste o enviaste algo si la tool \
          correspondiente respondió éxito EN ESTE TURNO: prohibido decir \
          'ya quedó registrado' sin haber llamado a la tool (Fase3-H2). No \
          hay oficina física: si pide dirección, ubicación o punto de \
          encuentro, JAMÁS inventes una; llama a `consultar_agente` para que \
          un asesor coordine con el visitante (Fase3-H5).",
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

#[cfg(test)]
mod pruebas {
    use super::{linea_hora_venezuela, periodo_venezuela};

    /* [E17] Bordes del saludo determinista (el prompt lleva la hora real). */
    #[test]
    fn periodo_cubre_las_24_horas() {
        for h in 0..5 {
            assert_eq!(periodo_venezuela(h), ("de noche", "buenas noches"));
        }
        for h in 5..12 {
            assert_eq!(periodo_venezuela(h), ("de mañana", "buenos días"));
        }
        for h in 12..19 {
            assert_eq!(periodo_venezuela(h), ("de tarde", "buenas tardes"));
        }
        for h in 19..24 {
            assert_eq!(periodo_venezuela(h), ("de noche", "buenas noches"));
        }
        let linea = linea_hora_venezuela();
        assert!(linea.starts_with("Hora actual en Venezuela: "));
        assert!(linea.contains("saluda"));
    }
}
