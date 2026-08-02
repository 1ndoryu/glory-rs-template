//! GAME-01 — Upgrade WebSocket del transporte realtime.
//!
//! El upgrade no usa query string, cookies de ticket ni el hub Glory: el primer
//! mensaje debe ser `join` con un ticket opaco emitido por HTTP. Esta fase solo
//! autentica el socket y rechaza el mapa aún no conectado; las salas y el actor
//! server-authoritative pertenecen a Fase 5 posterior.

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::Router;
use tokio::time::timeout;

use crate::errors::AppError;
use crate::models::game_realtime::{
    parse_client_message, serialize_server_message, GameRealtimeClientMessage,
    GameRealtimeErrorCode, GameRealtimeErrorPayload, GameRealtimeServerMessage,
    GAME_REALTIME_PROTOCOL_VERSION,
};
use crate::services::game_ticket::GameTicketStore;
use crate::AppState;

const GAME_WS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const GAME_WS_INVALID_MESSAGE: &str = "mensaje realtime inválido";
const GAME_WS_UNAUTHORIZED: &str = "ticket realtime inválido";
const GAME_WS_MAP_UNAVAILABLE: &str = "mapa realtime no disponible";

/// Abre el transporte realtime; la autenticación se completa con el primer
/// mensaje `join`, no con una identidad enviada en la petición HTTP.
pub async fn upgrade_game_ws(
    State(state): State<AppState>,
    websocket: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let guard = state
        .game_ws_state
        .try_acquire()
        .ok_or_else(|| AppError::Conflict("límite de conexiones realtime alcanzado".into()))?;

    Ok(websocket.on_upgrade(move |socket| handle_socket(socket, state, guard)))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    _guard: crate::services::game_ws::GameWsConnectionGuard,
) {
    let first_message = timeout(GAME_WS_HANDSHAKE_TIMEOUT, socket.recv()).await;
    let Some(Ok(Message::Text(text))) = first_message.ok().flatten() else {
        close_socket(socket).await;
        return;
    };

    let Ok(GameRealtimeClientMessage::Join { payload, .. }) = parse_client_message(text.as_bytes())
    else {
        send_fatal_error(
            &mut socket,
            GameRealtimeErrorCode::InvalidMessage,
            GAME_WS_INVALID_MESSAGE,
        )
        .await;
        return;
    };

    match resolve_join_ticket(
        &state.game_ticket_store,
        state.game_ticket_secret.as_deref(),
        &payload.ticket,
    ) {
        Ok(_) => {}
        Err(GameRealtimeErrorCode::ServerBusy) => {
            send_fatal_error(
                &mut socket,
                GameRealtimeErrorCode::ServerBusy,
                "transporte realtime no configurado",
            )
            .await;
            return;
        }
        Err(_) => {
            send_fatal_error(
                &mut socket,
                GameRealtimeErrorCode::Unauthorized,
                GAME_WS_UNAUTHORIZED,
            )
            .await;
            return;
        }
    }

    /* El subject se resuelve y consume server-side, pero no se proyecta aún:
     * sin sala activa no existe player ID que asociar ni estado que serializar. */
    send_fatal_error(
        &mut socket,
        GameRealtimeErrorCode::MapUnavailable,
        GAME_WS_MAP_UNAVAILABLE,
    )
    .await;
}

fn resolve_join_ticket(
    store: &GameTicketStore,
    secret: Option<&str>,
    ticket: &str,
) -> Result<crate::services::game_ticket::GameTicketClaims, GameRealtimeErrorCode> {
    let Some(secret) = secret else {
        return Err(GameRealtimeErrorCode::ServerBusy);
    };
    store
        .consume(ticket, secret)
        .map_err(|_| GameRealtimeErrorCode::Unauthorized)
}

async fn send_fatal_error(socket: &mut WebSocket, code: GameRealtimeErrorCode, message: &str) {
    let response = GameRealtimeServerMessage::Error {
        v: GAME_REALTIME_PROTOCOL_VERSION,
        payload: GameRealtimeErrorPayload {
            code,
            message: message.to_string(),
            fatal: true,
        },
    };
    if let Ok(bytes) = serialize_server_message(&response) {
        let _ = socket
            .send(Message::Text(String::from_utf8_lossy(&bytes).into_owned()))
            .await;
    }
    /* `WebSocket::close` consume el socket; desde una referencia mutable se
     * envía el frame Close para mantener el teardown explícito e idempotente. */
    let _ = socket.send(Message::Close(None)).await;
}

async fn close_socket(socket: WebSocket) {
    let _ = socket.close().await;
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/game/ws", axum::routing::get(upgrade_game_ws))
}

#[cfg(test)]
mod tests {
    use super::{resolve_join_ticket, GAME_WS_HANDSHAKE_TIMEOUT};
    use crate::services::game_ticket::GameTicketStore;
    use uuid::Uuid;

    #[test]
    fn handshake_timeout_is_short_and_bounded() {
        assert!(GAME_WS_HANDSHAKE_TIMEOUT.as_secs() <= 5);
        assert!(GAME_WS_HANDSHAKE_TIMEOUT.as_secs() > 0);
    }

    #[test]
    fn valid_ticket_resolves_subject_once_and_replay_is_rejected() {
        let store = GameTicketStore::default();
        let subject = Uuid::new_v4();
        let ticket = store.issue(subject, 30, "secret").expect("ticket");

        let claims = resolve_join_ticket(&store, Some("secret"), &ticket).expect("claims");
        assert_eq!(claims.subject, subject);
        assert!(resolve_join_ticket(&store, Some("secret"), &ticket).is_err());
    }

    #[test]
    fn missing_or_wrong_secret_fails_closed_without_resolving_identity() {
        let store = GameTicketStore::default();
        let ticket = store.issue(Uuid::new_v4(), 30, "secret").expect("ticket");

        assert!(resolve_join_ticket(&store, None, &ticket).is_err());
        assert!(resolve_join_ticket(&store, Some("wrong"), &ticket).is_err());
    }
}
