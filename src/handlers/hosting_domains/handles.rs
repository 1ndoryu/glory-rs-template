/* [01AA-4-f3p] Handles WHOIS de Contabo (extraido de hosting_domains.rs). */

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use super::helpers::{contabo, require_admin};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::contabo_domains::{ContaboHandle, CreateHandleRequest};
use crate::AppState;

pub(super) async fn list_handles(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ContaboHandle>>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let handles = svc.list_handles().await.map_err(AppError::Internal)?;
    Ok(Json(handles))
}

pub(super) async fn create_handle(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateHandleRequest>,
) -> Result<(StatusCode, Json<ContaboHandle>), AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let handle = svc.create_handle(&body).await.map_err(AppError::Internal)?;
    Ok((StatusCode::CREATED, Json(handle)))
}
