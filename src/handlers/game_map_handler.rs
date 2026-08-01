use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::errors::AppError;
use crate::models::game_map::GameMapVersionPublic;
use crate::services::game_map_svc::GameMapService;
use crate::AppState;

/// Obtener el snapshot publicado activo de un mapa del juego.
#[utoipa::path(
    get,
    path = "/api/game/maps/{map_id}",
    params(("map_id" = String, Path, description = "Identificador público del mapa")),
    responses(
        (status = 200, description = "Mapa publicado", body = GameMapVersionPublic),
        (status = 400, description = "Identificador inválido", body = ErrorResponse),
        (status = 404, description = "Mapa no encontrado", body = ErrorResponse),
        (status = 500, description = "Snapshot inválido", body = ErrorResponse)
    )
)]
pub async fn get_active_map(
    State(state): State<AppState>,
    Path(map_id): Path<String>,
) -> Result<Json<GameMapVersionPublic>, AppError> {
    Ok(Json(
        GameMapService::get_active(&state.pool, &map_id).await?,
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/game/maps/:map_id", get(get_active_map))
}
