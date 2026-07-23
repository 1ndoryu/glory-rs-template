/* [P-1 Chatbot v2] Funciones auxiliares del WS visitor.
 * Extraídas de ws_visitor.rs para mantener el límite de líneas por archivo.
 * send_history: reenvío de historial al reconectar.
 * handle_reset: comando /reset para limpiar sesión.
 * process_visitor_messages: loop principal de procesamiento de mensajes. */

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};

use crate::models::{WsClientMessage, WsServerMessage};
use crate::repositories::ChatRepository;
use crate::services::{RateCheckResult, TimingEvent};
use crate::AppState;

/* Enviar historial de mensajes al visitante al reconectar.
 * Retorna true si la sesión tenía mensajes previos, false si es nueva.
 * [096A-14] Timeout 5s por mensaje: si el WS TCP está half-open (cliente
 * desapareció sin FIN/RST), el write se bloquea indefinidamente ocupando
 * el worker tokio. Con timeout, abortamos y liberamos el handler. */
pub async fn send_history(
    state: &AppState,
    session_id: uuid::Uuid,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> Result<bool, ()> {
    if let Ok(history) =
        crate::repositories::ChatRepository::list_messages(&state.pool, session_id, 50, 0).await
    {
        let had_messages = !history.is_empty();
        for msg in history {
            let ws_msg = WsServerMessage::Message {
                id: msg.id,
                session_id: msg.session_id,
                sender: msg.sender_type,
                sender_id: msg.sender_id,
                content: msg.content,
                created_at: msg.created_at,
                message_type: msg.message_type,
                metadata: msg.metadata,
            };
            if let Ok(json) = serde_json::to_string(&ws_msg) {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    sender.send(Message::Text(json)),
                )
                .await
                {
                    Ok(Ok(())) => { /* enviado OK */ }
                    Ok(Err(_)) => {
                        tracing::debug!("send_history: WS write error, abortando");
                        return Err(());
                    }
                    Err(_) => {
                        tracing::warn!(
                            "send_history: WS write timeout (5s) para session {session_id}"
                        );
                        return Err(());
                    }
                }
            }
        }
        Ok(had_messages)
    } else {
        Ok(false)
    }
}

/* [084A-40][237A-5] `/reset` solo archiva chats anónimos y limpia su perfil.
 * Nunca borra mensajes: el historial permanece auditable. Las conversaciones
 * vinculadas a usuario u orden rechazan el comando y continúan conectadas. */
pub async fn handle_reset(state: &AppState, session_id: uuid::Uuid, visitor_id: &str) -> bool {
    let pool = &state.pool;

    let session = match ChatRepository::find_session_by_id(pool, session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            tracing::warn!("Reset: sesión inexistente session={session_id}");
            return false;
        }
        Err(error) => {
            tracing::error!("Reset: error consultando session={session_id}: {error}");
            return false;
        }
    };
    if session.order_id.is_some() || session.user_id.is_some() {
        state.chat_hub.broadcast(
            session_id,
            &WsServerMessage::Error {
                message: "Esta conversación forma parte de tu historial y no se puede borrar."
                    .to_string(),
            },
        );
        tracing::warn!("Reset rechazado para sesión vinculada session={session_id}");
        return false;
    }

    if let Err(e) = ChatRepository::delete_visitor_profile(pool, visitor_id).await {
        tracing::error!("Reset: error borrando perfil visitor={visitor_id}: {e}");
    }
    if let Err(e) = ChatRepository::close_session(pool, session_id).await {
        tracing::error!("Reset: error cerrando session={session_id}: {e}");
    }

    state
        .chat_hub
        .broadcast(session_id, &WsServerMessage::Reset);
    tracing::info!(
        "Reset archivado sin borrar mensajes: session={session_id}, visitor={visitor_id}"
    );
    true
}

enum VisitorTextFlow {
    Continue,
    Close,
}

fn truncate_visitor_message(content: String) -> String {
    let max_length = crate::services::ChatTimingService::max_message_length();
    if content.len() > max_length {
        content[..max_length].to_string()
    } else {
        content
    }
}

async fn apply_rate_limit_feedback(
    state: &AppState,
    session_id: uuid::Uuid,
    rate_result: RateCheckResult,
    rate_msg: Option<String>,
) -> Option<bool> {
    if let Some(message) = rate_msg {
        let _ = state
            .chat_hub
            .send_message(session_id, "ai", Some("ai"), &message)
            .await;
    }

    match rate_result {
        RateCheckResult::Muted => Some(false),
        RateCheckResult::Closed => Some(true),
        RateCheckResult::Warning | RateCheckResult::Ok => None,
    }
}

