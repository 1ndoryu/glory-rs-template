/* [01AA-4-F3g] Loop de respuesta del AI Chat (era parte de ai_chat.rs):
 * AiChatService (generate_response, generate_intermediary_response,
 * maybe_update_order_summary) + tool calls (ToolCallLoopContext, process_tool_calls). */

use std::fmt::Write;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::config::AiChatConfig;
use super::context::{build_context_messages, parse_escalation, tool_results_with_ids};
use crate::models::{ChatMessage, Order};
use crate::repositories::{ChatRepository, OrderRepository};
use crate::services::ai_prompts::{build_intermediary_prompt, build_system_prompt};
use crate::services::ai_providers::{call_ai_api, call_ai_api_with_options, ChatApiOptions};
use crate::services::ai_tools::{self, RichMessage};

pub struct AiChatService;

/* [T-6] Respuesta de IA con señal de escalación y mensajes ricos (T-2).
 * `needs_escalation` = true si la IA detecta que necesita intervención humana.
 * `rich_messages` contiene service_cards, invoices, etc. generados por tool calls. */
pub struct AiResponse {
    pub text: String,
    pub needs_escalation: bool,
    pub rich_messages: Vec<RichMessage>,
}

/* [T-9] Contexto de la sesión de chat para generate_response.
 * Agrupa parámetros relacionados para no exceder 7 argumentos.
 * [084A-28] Campo context para soporte contextual (hosting, servicio, etc.) */
pub struct AiSessionContext<'a> {
    pub session_id: Uuid,
    pub visitor_id: Option<&'a str>,
    pub auth: Option<ai_tools::ToolAuthContext>,
    pub user_id: Option<Uuid>,
    pub context: Option<&'a str>,
}

impl AiChatService {
    /// Genera respuesta de IA con soporte para tool use (function calling).
    /// Loop: envía mensajes -> si la IA llama tools -> ejecuta -> reenvía resultados -> repite.
    /// Máximo 3 iteraciones de tool calls para prevenir loops infinitos.
    pub async fn generate_response(
        pool: &PgPool,
        config: &AiChatConfig,
        http_client: &reqwest::Client,
        stripe_key: Option<&str>,
        ctx: AiSessionContext<'_>,
        user_message: &str,
    ) -> Result<AiResponse, String> {
        if !config.is_configured() {
            tracing::warn!("AI: sin API keys configuradas, usando fallback");
            return Ok(AiResponse {
                text: "Un miembro del equipo se conectará pronto para ayudarte. \
                      Mientras tanto, ¿en qué puedo orientarte?"
                    .to_string(),
                needs_escalation: true,
                rich_messages: Vec::new(),
            });
        }

        let resolved_user_id = ctx.auth.map(|auth| auth.user_id).or(ctx.user_id);
        let system_prompt = build_system_prompt(
            pool,
            ctx.session_id,
            ctx.visitor_id,
            resolved_user_id,
            ctx.auth,
            ctx.context,
        )
        .await;
        let history = ChatRepository::list_messages(pool, ctx.session_id, 20, 0)
            .await
            .unwrap_or_default();

        let mut messages = build_context_messages(&system_prompt, &history, user_message);
        messages.push(serde_json::json!({"role": "user", "content": user_message}));

        let tools = ai_tools::tool_definitions();
        let mut rich_messages: Vec<RichMessage> = Vec::new();
        let mut needs_escalation = false;

        /* [T-2] Tool call loop: máximo 3 iteraciones */
        for iteration in 0..3 {
            let resp = call_ai_api(config, &messages, Some(&tools), Some(http_client)).await?;

            let choice = &resp["choices"][0];
            let tool_calls = &choice["message"]["tool_calls"];

            if tool_calls.is_array() && tool_calls.as_array().is_some_and(|a| !a.is_empty()) {
                /* La IA quiere llamar tools: ejecutarlas */
                messages.push(choice["message"].clone());
                let tool_results = process_tool_calls(
                    ToolCallLoopContext {
                        pool,
                        http_client,
                        stripe_key,
                        visitor_id: ctx.visitor_id,
                        auth: ctx.auth,
                        session_id: ctx.session_id,
                    },
                    tool_calls,
                    &mut rich_messages,
                )
                .await;

                if tool_results
                    .iter()
                    .any(|r| r.contains("\"status\":\"escalated\""))
                {
                    needs_escalation = true;
                }

                for (tool_call_id, result) in tool_results_with_ids(tool_calls, &tool_results) {
                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call_id,
                        "content": result
                    }));
                }

