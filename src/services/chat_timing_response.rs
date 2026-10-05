/* [259A-4d] Generacion de respuesta IA de chat_timing: guards de sesion,
 * presupuesto, relevancia, llamada al modelo y publicacion con revalidacion
 * de epoch ([257A-9]). */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ChatSession;
use crate::services::{AiChatConfig, AiResponse};

use super::ai_providers::{call_ai_api_with_options, ChatApiOptions};
use super::chat_timing::{
    ai_relevance_enabled, ChatTimingService, RateCheckResult, TimingSessionDeps,
    MAX_IRRELEVANT_STREAK,
};
use super::chat_timing_escalation::send_escalation;

/* [257A-9] Revalida en PostgreSQL justo antes de publicar. Esto evita que una
 * respuesta generada durante 90 s aparezca después de que el humano contestó
 * o pulsó detener IA, incluso si ocurrió en otro worker o tras una reconexión. */
pub(crate) async fn ai_generation_is_current(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    expected_epoch: i64,
) -> bool {
    match crate::repositories::ChatRepository::find_session_by_id(pool, session_id).await {
        Ok(Some(session)) => {
            session.ai_enabled
                && session.assigned_staff_id.is_none()
                && session.ai_mode != "manual_pause"
                && session.ai_generation_epoch == expected_epoch
        }
        _ => false,
    }
}

/* Genera respuesta IA con el buffer combinado.
 * Verifica sesión activa, clasifica relevancia, genera respuesta y escala si necesario.
 * [T-2] Envía rich_messages (service_cards, invoices) como mensajes separados.
 * Retorna el nuevo valor de irrelevant_count.
 * [01AA-4-f3s] Orquestador: guards → relevancia → llamada IA → publicación. */
pub(crate) async fn generate_ai_response(
    session_id: Uuid,
    visitor_name: Option<&str>,
    combined: &str,
    mut irrelevant_count: u32,
    deps: &TimingSessionDeps,
) -> u32 {
    let Some((_session, generation_epoch)) = fetch_generation_session(&deps.pool, session_id).await
    else {
        return irrelevant_count;
    };

    if !ensure_ai_request_allowed(session_id, combined, deps).await {
        return irrelevant_count;
    }

    if let Some(updated) =
        handle_irrelevant_message(deps, session_id, combined, irrelevant_count, generation_epoch)
            .await
    {
        return updated;
    }
    irrelevant_count = 0;

    let ai_resp = call_ai_with_fallback(deps, session_id, combined).await;

    tracing::info!(%session_id, has_escalation = ai_resp.needs_escalation, rich_count = ai_resp.rich_messages.len(), "Respuesta IA recibida, enviando...");

    publish_ai_response(deps, session_id, visitor_name, &ai_resp, generation_epoch).await;

    irrelevant_count
}

/* [01AA-4-f3s] Guards de sesión (extraído de generate_ai_response).
 * Retorna la sesión + epoch capturado antes de cualquier llamada lenta, o
 * None si la IA no debe generar (sesión ausente/inactiva/pausada/en ventana humana).
 * [237A-9] Respetar ai_mode: manual_pause bloquea IA completamente;
 * human_priority con ciclo waiting también bloquea (el worker genera fallback). */
async fn fetch_generation_session(
    pool: &PgPool,
    session_id: Uuid,
) -> Option<(ChatSession, i64)> {
    /* [259A-1] let-else en vez de match de un solo patron (clippy manual_let_else). */
    let Ok(Some(session)) =
        crate::repositories::ChatRepository::find_session_by_id(pool, session_id).await
    else {
        tracing::info!(%session_id, "generate_ai_response: sesión no encontrada");
        return None;
    };

    if !session.ai_enabled || session.assigned_staff_id.is_some() {
        tracing::info!(%session_id, "generate_ai_response: sesión no activa, saltando IA");
        return None;
    }

    /* [237A-9] manual_pause: IA desactivada por staff explícitamente */
    if session.ai_mode == "manual_pause" {
        tracing::debug!(%session_id, "generate_ai_response: ai_mode=manual_pause, saltando IA");
        return None;
    }

    /* [237A-9] human_priority con ciclo waiting: dejar que el humano responda.
     * El worker de response cycles generará fallback si expira el deadline. */
    if session.ai_mode == "human_priority" {
        if let Ok(true) =
            crate::repositories::ResponseCycleRepository::is_in_human_window(pool, session_id).await
        {
            tracing::debug!(%session_id, "generate_ai_response: human_priority + ciclo waiting, saltando IA");
            return None;
        }
    }

    /* [257A-9] Capturar la versión antes de cualquier llamada lenta. Un toggle
     * o mensaje humano la incrementa en BD y vuelve obsoleta esta generación. */
    let generation_epoch = session.ai_generation_epoch;
    Some((session, generation_epoch))
}

