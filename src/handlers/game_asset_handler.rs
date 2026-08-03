use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::game_asset::{
    CreateGameAssetRequest, GameAssetAdminResponse, GameAssetPublicResponse, UpdateGameAssetRequest,
};
use crate::services::game_asset_svc::GameAssetService;
use crate::AppState;

/// Catálogo activo de assets allowlisted para el Editor de mapa y el runtime.
#[utoipa::path(
    get,
    path = "/api/game/assets",
    responses(
        (status = 200, description = "Catálogo de assets", body = [GameAssetPublicResponse]),
        (status = 500, description = "Catálogo no disponible", body = ErrorResponse)
    )
)]
pub async fn list_game_assets(
    State(state): State<AppState>,
) -> Result<Json<Vec<GameAssetPublicResponse>>, AppError> {
    Ok(Json(
        GameAssetService::list_active(&state.pool)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}

/// Listado completo del catálogo, incluidas las opciones desactivadas, para
/// el panel admin (el público nunca ve inactivas).
#[utoipa::path(
    get,
    path = "/api/admin/game/assets",
    responses(
        (status = 200, description = "Catálogo completo (activas e inactivas)", body = [GameAssetAdminResponse]),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Prohibido", body = ErrorResponse)
    ),
    security(("session_cookie" = []))
)]
pub async fn list_admin_game_assets(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<GameAssetAdminResponse>>, AppError> {
    Ok(Json(
        GameAssetService::list_all(&state.pool)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}

/// Alta de un asset allowlisted del catálogo (admin).
#[utoipa::path(
    post,
    path = "/api/admin/game/assets",
    request_body = CreateGameAssetRequest,
    responses(
        (status = 200, description = "Asset creado", body = GameAssetAdminResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Prohibido", body = ErrorResponse),
        (status = 409, description = "Ya existe un asset con ese id", body = ErrorResponse),
        (status = 422, description = "Datos inválidos", body = ErrorResponse)
    ),
    security(("session_cookie" = []))
)]
pub async fn create_game_asset(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(request): Json<CreateGameAssetRequest>,
) -> Result<Json<GameAssetAdminResponse>, AppError> {
    Ok(Json(
        GameAssetService::create(&state.pool, admin.user_id, request)
            .await?
            .into(),
    ))
}

/// Actualización completa de un asset, incluida su desactivación (admin).
#[utoipa::path(
    put,
    path = "/api/admin/game/assets/{id}",
    params(("id" = String, Path, description = "Identificador del asset")),
    request_body = UpdateGameAssetRequest,
    responses(
        (status = 200, description = "Asset actualizado", body = GameAssetAdminResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Prohibido", body = ErrorResponse),
        (status = 404, description = "Asset no encontrado", body = ErrorResponse),
        (status = 422, description = "Datos inválidos", body = ErrorResponse)
    ),
    security(("session_cookie" = []))
)]
pub async fn update_game_asset(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateGameAssetRequest>,
) -> Result<Json<GameAssetAdminResponse>, AppError> {
    Ok(Json(
        GameAssetService::update(&state.pool, admin.user_id, &id, request)
            .await?
            .into(),
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/game/assets", get(list_game_assets))
        .route(
            "/admin/game/assets",
            get(list_admin_game_assets).post(create_game_asset),
        )
        .route("/admin/game/assets/:id", put(update_game_asset))
}
