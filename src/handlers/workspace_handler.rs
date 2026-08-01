use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::workspace::{PublishReleaseRequest, WorkspaceRelease, WorkspaceReleasePublic};
use crate::services::workspace_svc::WorkspaceService;
use crate::AppState;

/// Response del historial de releases (admin).
#[derive(Serialize, ToSchema)]
pub struct ReleaseListResponse {
    pub items: Vec<WorkspaceRelease>,
}

/// Obtener el release activo del workspace (público).
#[utoipa::path(
    get,
    path = "/api/workspace/release",
    responses(
        (status = 200, description = "Release activo", body = WorkspaceReleasePublic),
        (status = 404, description = "No hay releases", body = crate::errors::ErrorResponse)
    )
)]
pub async fn get_active_release(
    State(state): State<AppState>,
) -> Result<Json<WorkspaceReleasePublic>, AppError> {
    let release = WorkspaceService::get_active_release(&state.pool).await?;
    Ok(Json(release))
}

/// Obtener un release por versión (admin — rollback).
#[utoipa::path(
    get,
    path = "/admin/workspace/releases/{version}",
    params(("version" = i32, Path, description = "Versión del release")),
    responses(
        (status = 200, description = "Release encontrado", body = WorkspaceReleasePublic),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse)
    )
)]
pub async fn get_release_by_version(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(version): Path<i32>,
) -> Result<Json<WorkspaceReleasePublic>, AppError> {
    let release = WorkspaceService::get_release_by_version(&state.pool, version).await?;
    Ok(Json(release))
}

/// Listar todos los releases (admin — historial).
#[utoipa::path(
    get,
    path = "/admin/workspace/releases",
    responses(
        (status = 200, description = "Historial de releases", body = ReleaseListResponse)
    )
)]
pub async fn list_releases(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<ReleaseListResponse>, AppError> {
    let releases = WorkspaceService::list_releases(&state.pool).await?;
    Ok(Json(ReleaseListResponse { items: releases }))
}

/// Publicar un nuevo release del workspace (admin).
/// [297A-11 §9.2] Publicación transaccional a release inmutable.
#[utoipa::path(
    post,
    path = "/admin/workspace/publish",
    request_body = PublishReleaseRequest,
    responses(
        (status = 201, description = "Release publicado", body = WorkspaceRelease),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Prohibido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn publish_release(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<PublishReleaseRequest>,
) -> Result<(axum::http::StatusCode, Json<WorkspaceRelease>), AppError> {
    let release = WorkspaceService::publish(&state.pool, req.tree, admin.user_id).await?;
    Ok((axum::http::StatusCode::CREATED, Json(release)))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workspace/release", get(get_active_release))
        .route("/admin/workspace/releases", get(list_releases))
        .route(
            "/admin/workspace/releases/:version",
            get(get_release_by_version),
        )
        .route("/admin/workspace/publish", post(publish_release))
}
