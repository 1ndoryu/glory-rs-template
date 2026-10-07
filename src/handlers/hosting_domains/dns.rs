/* [01AA-4-f3p] Zonas y registros DNS admin (extraido de hosting_domains.rs). */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use super::helpers::{contabo, require_admin};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::contabo_domains::{
    CreateDnsRecordRequest, DnsRecord, DnsZone, UpdateDnsRecordRequest,
};
use crate::AppState;

pub(super) async fn list_dns_zones(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<DnsZone>>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let zones = svc.list_dns_zones().await.map_err(AppError::Internal)?;
    Ok(Json(zones))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(super) struct CreateDnsZoneBody {
    pub zone_name: String,
}

pub(super) async fn create_dns_zone(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateDnsZoneBody>,
) -> Result<(StatusCode, Json<DnsZone>), AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let zone = svc
        .create_dns_zone(&body.zone_name)
        .await
        .map_err(AppError::Internal)?;
    Ok((StatusCode::CREATED, Json(zone)))
}

pub(super) async fn delete_dns_zone(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(zone): Path<String>,
) -> Result<StatusCode, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    svc.delete_dns_zone(&zone)
        .await
        .map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn list_dns_records(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(zone): Path<String>,
) -> Result<Json<Vec<DnsRecord>>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let records = svc
        .list_dns_records(&zone)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(records))
}

pub(super) async fn create_dns_record(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(zone): Path<String>,
    Json(body): Json<CreateDnsRecordRequest>,
) -> Result<(StatusCode, Json<DnsRecord>), AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let record = svc
        .create_dns_record(&zone, &body)
        .await
        .map_err(AppError::Internal)?;
    Ok((StatusCode::CREATED, Json(record)))
}

pub(super) async fn update_dns_record(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((zone, record_id)): Path<(String, i64)>,
    Json(body): Json<UpdateDnsRecordRequest>,
) -> Result<Json<DnsRecord>, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    let record = svc
        .update_dns_record(&zone, record_id, &body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(record))
}

pub(super) async fn delete_dns_record(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((zone, record_id)): Path<(String, i64)>,
) -> Result<StatusCode, AppError> {
    require_admin(&auth)?;
    let svc = contabo(&state)?;
    svc.delete_dns_record(&zone, record_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}
