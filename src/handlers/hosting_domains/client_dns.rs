/* [01AA-4-f3p] DNS de cliente por suscripción (extraido de hosting_domains.rs).
 * El cliente solo gestiona registros del dominio ligado a su suscripción;
 * la zona se deduce del dominio de la suscripción. */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use super::helpers::contabo;
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::repositories::HostingRepository;
use crate::services::contabo_domains::{CreateDnsRecordRequest, DnsRecord, UpdateDnsRecordRequest};
use crate::AppState;

async fn client_zone(state: &AppState, auth: &AuthUser, sub_id: Uuid) -> Result<String, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, sub_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Suscripción no encontrada".into()))?;

    let is_owner = sub.user_id == Some(auth.user_id);
    let is_admin = auth.effective_role == UserRole::Admin;
    if !is_owner && !is_admin {
        return Err(AppError::Forbidden(
            "No tienes acceso a esta suscripción".into(),
        ));
    }

    sub.domain
        .filter(|d| !d.is_empty())
        .ok_or_else(|| AppError::BadRequest("La suscripción no tiene dominio configurado".into()))
}

pub(super) async fn client_list_dns_records(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(sub_id): Path<Uuid>,
) -> Result<Json<Vec<DnsRecord>>, AppError> {
    let zone = client_zone(&state, &auth, sub_id).await?;
    let svc = contabo(&state)?;
    let records = svc
        .list_dns_records(&zone)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(records))
}

pub(super) async fn client_create_dns_record(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(sub_id): Path<Uuid>,
    Json(body): Json<CreateDnsRecordRequest>,
) -> Result<(StatusCode, Json<DnsRecord>), AppError> {
    let zone = client_zone(&state, &auth, sub_id).await?;
    let svc = contabo(&state)?;
    let record = svc
        .create_dns_record(&zone, &body)
        .await
        .map_err(AppError::Internal)?;
    Ok((StatusCode::CREATED, Json(record)))
}

pub(super) async fn client_update_dns_record(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((sub_id, record_id)): Path<(Uuid, i64)>,
    Json(body): Json<UpdateDnsRecordRequest>,
) -> Result<Json<DnsRecord>, AppError> {
    let zone = client_zone(&state, &auth, sub_id).await?;
    let svc = contabo(&state)?;
    let record = svc
        .update_dns_record(&zone, record_id, &body)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(record))
}

pub(super) async fn client_delete_dns_record(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((sub_id, record_id)): Path<(Uuid, i64)>,
) -> Result<StatusCode, AppError> {
    let zone = client_zone(&state, &auth, sub_id).await?;
    let svc = contabo(&state)?;
    svc.delete_dns_record(&zone, record_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}
