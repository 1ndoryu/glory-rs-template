/* [P-1 Chatbot v2] WS handler del staff (autenticado con JWT).
 * Endpoint: /ws/chat/staff?token=JWT
 * Recibe sesiones activas al conectar, se suscribe a nuevas sesiones,
 * puede join/toggle_ai/close sesiones individuales. */

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::{SinkExt, StreamExt};
use uuid::Uuid;

use crate::models::{UserRole, WsClientMessage, WsServerMessage};
use crate::repositories::{ChatRepository, OrderRepository};
use crate::AppState;

use super::StaffWsParams;

/* ============================================================
WEBSOCKET: STAFF (autenticado con JWT)
============================================================ */

async fn ws_staff(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<StaffWsParams>,
) -> impl IntoResponse {
    /* Verificar JWT desde query param (WS no soporta headers custom) */
    let Ok(claims) = crate::services::AuthService::verify_token(&params.token, &state.jwt_secret)
    else {
        return (StatusCode::UNAUTHORIZED, "Token inválido").into_response();
    };

    if !matches!(claims.effective_role, UserRole::Admin | UserRole::Employee) {
        return (StatusCode::FORBIDDEN, "Rol sin acceso al canal staff").into_response();
    }

    ws.on_upgrade(move |socket| handle_staff_ws(socket, state, claims.sub, claims.effective_role))
        .into_response()
}

/* [237A-6] Admin supervisa cualquier sesión; el empleado solo la asignada
 * actualmente en la orden o en la propia conversación. */
async fn staff_can_access_session(
    state: &AppState,
    staff_id: Uuid,
    role: UserRole,
    session_id: Uuid,
) -> bool {
    if role == UserRole::Admin {
        return true;
    }
    let Ok(Some(session)) = ChatRepository::find_session_by_id(&state.pool, session_id).await
    else {
        return false;
    };
    if let Some(order_id) = session.order_id {
        return OrderRepository::get_order_participants(&state.pool, order_id)
            .await
            .ok()
            .flatten()
            .is_some_and(|(_, assigned)| assigned == Some(staff_id));
    }
    session.assigned_staff_id == Some(staff_id)
}

async fn handle_staff_ws(socket: WebSocket, state: AppState, staff_id: Uuid, role: UserRole) {
    let (mut ws_sender, mut receiver) = socket.split();

    let hub = state.chat_hub.clone();
    let pool = state.pool.clone();

    /* Canal interno: las suscripciones a sesiones envían aquí, un task central escribe al WS */
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsServerMessage>(128);

    /* Enviar lista de sesiones activas al conectar.
     * [096A-14] 5s timeout para evitar bloqueo en TCP half-open. */
    let initial_sessions = if role == UserRole::Admin {
        hub.list_all_sessions().await
    } else {
        hub.list_sessions_for_user(staff_id).await
    };
    if let Ok(sessions) = initial_sessions {
        let init_msg = serde_json::json!({
            "type": "init",
            "sessions": sessions
        });
        if let Ok(json) = serde_json::to_string(&init_msg) {
            let send_fut = ws_sender.send(Message::Text(json));
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), send_fut).await;
        }
    }

    /* [064A-68][237A-6] El canal global contiene sesiones de todos los clientes
     * y queda reservado al admin supervisor. Empleados reciben realtime solo
     * tras autorizar y unirse a sus sesiones asignadas. */
    let staff_sub = if role == UserRole::Admin {
        let mut staff_rx = hub.subscribe_staff().await;
        let tx_staff = tx.clone();
        Some(tokio::spawn(async move {
            while let Some(server_msg) = staff_rx.recv().await {
                if tx_staff.send(server_msg).await.is_err() {
                    break;
                }
            }
        }))
    } else {
        None
    };

    /* Task: leer del mpsc y enviar al WS.
     * [096A-14] 5s timeout en cada envío WS para no bloquear el task ante TCP half-open. */
    let send_task = tokio::spawn(async move {
        while let Some(server_msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&server_msg) {
                let send_fut = ws_sender.send(Message::Text(json));
                if tokio::time::timeout(std::time::Duration::from_secs(5), send_fut)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    /* Track de suscripciones */
    let mut subscriptions = Vec::new();

    /* Recibir mensajes del staff.
     * [096A-1] Timeout de inactividad: 5 minutos sin mensajes → cerrar conexión. */
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
            continue;
        };

        match ws_msg {
            WsClientMessage::Join { session_id } => {
                if !staff_can_access_session(&state, staff_id, role, session_id).await {
                    tracing::warn!(%staff_id, %session_id, "Join de chat staff rechazado");
                    continue;
                }

                /* Suscribirse al canal de esta sesión → reenviar al mpsc
                 * [096A-13] subscribe() devuelve mpsc::UnboundedReceiver. */
                let mut session_rx = hub.subscribe(session_id);
                let tx_clone = tx.clone();
                let handle = tokio::spawn(async move {
                    while let Some(server_msg) = session_rx.recv().await {
                        if tx_clone.send(server_msg).await.is_err() {
                            break;
                        }
                    }
                });
                subscriptions.push(handle);
            }
            WsClientMessage::Message { content } => {
                tracing::debug!(
                    "Staff message (sin session_id en WsClientMessage::Message): {content}"
                );
            }
            WsClientMessage::Typing {
                content,
                session_id,
            } => {
                /* [104A-40] Broadcast typing del staff al visitante de la sesión indicada.
                 * session_id es obligatorio para staff (puede estar en varias sesiones).
                 * Gotcha: WsClientMessage::Typing no lo tenía antes → fix aquí. */
                if let Some(sid) = session_id {
                    if staff_can_access_session(&state, staff_id, role, sid).await {
                        hub.send_typing(sid, "staff", &content);
                    }
                }
            }
            WsClientMessage::ToggleAi {
                session_id,
                enabled,
            } => {
                if staff_can_access_session(&state, staff_id, role, session_id).await {
                    if let Err(error) = hub.toggle_ai(session_id, enabled).await {
                        tracing::error!(%session_id, %staff_id, %error, "No se pudo cambiar estado IA");
                    }
                } else {
                    tracing::warn!(%staff_id, %session_id, "Toggle IA de chat rechazado");
                }
            }
            WsClientMessage::Close => {
                break;
            }
            /* [T-2] Action no aplica a staff — ignorar */
            WsClientMessage::Action { .. } => {}
        }
    }

    /* Cleanup */
    for handle in subscriptions {
        handle.abort();
    }
    if let Some(staff_sub) = staff_sub {
        staff_sub.abort();
    }
    send_task.abort();

    let _ = pool;
}

/* ============================================================
ROUTES (Staff WS — montadas junto a ws_visitor en root)
============================================================ */

pub fn ws_staff_routes() -> Router<AppState> {
    Router::new().route("/ws/chat/staff", get(ws_staff))
}