                tracing::debug!(
                    "AI tool call iteration {}, {} tools executed",
                    iteration,
                    tool_results.len()
                );
                continue;
            }

            /* Respuesta de texto final (sin tool calls) */
            let text = choice["message"]["content"].as_str().unwrap_or("");
            if text.is_empty() {
                return Err("AI: respuesta vacía después de tool calls".to_string());
            }

            let (clean_text, text_escalation) = parse_escalation(text);
            return Ok(AiResponse {
                text: clean_text,
                needs_escalation: needs_escalation || text_escalation,
                rich_messages,
            });
        }

        /* Si agotamos iteraciones de tools, generar sin tools */
        let resp = call_ai_api(config, &messages, None, Some(http_client)).await?;
        let text = resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Disculpa, hubo un problema procesando tu solicitud.");
        let (clean_text, text_escalation) = parse_escalation(text);
        Ok(AiResponse {
            text: clean_text,
            needs_escalation: needs_escalation || text_escalation,
            rich_messages,
        })
    }

    /* [T-10] Genera respuesta IA como intermediario de una orden.
     * System prompt especial con contexto completo del pedido. */
    pub async fn generate_intermediary_response(
        pool: &PgPool,
        config: &AiChatConfig,
        http_client: &reqwest::Client,
        session_id: Uuid,
        order: &Order,
        user_id: Uuid,
        _user_message: &str,
    ) -> Result<AiResponse, String> {
        if !config.is_configured() {
            return Ok(AiResponse {
                text: "El equipo responderá pronto.".to_string(),
                needs_escalation: false,
                rich_messages: Vec::new(),
            });
        }

        let system_prompt = build_intermediary_prompt(pool, order, user_id).await;
        let history = ChatRepository::list_messages(pool, session_id, 20, 0)
            .await
            .unwrap_or_default();

        let mut messages = vec![serde_json::json!({"role": "system", "content": system_prompt})];
        for msg in &history {
            let role = match msg.sender_type.as_str() {
                "ai" | "ai_intermediary" => "assistant",
                _ => "user",
            };
            messages.push(serde_json::json!({"role": role, "content": msg.content}));
        }

        let resp = call_ai_api(config, &messages, None, Some(http_client)).await?;
        let text = resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Disculpa, no pude procesar tu mensaje. Un miembro del equipo te asistirá.");
        let (clean_text, escalation) = parse_escalation(text);
        Ok(AiResponse {
            text: clean_text,
            needs_escalation: escalation,
            rich_messages: Vec::new(),
        })
    }

    /* [T-10c] Genera resumen de la conversación de la orden si hay >5 msgs.
     * Usa modelo ligero (8b), reemplaza ai_summary de la orden. */
    pub async fn maybe_update_order_summary(
        pool: &PgPool,
        config: &AiChatConfig,
        http_client: &reqwest::Client,
        order_id: Uuid,
        msgs: &[ChatMessage],
    ) {
        let mut conversation = String::new();
        for m in msgs.iter().take(50) {
            let _ = writeln!(conversation, "[{}]: {}", m.sender_type, m.content);
        }

        let prompt = format!(
            "Resume esta conversación de soporte de pedido en máximo 200 palabras. \
             Incluye: solicitudes del cliente, cambios pedidos, estado emocional, \
             acciones pendientes.\n\nConversación:\n{conversation}"
        );

        let messages = [
            serde_json::json!({"role": "system", "content": "Eres un asistente que genera resúmenes concisos de conversaciones de soporte."}),
            serde_json::json!({"role": "user", "content": prompt}),
        ];

        if let Ok(json) = call_ai_api_with_options(
            config,
            &messages,
            None,
            ChatApiOptions::terse(400),
            Some(http_client),
        )
        .await
        {
            if let Some(summary) = json["choices"][0]["message"]["content"].as_str() {
                let _ = OrderRepository::update_ai_summary(pool, order_id, summary).await;
            }
        }
    }
}

/* [174A-2] Providers extraídos a ai_providers.rs */

struct ToolCallLoopContext<'a> {
    pool: &'a PgPool,
    http_client: &'a reqwest::Client,
    stripe_key: Option<&'a str>,
    visitor_id: Option<&'a str>,
    auth: Option<ai_tools::ToolAuthContext>,
    session_id: Uuid,
}

/* [T-2] Ejecutar tool calls y recopilar rich messages.
 * Retorna Vec<String> con los resultados JSON de cada tool.
 * [T-3] visitor_id para tools que actualizan visitor_profiles.
 * [124A-CHAT2] session_id para actualizar visitor_name en la sesión.
 * [095A-8] ToolCallLoopContext evita firmas crecientes al sumar tools de hosting. */
async fn process_tool_calls(
    ctx: ToolCallLoopContext<'_>,
    tool_calls: &Value,
    rich_messages: &mut Vec<RichMessage>,
) -> Vec<String> {
    let Some(calls) = tool_calls.as_array() else {
        return Vec::new();
    };

    let mut results = Vec::with_capacity(calls.len());
    for call in calls {
        let name = call["function"]["name"].as_str().unwrap_or("");
        /* [084A-52] Parsing defensivo: Gemini puede retornar arguments como
         * objeto JSON directo en vez de string JSON (OpenAI spec = string).
         * Si es string -> parse; si es objeto -> usar directo; si es otro -> vacío. */
        let args: Value = if let Some(s) = call["function"]["arguments"].as_str() {
            serde_json::from_str(s).unwrap_or_default()
        } else if call["function"]["arguments"].is_object() {
            call["function"]["arguments"].clone()
        } else {
            Value::Object(serde_json::Map::new())
        };
        tracing::debug!("AI tool call solicitada: {name}");

        let result = ai_tools::execute_tool(
            ai_tools::ToolExecutionContext {
                pool: ctx.pool,
                http_client: ctx.http_client,
                stripe_key: ctx.stripe_key,
                visitor_id: ctx.visitor_id,
                auth: ctx.auth,
                session_id: ctx.session_id,
            },
            name,
            &args,
        )
        .await;
        let status = super::context::tool_result_status(&result.tool_result_json);
        tracing::info!(
            session_id = %ctx.session_id,
            tool = %name,
            status = %status,
            user_id = ?ctx.auth.map(|auth| auth.user_id),
            effective_role = ?ctx.auth.map(|auth| auth.effective_role),
            "AI tool ejecutada"
        );
        if let Some(rm) = result.rich_message {
            rich_messages.push(rm);
        }
        results.push(result.tool_result_json);
    }
    results
}
