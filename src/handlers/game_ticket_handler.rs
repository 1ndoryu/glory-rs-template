use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::game_ticket::GAME_TICKET_DEFAULT_TTL_SECS;
use crate::AppState;

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct GameTicketResponse {
    /// Ticket opaco de un solo uso para el futuro transporte realtime.
    pub ticket: String,
}

/// Emite un ticket corto para conectar el juego.
///
/// La identidad procede exclusivamente de `AuthUser`; el cliente no puede
/// elegir el subject. El endpoint no abre todavía WebSocket ni crea salas.
#[utoipa::path(
    post,
    path = "/api/game/ticket",
    responses(
        (status = 200, description = "Ticket de juego emitido", body = GameTicketResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "CSRF inválido", body = ErrorResponse),
        (status = 500, description = "Ticket no configurado", body = ErrorResponse)
    ),
    security(("session_cookie" = []))
)]
pub async fn issue_game_ticket(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<GameTicketResponse>, AppError> {
    let secret = state
        .game_ticket_secret
        .as_deref()
        .ok_or_else(|| AppError::Internal("secreto de tickets de juego no configurado".into()))?;
    let ticket =
        state
            .game_ticket_store
            .issue(auth.user_id, GAME_TICKET_DEFAULT_TTL_SECS, secret)?;
    Ok(Json(GameTicketResponse { ticket }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/game/ticket", post(issue_game_ticket))
}
