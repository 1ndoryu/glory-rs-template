use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use rand::Rng;
use uuid::Uuid;

use super::domain::{
    activate_domain_route, domain_ready_for_route, DomainActivation, DOMAIN_STATUS_ACTIVE,
    DOMAIN_STATUS_VERIFIED,
};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{HostingSubscriptionResponse, UserRole};
use crate::repositories::{
    HostingRepository, HostingResourceAllocation, InfrastructureRepository, ServerInfo,
};
use crate::services::{
    HostingRuntimeProvisionResult, HostingRuntimeService, HostingRuntimeUpdate,
    HostingStripeService,
};
use crate::AppState;

/* [250A-1] Extraído de provision_subscription() para cumplir límite de 100 líneas.
 * Valida estado, resuelve runtime/config, reserva capacidad. */
struct ProvisionPrep {
    sub: crate::models::HostingSubscription,
    runtime_kind: crate::services::HostingRuntimeKind,
    config: Option<crate::services::coolify::CoolifyConfig>,
    service_name: String,
    sftp_port: i32,
    plan_config: crate::models::HostingPlanConfig,
    allocation: Option<HostingResourceAllocation>,
}

async fn prepare_provision(
    pool: &sqlx::PgPool,
    state: &AppState,
    id: Uuid,
) -> Result<ProvisionPrep, AppError> {
    let sub = HostingRepository::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Suscripción {id} no encontrada")))?;

    if sub.status != "pending" && sub.status != "provisioning" {
        return Err(AppError::Validation(format!(
            "Solo se puede provisionar hostings en estado 'pending' o 'provisioning', actual: '{}'",
            sub.status
        )));
    }

    let runtime_kind = crate::services::HostingRuntimeKind::from_persisted(&sub.runtime_kind);
    let config_ref = HostingRuntimeService::optional_target_config_for(
        runtime_kind,
        state.coolify_config.as_ref(),
        "provisionar hostings",
    )?;

    HostingRepository::update_status(pool, id, "provisioning").await?;

    let service_name = HostingRuntimeService::deployment_name_for(&id);
    let sftp_port = HostingRepository::find_available_sftp_port(pool).await?;
    let plan_config = HostingRepository::get_plan_config(pool, &sub.plan)
        .await?
        .ok_or_else(|| {
            AppError::Internal(format!("Plan config '{}' no encontrado en BD", sub.plan))
        })?;
    let allocation = if config_ref.is_some() {
        Some(InfrastructureRepository::hosting_allocation_for_plan(pool, &sub.plan).await?)
    } else {
        None
    };
    if let (Some(config), Some(allocation)) = (config_ref, allocation.as_ref()) {
        let capacity_reserved = InfrastructureRepository::reserve_capacity_if_known(
            pool,
            &config.server_uuid,
            allocation,
        )
        .await?;
        if !capacity_reserved {
            HostingRepository::update_status(pool, id, "pending")
                .await
                .ok();
            return Err(AppError::Validation(
                "La VPS no tiene capacidad suficiente para provisionar este plan".into(),
            ));
        }
    }
    /* [250A-1] Re-obtener config como owned para almacenar en struct (el
     * borrow anterior se consumió en el capacity check). */
    let config_owned = HostingRuntimeService::optional_target_config_for(
        runtime_kind,
        state.coolify_config.as_ref(),
        "provisionar hostings",
    )?
    .cloned();

    Ok(ProvisionPrep {
        sub,
        runtime_kind,
        config: config_owned,
        service_name,
        sftp_port,
        plan_config,
        allocation,
    })
}

/* [250A-1] Extraído de provision_subscription() para cumplir límite de 100 líneas.
 * Ejecuta el provisioning, maneja rollback de capacidad en error. */
async fn execute_provision(
    state: &AppState,
    pool: &sqlx::PgPool,
    prep: &ProvisionPrep,
    id: Uuid,
) -> Result<HostingRuntimeProvisionResult, AppError> {
    let provision_preferences = HostingStripeService::load_provision_preferences(pool, id).await;
    let result = match HostingRuntimeService::provision_hosting(
        &state.http_client,
        prep.config.as_ref(),
        Some(prep.runtime_kind),
        &prep.service_name,
        prep.sftp_port,
        &prep.plan_config,
        &prep.sub.client_name,
        &prep.sub.client_email,
        provision_preferences.as_ref(),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            tracing::error!("[Provision] Falló para {id}: {error}");
            if let (Some(config), Some(allocation)) =
                (prep.config.as_ref(), prep.allocation.as_ref())
            {
                InfrastructureRepository::release_capacity(
                    &state.pool,
                    &config.server_uuid,
                    allocation,
                )
                .await
                .ok();
            }
            HostingRepository::update_status(&state.pool, id, "pending")
                .await
                .ok();
            return Err(error);
        }
    };
    Ok(result)
}