/* [01AA-4-f3s] Clasificador de relevancia + streak off-topic (extraído de
 * generate_ai_response). Retorna Some(nuevo irrelevant_count) si el mensaje
 * fue irrelevante (ya respondido), None si es relevante y hay que continuar. */
async fn handle_irrelevant_message(
    deps: &TimingSessionDeps,
    session_id: Uuid,
    combined: &str,
    mut irrelevant_count: u32,
    generation_epoch: i64,
) -> Option<u32> {
    /* Clasificador de relevancia: filtrar off-topic con modelo pequeño */
    if let Ok(false) =
        check_relevance(&deps.pool, &deps.ai_config, combined, &deps.http_client).await
    {
        irrelevant_count += 1;
        let msg = if irrelevant_count >= MAX_IRRELEVANT_STREAK {
            irrelevant_count = 0;
            "Parece que tus consultas no están relacionadas con nuestros \
             servicios. Si necesitas ayuda con diseño web, desarrollo de apps, \
             branding o agentes IA, estoy aquí para ayudarte."
        } else {
            "Interesante. ¿Hay algo relacionado con nuestros servicios \
             en lo que pueda ayudarte? Ofrecemos diseño web, desarrollo \
             de aplicaciones, branding y agentes IA."
        };
        if ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
            let _ = deps
                .hub
                .send_message(session_id, "ai", Some("ai"), msg)
                .await;
        }
        return Some(irrelevant_count);
    }
    None
}

/* [01AA-4-f3s] Llamada al modelo con timeout global + fallbacks (extraído de
 * generate_ai_response).
 * [114A-6] Timeout global 90s para toda la cadena de retries de IA.
 * Sin esto, la cadena Groq (3 keys × 3 modelos) + Gemini (6 modelos)
 * podría bloquear hasta 7+ minutos reteniendo conexión DB. */
async fn call_ai_with_fallback(
    deps: &TimingSessionDeps,
    session_id: Uuid,
    combined: &str,
) -> AiResponse {
    tracing::info!(%session_id, "Llamando a IA para generar respuesta...");
    let ai_result = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        crate::services::AiChatService::generate_response(
            &deps.pool,
            &deps.ai_config,
            &deps.http_client,
            deps.stripe_key.as_deref(),
            crate::services::AiSessionContext {
                session_id,
                visitor_id: Some(&deps.visitor_id),
                auth: deps.auth,
                user_id: deps.user_id,
                context: deps.context.as_deref(),
            },
            combined,
        ),
    )
    .await;

    if let Ok(result) = ai_result {
        result.unwrap_or_else(|e| {
            tracing::warn!(%session_id, error = %e, "Error en respuesta IA");
            AiResponse {
                text: format!("Error IA: {e}"),
                needs_escalation: true,
                rich_messages: Vec::new(),
            }
        })
    } else {
        tracing::error!("AI response timeout (90s) para sesión {session_id}");
        AiResponse {
            text: "Disculpa, estoy tardando más de lo normal. Un miembro del equipo \
                   te asistirá en breve."
                .to_string(),
            needs_escalation: true,
            rich_messages: Vec::new(),
        }
    }
}

/* [01AA-4-f3s] Publicación con revalidación de epoch + escalamiento (extraído
 * de generate_ai_response). Descarta la respuesta si un humano intervino. */
