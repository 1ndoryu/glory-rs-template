use axum::extract::{DefaultBodyLimit, Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::game_map::{
    GameMapVersionPublic, PublishMapRequest, MAP_VERSION_MAX_JSON_BYTES,
};
use crate::services::game_map_svc::GameMapService;
use crate::AppState;

/// Publicar una nueva versión activa de un mapa del juego (admin).
#[utoipa::path(
    post,
    path = "/api/admin/game/maps",
    request_body = PublishMapRequest,
    responses(
        (status = 200, description = "Mapa publicado", body = GameMapVersionPublic),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Prohibido", body = ErrorResponse),
        (status = 409, description = "La versión activa cambió", body = ErrorResponse),
        (status = 413, description = "El documento supera el tamaño permitido"),
        (status = 422, description = "MapVersion inválido", body = ErrorResponse)
    ),
    security(("session_cookie" = []))
)]
pub async fn publish_map(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(request): Json<PublishMapRequest>,
) -> Result<Json<GameMapVersionPublic>, AppError> {
    Ok(Json(
        GameMapService::publish(&state.pool, admin.user_id, request).await?,
    ))
}

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
    Router::new()
        .route("/game/maps/:map_id", get(get_active_map))
        .route(
            "/admin/game/maps",
            axum::routing::post(publish_map)
                .layer(DefaultBodyLimit::max(MAP_VERSION_MAX_JSON_BYTES)),
        )
}