/// Provisionar un hosting: crea el despliegue Nginx/WordPress en el runtime persistido y actualiza la suscripción.
/// Solo admin. La suscripción debe estar en estado "pending" o "provisioning".
#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/provision",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Hosting provisionado", body = HostingSubscriptionResponse),
        (status = 400, description = "Estado inválido para provisioning"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
        (status = 503, description = "Coolify no configurado"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
#[allow(clippy::too_many_lines)]
pub(super) async fn provision_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<HostingSubscriptionResponse>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let prep = prepare_provision(&state.pool, &state, id).await?;
    let result = execute_provision(&state, &state.pool, &prep, id).await?;

    HostingRepository::update_server_info(
        &state.pool,
        id,
        &ServerInfo {
            runtime_kind: result.runtime_kind.as_str(),
            deployment_id: &result.deployment_id,
            coolify_site_name: &prep.service_name,
            server_uuid: &result.deployment_id,
            server_ip: &result.server_ip,
            sftp_user: &result.access_user,
            sftp_password: &result.access_password,
            sftp_port: result.access_port,
        },
    )
    .await?;

    if let Some(custom_domain) = domain_ready_for_route(&prep.sub) {
        let _ = activate_domain_route(
            &state,
            DomainActivation {
                subscription_id: id,
                runtime_kind: result.runtime_kind,
                token: prep.sub.domain_verification_token.as_deref(),
                verified_at: prep.sub.domain_verified_at,
                update: HostingRuntimeUpdate {
                    deployment_id: &result.deployment_id,
                    deployment_name: &prep.service_name,
                    custom_domain: Some(custom_domain),
                    access_user: &result.access_user,
                    access_password: &result.access_password,
                    access_port: result.access_port,
                    plan_config: &prep.plan_config,
                },
            },
        )
        .await;
    }

    HostingRepository::update_status(&state.pool, id, "active").await?;

    record_provisioned_event(&state, id, auth.user_id, &prep.service_name, &result).await;

    let updated = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("Suscripción perdida tras provisioning".into()))?;

    Ok(Json(updated.into()))
}

