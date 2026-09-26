use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{HostingSubscription, UserRole};
use crate::repositories::HostingRepository;
use crate::services::{HostingRuntimeKind, HostingRuntimeService};
use crate::AppState;

/* [265A-6] Reutiliza resolve_ssh_key de stats para resolver la SSH key
 * correcta según el server_ip de la suscripción (multi-VPS). */
fn resolve_ssh_key_for_sub<'a>(state: &'a AppState, server_ip: &str) -> Option<&'a str> {
    crate::handlers::hosting::stats::resolve_ssh_key(state, server_ip)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateBackupRequest {
    pub tier: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RestoreBackupRequest {
    pub backup_id: String,
    pub access_password: Option<String>,
    pub skip_safety_snapshot: Option<bool>,
}

fn ensure_subscription_access(
    auth: &AuthUser,
    subscription: &HostingSubscription,
) -> Result<(), AppError> {
    if auth.effective_role == UserRole::Client && subscription.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden(
            "Sin permisos para ver los backups de esta suscripción".into(),
        ));
    }
    Ok(())
}

/* [245A-9] Backup/restore lightweight vive por suscripcion, no por panel suelto.
 * Si un restore regenera la password SFTP y este boundary no la persiste, el
 * runtime se recupera pero el panel queda mostrando credenciales muertas. */

#[utoipa::path(
    get,
    path = "/api/hosting/subscriptions/{id}/backups",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Backups disponibles para la suscripción"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
        (status = 503, description = "Runtime sin soporte de backups"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn list_backups(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let subscription = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;
    ensure_subscription_access(&auth, &subscription)?;

    let deployment_id = subscription.deployment_id_or_legacy().ok_or_else(|| {
        AppError::Validation("La suscripción aún no tiene un deployment provisionado".into())
    })?;
    let runtime_kind = HostingRuntimeKind::from_persisted(&subscription.runtime_kind);
    /* [265A-6] Para Coolify se necesita server_ip + ssh_key_path para
     * listar backups vía SSH (lectura del volumen Docker). */
    let (server_ip, ssh_key_path) = match runtime_kind {
        HostingRuntimeKind::Coolify => {
            let ip = subscription.server_ip.as_deref().ok_or_else(|| {
                AppError::Validation("Suscripción Coolify sin server_ip configurado".into())
            })?;
            let key = resolve_ssh_key_for_sub(&state, ip).ok_or_else(|| {
                AppError::Internal(
                    "SSH key no disponible para el servidor de esta suscripción".into(),
                )
            })?;
            (Some(ip), Some(key))
        }
        HostingRuntimeKind::Lightweight => (None, None),
    };
    let entries = HostingRuntimeService::list_backups(
        Some(runtime_kind),
        deployment_id,
        server_ip,
        ssh_key_path,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "runtime_kind": runtime_kind.as_str(),
        "deployment_id": deployment_id,
        "data": entries,
    })))
}

