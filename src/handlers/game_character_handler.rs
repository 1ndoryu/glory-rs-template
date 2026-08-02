use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::errors::AppError;
use crate::models::game_character::GameCharacterPublicResponse;
use crate::services::game_profile::GameProfileService;
use crate::AppState;

/// Catálogo activo de opciones visuales allowlisted para el personaje base.
#[utoipa::path(
    get,
    path = "/api/game/characters",
    responses(
        (status = 200, description = "Catálogo de personajes", body = [GameCharacterPublicResponse]),
        (status = 500, description = "Catálogo no disponible", body = ErrorResponse)
    )
)]
pub async fn list_game_characters(
    State(state): State<AppState>,
) -> Result<Json<Vec<GameCharacterPublicResponse>>, AppError> {
    Ok(Json(
        GameProfileService::list_characters(&state.pool)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/game/characters", get(list_game_characters))
}