async fn record_provisioned_event(
    state: &AppState,
    id: Uuid,
    actor_id: Uuid,
    service_name: &str,
    result: &HostingRuntimeProvisionResult,
) {
    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        id,
        "provisioned",
        Some(serde_json::json!({
            "runtime_kind": result.runtime_kind.as_str(),
            "deployment_id": result.deployment_id,
            "public_url": result.public_url,
            "server_ip": result.server_ip,
            "service_name": service_name,
            "wordpress_ready": result.wordpress_ready,
            "wordpress_install_error": result.wordpress_install_error,
            "by": actor_id.to_string(),
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento provisioned para {id}: {e}");
    }
}

/* [114A-1] Rotación de credenciales SFTP: genera nueva contraseña, actualiza BD y
 * compose en Coolify, reinicia servicio SSH para que tome efecto. */
#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/rotate-credentials",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Credenciales rotadas"),
        (status = 400, description = "Hosting no provisionado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "No encontrada"),
        (status = 503, description = "Coolify no configurado"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn rotate_credentials(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    let deployment_id = sub.deployment_id_or_legacy().ok_or_else(|| {
        AppError::Validation("Hosting no provisionado — no se pueden rotar credenciales".into())
    })?;
    let sftp_user = sub.sftp_user.as_ref().ok_or_else(|| {
        AppError::Internal("SFTP user ausente en suscripción provisionada".into())
    })?;
    let sftp_port = sub.sftp_port.ok_or_else(|| {
        AppError::Internal("SFTP port ausente en suscripción provisionada".into())
    })?;
    let service_name = sub.coolify_site_name.as_ref().ok_or_else(|| {
        AppError::Internal("Nombre de servicio Coolify ausente en suscripción provisionada".into())
    })?;

    let runtime_kind = crate::services::HostingRuntimeKind::from_persisted(&sub.runtime_kind);
    let config = HostingRuntimeService::optional_target_config_for(
        runtime_kind,
        state.coolify_config.as_ref(),
        "rotar credenciales",
    )?;

    let new_password: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();

    HostingRepository::update_sftp_password(&state.pool, id, &new_password).await?;

    let plan_config = HostingRepository::get_plan_config(&state.pool, &sub.plan)
        .await?
        .ok_or_else(|| {
            AppError::Internal(format!("Plan config '{}' no encontrado en BD", sub.plan))
        })?;
    HostingRuntimeService::update_deployment(
        &state.http_client,
        config,
        Some(runtime_kind),
        HostingRuntimeUpdate {
            deployment_id,
            deployment_name: service_name,
            custom_domain: domain_ready_for_route(&sub),
            access_user: sftp_user,
            access_password: &new_password,
            access_port: sftp_port,
            plan_config: &plan_config,
        },
    )
    .await?;

    mark_verified_domain_active(&state, id, &sub).await;

    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        id,
        "credentials_rotated",
        Some(serde_json::json!({"by": auth.user_id.to_string()})),
    )
    .await
    {
        tracing::warn!("Error registrando evento credentials_rotated para {id}: {e}");
    }

    Ok(Json(serde_json::json!({
        "sftp_user": sftp_user,
        "sftp_password": new_password,
        "sftp_port": sftp_port,
    })))
}

/* [114A-4] Refresh: regenera compose con plan config actual y redeploya en Coolify.
 * Usado para migrar hostings existentes cuando se cambian limits/features del plan. */
#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/refresh",
    params(("id" = Uuid, Path, description = "ID suscripción")),
    responses(
        (status = 200, description = "Hosting redeployado con config actual"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
        (status = 503, description = "Coolify no configurado"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn refresh_hosting(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    let deployment_id = sub.deployment_id_or_legacy().ok_or_else(|| {
        AppError::Validation("Hosting no provisionado — no se puede refrescar".into())
    })?;
    let sftp_user = sub.sftp_user.as_ref().ok_or_else(|| {
        AppError::Internal("SFTP user ausente en suscripción provisionada".into())
    })?;
    let sftp_password = sub.sftp_password.as_ref().ok_or_else(|| {
        AppError::Internal("SFTP password ausente en suscripción provisionada".into())
    })?;
    let sftp_port = sub.sftp_port.ok_or_else(|| {
        AppError::Internal("SFTP port ausente en suscripción provisionada".into())
    })?;
    let service_name = sub.coolify_site_name.as_ref().ok_or_else(|| {
        AppError::Internal("Nombre de servicio Coolify ausente en suscripción provisionada".into())
    })?;

    let runtime_kind = crate::services::HostingRuntimeKind::from_persisted(&sub.runtime_kind);
    let config = HostingRuntimeService::optional_target_config_for(
        runtime_kind,
        state.coolify_config.as_ref(),
        "refrescar hostings",
    )?;

    let plan_config = HostingRepository::get_plan_config(&state.pool, &sub.plan)
        .await?
        .ok_or_else(|| {
            AppError::Internal(format!("Plan config '{}' no encontrado en BD", sub.plan))
        })?;
    HostingRuntimeService::update_deployment(
        &state.http_client,
        config,
        Some(runtime_kind),
        HostingRuntimeUpdate {
            deployment_id,
            deployment_name: service_name,
            custom_domain: domain_ready_for_route(&sub),
            access_user: sftp_user,
            access_password: sftp_password,
            access_port: sftp_port,
            plan_config: &plan_config,
        },
    )
    .await?;

    mark_verified_domain_active(&state, id, &sub).await;

    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        id,
        "refreshed",
        Some(serde_json::json!({"by": auth.user_id.to_string(), "plan": sub.plan})),
    )
    .await
    {
        tracing::warn!("Error registrando evento refreshed para {id}: {e}");
    }

    Ok(Json(serde_json::json!({
        "message": "Hosting redeployado con configuración actual",
        "plan": sub.plan,
    })))
}

async fn mark_verified_domain_active(
    state: &AppState,
    id: Uuid,
    sub: &crate::models::HostingSubscription,
) {
    if sub.domain_verification_status != DOMAIN_STATUS_VERIFIED {
        return;
    }

    let _ = HostingRepository::update_domain_verification(
        &state.pool,
        id,
        DOMAIN_STATUS_ACTIVE,
        sub.domain_verification_token.as_deref(),
        Some(sub.domain_verified_at.unwrap_or_else(Utc::now)),
    )
    .await;
}