#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/backups",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Backup creado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
        (status = 503, description = "Runtime sin soporte de backups"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn create_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<CreateBackupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let subscription = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;
    let deployment_id = subscription.deployment_id_or_legacy().ok_or_else(|| {
        AppError::Validation("La suscripción aún no tiene un deployment provisionado".into())
    })?;
    let runtime_kind = HostingRuntimeKind::from_persisted(&subscription.runtime_kind);
    let tier = request.tier.as_deref().unwrap_or("manual");
    /* [265A-6] Para Coolify se necesita server_ip + ssh_key_path */
    let (server_ip, ssh_key_path) = match runtime_kind {
        HostingRuntimeKind::Coolify => {
            let ip = subscription.server_ip.as_deref().ok_or_else(|| {
                AppError::Validation("Suscripción Coolify sin server_ip configurado".into())
            })?;
            let key = resolve_ssh_key_for_sub(&state, ip).ok_or_else(|| {
                AppError::Internal(
                    "SSH key no disponible para el servidor de esta suscripción".into(),
                )
            })?;
            (Some(ip), Some(key))
        }
        HostingRuntimeKind::Lightweight => (None, None),
    };
    let report = HostingRuntimeService::create_backup(
        Some(runtime_kind),
        deployment_id,
        tier,
        request.label.as_deref(),
        server_ip,
        ssh_key_path,
    )
    .await?;

    if let Err(error) = HostingRepository::add_event(
        &state.pool,
        id,
        "backup_created",
        Some(serde_json::json!({
            "runtime_kind": runtime_kind.as_str(),
            "deployment_id": deployment_id,
            "backup_id": &report.backup_id,
            "tier": &report.tier,
            "status": &report.status,
            "notes": &report.notes,
            "by": auth.user_id.to_string(),
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento backup_created para {id}: {error}");
    }

    Ok(Json(serde_json::json!({
        "runtime_kind": runtime_kind.as_str(),
        "deployment_id": deployment_id,
        "data": report,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteBackupRequest {
    pub file_name: String,
}

/* [265A-6] Eliminar un archivo de backup específico.
 * Solo disponible para administradores.
 * Para Coolify: elimina el archivo del volumen Docker vía SSH.
 * Para Lightweight: usa coolify-manager-rs light-backup --delete. */
#[utoipa::path(
    delete,
    path = "/api/hosting/subscriptions/{id}/backups",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Backup eliminado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
        (status = 503, description = "Runtime sin soporte de eliminación de backups"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn delete_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<DeleteBackupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    let subscription = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;
    let deployment_id = subscription.deployment_id_or_legacy().ok_or_else(|| {
        AppError::Validation("La suscripción aún no tiene un deployment provisionado".into())
    })?;
    let runtime_kind = HostingRuntimeKind::from_persisted(&subscription.runtime_kind);

    match runtime_kind {
        HostingRuntimeKind::Coolify => {
            let server_ip = subscription.server_ip.as_deref().ok_or_else(|| {
                AppError::Validation("Suscripción Coolify sin server_ip configurado".into())
            })?;
            let ssh_key_path = resolve_ssh_key_for_sub(&state, server_ip).ok_or_else(|| {
                AppError::Internal(
                    "SSH key no disponible para el servidor de esta suscripción".into(),
                )
            })?;
            HostingRuntimeService::delete_coolify_backup_via_ssh(
                server_ip,
                ssh_key_path,
                deployment_id,
                &request.file_name,
            )
            .await?;
        }
        HostingRuntimeKind::Lightweight => {
            return Err(AppError::BadRequest(
                "Eliminar backups no está disponible para runtime Lightweight".into(),
            ));
        }
    }

    if let Err(error) = HostingRepository::add_event(
        &state.pool,
        id,
        "backup_deleted",
        Some(serde_json::json!({
            "runtime_kind": runtime_kind.as_str(),
            "deployment_id": deployment_id,
            "file_name": &request.file_name,
            "by": auth.user_id.to_string(),
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento backup_deleted para {id}: {error}");
    }

    Ok(Json(serde_json::json!({
        "runtime_kind": runtime_kind.as_str(),
        "deployment_id": deployment_id,
        "deleted": request.file_name,
    })))
}

#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/restore",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Backup restaurado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
        (status = 503, description = "Runtime sin soporte de restore"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn restore_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<RestoreBackupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let subscription = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;
    let deployment_id = subscription.deployment_id_or_legacy().ok_or_else(|| {
        AppError::Validation("La suscripción aún no tiene un deployment provisionado".into())
    })?;
    let runtime_kind = HostingRuntimeKind::from_persisted(&subscription.runtime_kind);
    /* [265A-6] Para Coolify se necesita server_ip + ssh_key_path */
    let (server_ip, ssh_key_path) = match runtime_kind {
        HostingRuntimeKind::Coolify => {
            let ip = subscription.server_ip.as_deref().ok_or_else(|| {
                AppError::Validation("Suscripción Coolify sin server_ip configurado".into())
            })?;
            let key = resolve_ssh_key_for_sub(&state, ip).ok_or_else(|| {
                AppError::Internal(
                    "SSH key no disponible para el servidor de esta suscripción".into(),
                )
            })?;
            (Some(ip), Some(key))
        }
        HostingRuntimeKind::Lightweight => (None, None),
    };
    let report = HostingRuntimeService::restore_backup(
        Some(runtime_kind),
        deployment_id,
        &request.backup_id,
        request.access_password.as_deref(),
        request.skip_safety_snapshot.unwrap_or(false),
        server_ip,
        ssh_key_path,
    )
    .await?;

    if let Some(access_password) = report.access_password.as_deref() {
        HostingRepository::update_sftp_password(&state.pool, id, access_password).await?;
    }

    if let Err(error) = HostingRepository::add_event(
        &state.pool,
        id,
        "backup_restored",
        Some(serde_json::json!({
            "runtime_kind": runtime_kind.as_str(),
            "deployment_id": deployment_id,
            "backup_id": &report.backup_id,
            "status": &report.status,
            "fqdn": &report.fqdn,
            "access_user": &report.access_user,
            "password_rotated": report.access_password.is_some(),
            "notes": &report.notes,
            "by": auth.user_id.to_string(),
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento backup_restored para {id}: {error}");
    }

    Ok(Json(serde_json::json!({
        "runtime_kind": runtime_kind.as_str(),
        "deployment_id": deployment_id,
        "data": report,
    })))
}
