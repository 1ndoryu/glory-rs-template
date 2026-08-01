use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::{AdminUser, AuthUser};
use crate::models::notification::{
    CreateNotificationRequest, Notification, NotificationList, UpdateNotificationStatusRequest,
};
use crate::services::notification_svc::NotificationService;
use crate::AppState;

pub async fn list_public(
    State(state): State<AppState>,
) -> Result<Json<NotificationList>, AppError> {
    Ok(Json(NotificationService::list_public(&state.pool).await?))
}

pub async fn list_mine(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<NotificationList>, AppError> {
    Ok(Json(
        NotificationService::list_for_user(&state.pool, auth.user_id).await?,
    ))
}

pub async fn mark_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    NotificationService::mark_read(&state.pool, auth.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_admin(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<NotificationList>, AppError> {
    Ok(Json(NotificationService::list_admin(&state.pool).await?))
}

pub async fn create_admin(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(request): Json<CreateNotificationRequest>,
) -> Result<Json<Notification>, AppError> {
    Ok(Json(
        NotificationService::create(&state.pool, request, admin.user_id).await?,
    ))
}

pub async fn update_status_admin(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateNotificationStatusRequest>,
) -> Result<Json<Notification>, AppError> {
    Ok(Json(
        NotificationService::update_status(&state.pool, id, request).await?,
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list_public))
        .route("/me/notifications", get(list_mine))
        .route("/notifications/:id/read", post(mark_read))
        .route("/admin/notifications", get(list_admin).post(create_admin))
        .route(
            "/admin/notifications/:id/status",
            patch(update_status_admin),
        )
}