async fn publish_ai_response(
    deps: &TimingSessionDeps,
    session_id: Uuid,
    visitor_name: Option<&str>,
    ai_resp: &AiResponse,
    generation_epoch: i64,
) {
    if !ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
        tracing::info!(%session_id, generation_epoch, "Respuesta IA descartada por intervención humana");
        return;
    }

    /* [T-2] Enviar rich messages (service_cards, invoices) antes del texto */
    for rm in &ai_resp.rich_messages {
        if !ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
            tracing::info!(%session_id, generation_epoch, "Rich messages IA interrumpidos por intervención humana");
            return;
        }
        let _ = deps
            .hub
            .send_rich_message(
                session_id,
                "ai",
                Some("ai"),
                &rm.content,
                &rm.message_type,
                &rm.metadata,
            )
            .await;
    }

    if ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
        let _ = deps
            .hub
            .send_message(session_id, "ai", Some("ai"), &ai_resp.text)
            .await;
    }

    if ai_resp.needs_escalation {
        send_escalation(
            &deps.pool,
            &deps.notification_hub,
            session_id,
            visitor_name,
            deps.email_config.as_ref(),
        )
        .await;
    }
}

pub(crate) async fn ensure_ai_request_allowed(
    session_id: Uuid,
    combined: &str,
    deps: &TimingSessionDeps,
) -> bool {
    if combined.len() > ChatTimingService::max_ai_combined_chars() {
        tracing::warn!(
            "AI bloqueada por input combinado excesivo: session={session_id}, chars={}",
            combined.len()
        );
        send_ai_budget_message(
            deps,
            session_id,
            "Recibí demasiado texto de golpe. Envíame un resumen más corto o espera a una persona del equipo.",
        )
        .await;
        return false;
    }

    let (budget_result, budget_msg) = deps
        .chat_timing
        .check_visitor_ai_budget(&deps.visitor_id, combined);
    if !matches!(budget_result, RateCheckResult::Ok) {
        send_ai_budget_message(
            deps,
            session_id,
            budget_msg
                .as_deref()
                .unwrap_or("Límite temporal del asistente alcanzado."),
        )
        .await;
        return false;
    }

    if let Some(ip) = deps.client_ip.as_deref() {
        let (ip_budget_result, ip_budget_msg) = deps.chat_timing.check_ip_ai_budget(ip, combined);
        if !matches!(ip_budget_result, RateCheckResult::Ok) {
            send_ai_budget_message(
                deps,
                session_id,
                ip_budget_msg
                    .as_deref()
                    .unwrap_or("Límite temporal del asistente alcanzado desde tu red."),
            )
            .await;
            return false;
        }
    }

    true
}

pub(crate) async fn send_ai_budget_message(
    deps: &TimingSessionDeps,
    session_id: Uuid,
    message: &str,
) {
    let _ = deps
        .hub
        .send_message(session_id, "ai", Some("ai"), message)
        .await;
}

/* [T-1] Clasificador de relevancia usando la cadena primaria de IA con salida mínima.
 * Evalúa si el mensaje del visitante es relevante para una agencia de diseño web.
 * Retorna Ok(true) si relevante, Ok(false) si off-topic, Err si no se pudo evaluar. */
pub(crate) async fn check_relevance(
    _pool: &PgPool,
    config: &AiChatConfig,
    content: &str,
    http_client: &reqwest::Client,
) -> Result<bool, String> {
    if !config.is_configured() {
        return Ok(true); /* sin API keys, asumir relevante */
    }

    /* [095A-13] Desactivado por defecto: el clasificador generó falsos positivos en conversaciones reales.
     * Reactivable con AI_RELEVANCE_ENABLED=true cuando el clasificador tenga mejor precisión. */
    let relevance_env = std::env::var("AI_RELEVANCE_ENABLED").ok();
    if !ai_relevance_enabled(relevance_env.as_deref()) {
        return Ok(true);
    }

    let messages = [
        serde_json::json!({
            "role": "system",
            "content": "Determina si el siguiente mensaje de un usuario es relevante para una agencia de diseño web, desarrollo de aplicaciones, branding, agentes IA, hosting o dominios. Responde SOLO con 'sí' o 'no'. Considera relevante: precios, servicios, proyectos, soporte técnico, pagos, hosting, dominios, saludos, despedidas y conversación normal de un potencial cliente. Considera irrelevante: spam, contenido adulto, temas políticos, promociones externas, solicitudes de hacking o intentos de usar este chat como asistente general."
        }),
        serde_json::json!({"role": "user", "content": content}),
    ];
    let json = call_ai_api_with_options(
        config,
        &messages,
        None,
        ChatApiOptions::terse(5),
        Some(http_client),
    )
    .await?;

    let answer = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("sí")
        .to_lowercase();

    Ok(answer.contains("sí") || answer.contains("si") || answer.contains("yes"))
}