async fn handle_visitor_text_message(
    state: &AppState,
    session_id: uuid::Uuid,
    visitor_id: &str,
    client_ip: Option<&str>,
    timing_tx: &tokio::sync::mpsc::Sender<TimingEvent>,
    content: String,
) -> VisitorTextFlow {
    if content.trim().eq_ignore_ascii_case("/reset") {
        return if handle_reset(state, session_id, visitor_id).await {
            VisitorTextFlow::Close
        } else {
            VisitorTextFlow::Continue
        };
    }

    let content = truncate_visitor_message(content);

    tracing::debug!(%session_id, len = content.len(), "Procesando mensaje de texto del visitante");

    if let Some(ip) = client_ip {
        let (ip_result, ip_msg) = state.chat_timing.check_ip_rate(ip);
        if let Some(should_close) =
            apply_rate_limit_feedback(state, session_id, ip_result, ip_msg).await
        {
            return if should_close {
                VisitorTextFlow::Close
            } else {
                VisitorTextFlow::Continue
            };
        }
    }

    let (rate_result, rate_msg) = state.chat_timing.check_rate(visitor_id);
    if let Some(should_close) =
        apply_rate_limit_feedback(state, session_id, rate_result, rate_msg).await
    {
        return if should_close {
            VisitorTextFlow::Close
        } else {
            VisitorTextFlow::Continue
        };
    }

    let _ = state
        .chat_hub
        .send_message(session_id, "client", Some(visitor_id), &content)
        .await;
    tracing::debug!(%session_id, "Mensaje persistido y broadcast, enviando a timing channel...");
    /* [096A-8] try_send en vez de send: si el canal timing está lleno (IA ocupada),
     * NO bloquear el handler WS. send().await congela la lectura del WebSocket
     * → buffer TCP se llena → conexión se congela silenciosamente. */
    if timing_tx.try_send(TimingEvent::Message(content)).is_err() {
        tracing::warn!("Canal timing lleno, descartando mensaje para session {session_id}");
    }

    VisitorTextFlow::Continue
}

/* [T-4] Loop principal de procesamiento de mensajes del visitante.
 * Aplica rate limiting (por visitor_id + IP), persiste mensajes y envía eventos al timing service.
 * Retorna true si el cierre fue explícito (usuario o rate limit), false si el stream terminó. */
#[allow(clippy::too_many_lines)]
pub async fn process_visitor_messages(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    state: &AppState,
    session_id: uuid::Uuid,
    visitor_id: &str,
    client_ip: Option<&str>,
    timing_tx: &tokio::sync::mpsc::Sender<TimingEvent>,
) -> bool {
    /* [096A-1] Timeout de inactividad: 5 minutos sin mensajes del visitante
     * → cerrar conexión para liberar recursos y evitar acumulación de CLOSE_WAIT. */
    tracing::debug!(%session_id, "process_visitor_messages: entrando al loop");
    loop {
        let Ok(Some(Ok(msg))) =
            tokio::time::timeout(std::time::Duration::from_mins(5), receiver.next()).await
        else {
            break;
        };

        let Message::Text(text) = msg else {
            continue;
        };
        let Ok(ws_msg) = serde_json::from_str::<WsClientMessage>(&text) else {
            tracing::warn!(%session_id, "Mensaje WS con formato inválido");
            continue;
        };

        let tipo = match &ws_msg {
            WsClientMessage::Message { .. } => "message",
            WsClientMessage::Typing { .. } => "typing",
            WsClientMessage::Close => "close",
            WsClientMessage::Action { .. } => "action",
            _ => "other",
        };
        tracing::debug!(%session_id, %tipo, "Mensaje WS recibido");

        match ws_msg {
            WsClientMessage::Message { content } => {
                if matches!(
                    handle_visitor_text_message(
                        state, session_id, visitor_id, client_ip, timing_tx, content,
                    )
                    .await,
                    VisitorTextFlow::Close
                ) {
                    return true;
                }
            }
            WsClientMessage::Typing { content, .. } => {
                state.chat_hub.send_typing(session_id, "client", &content);
                /* [096A-8] try_send: typing events son no-críticos, nunca deben bloquear */
                if content.is_empty() {
                    let _ = timing_tx.try_send(TimingEvent::TypingStop);
                } else {
                    let _ = timing_tx.try_send(TimingEvent::TypingStart);
                }
            }
            WsClientMessage::Close => {
                /* [T-4] Cierre explícito del usuario — cierra para todas las conexiones */
                let _ = timing_tx.try_send(TimingEvent::Disconnect);
                return true;
            }
            /* [T-2] Acciones desde botones de mensajes ricos.
             * El frontend envía action_type + payload que se re-inyectan
             * como mensaje de texto para que la IA lo procese en contexto. */
            WsClientMessage::Action {
                action_type,
                payload,
            } => {
                let action_text = format!(
                    "[Acción: {action_type}] {}",
                    payload.as_str().unwrap_or(&payload.to_string())
                );
                let _ = state
                    .chat_hub
                    .send_message(session_id, "client", Some(visitor_id), &action_text)
                    .await;
                if timing_tx
                    .try_send(TimingEvent::Message(action_text))
                    .is_err()
                {
                    tracing::warn!(
                        "Canal timing lleno, descartando action para session {session_id}"
                    );
                }
            }
            _ => {} /* join/toggle_ai son solo para staff */
        }
    }
    /* [T-4] Stream terminó sin cierre explícito (pestaña cerrada, red caída) */
    false
}
