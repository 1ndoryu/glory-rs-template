/* [01AA-4-f3p] CRUD de dominios Contabo (extraido de hosting_domains.rs).
 * Solo admin puede comprar/cancelar/transferir; lectura admin + clientes. */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::helpers::{contabo, require_admin};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::contabo_domains::{
    ContaboDomain, DomainHandles, Nameserver, OrderDomainRequest,
};
use crate::AppState;

pub(super) async fn list_domains(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ContaboDomain>>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let domains = svc.list_domains().await.map_err(AppError::Internal)?;
    Ok(Json(domains))
}

pub(super) async fn get_domain(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(domain): Path<String>,
) -> Result<Json<ContaboDomain>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let d = svc.get_domain(&domain).await.map_err(AppError::Internal)?;
    Ok(Json(d))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(super) struct OrderDomainBody {
    pub domain: String,
    pub auth_code: Option<String>,
    pub handles: DomainHandles,
    pub nameservers: Vec<Nameserver>,
}

pub(super) async fn order_domain(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<OrderDomainBody>,
) -> Result<(StatusCode, Json<ContaboDomain>), AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;

    let req = OrderDomainRequest {
        domain: body.domain,
        auth_code: body.auth_code,
        handles: body.handles,
        nameservers: body.nameservers,
        resource_type: None,
        resource_id: None,
    };

    let domain = svc.order_domain(&req).await.map_err(AppError::Internal)?;
    Ok((StatusCode::CREATED, Json(domain)))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(super) struct UpdateDomainBody {
    pub nameservers: Option<Vec<Nameserver>>,
    pub handles: Option<DomainHandles>,
}

pub(super) async fn update_domain(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(domain): Path<String>,
    Json(body): Json<UpdateDomainBody>,
) -> Result<Json<ContaboDomain>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let d = svc
        .update_domain(&domain, body.nameservers, body.handles)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(d))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(super) struct CancelDomainBody {
    pub reason: Option<String>,
}

pub(super) async fn cancel_domain(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(domain): Path<String>,
    Json(body): Json<CancelDomainBody>,
) -> Result<StatusCode, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    svc.cancel_domain(&domain, body.reason.as_deref())
        .await
        .map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, ToSchema)]
pub(super) struct AuthCodeResponse {
    pub domain: String,
    pub auth_code: String,
}

pub(super) async fn get_auth_code(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(domain): Path<String>,
) -> Result<Json<AuthCodeResponse>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let code = svc
        .get_domain_auth_code(&domain)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(AuthCodeResponse {
        domain,
        auth_code: code,
    }))
}
