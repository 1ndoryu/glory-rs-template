use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;
use validator::Validate;

use super::checkout::maybe_backfill_test_hosting_access;
use super::domain::{
    active_custom_domain, build_domain_verification_state, compose_update_from_subscription,
    normalize_domain, sync_custom_domain_route,
};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    sanitize_hosting_event, AssignHostingRequest, CreateHostingRequest, HostingEvent,
    HostingSubscriptionResponse, UpdateHostingRequest, UpdateHostingStatusRequest, UserRole,
};
use crate::repositories::{
    CreateHostingParams, HostingRepository, UpdateHostingParams, UserRepository,
};
use crate::services::HostingRuntimeService;
use crate::AppState;

/// Listar suscripciones de hosting (admin: todas, cliente: las suyas)
#[utoipa::path(
    get,
    path = "/api/hosting/subscriptions",
    responses(
        (status = 200, description = "Lista de suscripciones", body = Vec<HostingSubscriptionResponse>),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<HostingSubscriptionResponse>>, AppError> {
    let subs = HostingRepository::list_all(&state.pool).await?;
    let filtered: Vec<HostingSubscriptionResponse> =
        if auth.effective_role == UserRole::Admin || auth.effective_role == UserRole::Employee {
            subs.into_iter().map(Into::into).collect()
        } else {
            let mut repaired = Vec::new();
            for sub in subs.into_iter().filter(|s| s.user_id == Some(auth.user_id)) {
                repaired.push(
                    maybe_backfill_test_hosting_access(&state, sub, "test_checkout_backfill_list")
                        .await?
                        .into(),
                );
            }
            repaired
        };
    Ok(Json(filtered))
}

/// Crear suscripción de hosting (semi-automático: crea registro, admin provisiona después)
#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions",
    request_body = CreateHostingRequest,
    responses(
        (status = 201, description = "Suscripción creada", body = HostingSubscriptionResponse),
        (status = 400, description = "Datos inválidos"),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn create_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateHostingRequest>,
) -> Result<(StatusCode, Json<HostingSubscriptionResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let plan_config = HostingRepository::get_plan_config(&state.pool, &req.plan)
        .await?
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Plan inválido: {}. Opciones disponibles en /hosting/plan-configs",
                req.plan
            ))
        })?;
    let price = plan_config.monthly_price_cents;
    let storage = plan_config.storage_limit_mb;
    let requested_domain = normalize_domain(req.domain.as_deref());
    let (domain_verification_status, domain_verification_token, domain_verified_at) =
        build_domain_verification_state(requested_domain.as_deref());

    let sub = HostingRepository::create(
        &state.pool,
        CreateHostingParams {
            user_id: Some(auth.user_id),
            client_name: &req.client_name,
            client_email: &req.client_email,
            plan: &req.plan,
            domain: requested_domain.as_deref(),
            domain_verification_status: &domain_verification_status,
            domain_verification_token: domain_verification_token.as_deref(),
            domain_verified_at,
            runtime_kind: if req.coolify_site_name.is_some() {
                "coolify"
            } else {
                HostingRuntimeService::runtime_kind_for_plan(&req.plan).as_str()
            },
            deployment_id: None,
            coolify_site_name: req.coolify_site_name.as_deref(),
            monthly_price_cents: price,
            storage_limit_mb: storage,
        },
    )
    .await?;

    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        sub.id,
        "created",
        Some(serde_json::json!({"plan": req.plan, "by": auth.user_id.to_string()})),
    )
    .await
    {
        tracing::warn!("Error registrando evento created para {}: {e}", sub.id);
    }

    Ok((StatusCode::CREATED, Json(sub.into())))
}

/// Obtener suscripción por ID
#[utoipa::path(
    get,
    path = "/api/hosting/subscriptions/{id}",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Suscripción", body = HostingSubscriptionResponse),
        (status = 404, description = "No encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<HostingSubscriptionResponse>, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if auth.effective_role == UserRole::Client && sub.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden("Sin permisos".into()));
    }

    let sub =
        maybe_backfill_test_hosting_access(&state, sub, "test_checkout_backfill_detail").await?;

    Ok(Json(sub.into()))
}

/// Actualizar status de suscripción (solo admin)
#[utoipa::path(
    patch,
    path = "/api/hosting/subscriptions/{id}/status",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    request_body = UpdateHostingStatusRequest,
    responses(
        (status = 204, description = "Status actualizado"),
        (status = 400, description = "Status inválido"),
        (status = 403, description = "Sin permisos"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn update_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateHostingStatusRequest>,
) -> Result<StatusCode, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let valid_statuses = [
        "pending",
        "provisioning",
        "active",
        "suspended",
        "cancelled",
    ];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(AppError::Validation(format!(
            "Status inválido: {}. Opciones: {}",
            req.status,
            valid_statuses.join(", ")
        )));
    }

    HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    HostingRepository::update_status(&state.pool, id, &req.status).await?;

    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        id,
        "status_change",
        Some(serde_json::json!({
            "new_status": req.status,
            "reason": req.reason,
            "by": auth.user_id.to_string()
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento status_change para {id}: {e}");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Listar eventos de una suscripción
#[utoipa::path(
    get,
    path = "/api/hosting/subscriptions/{id}/events",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Eventos", body = Vec<HostingEvent>),
        (status = 404, description = "Suscripción no encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn list_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<HostingEvent>>, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if auth.effective_role == UserRole::Client && sub.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden("Sin permisos".into()));
    }

    let events = HostingRepository::list_events(&state.pool, id, 100)
        .await?
        .into_iter()
        .map(sanitize_hosting_event)
        .collect();
    Ok(Json(events))
}

/* [074A-65] Actualizar suscripción de hosting (admin: cualquiera, cliente: solo las suyas)
 * [084A-4] Relajado de admin-only a role-aware: cliente valida ownership. */
#[utoipa::path(
    put,
    path = "/api/hosting/subscriptions/{id}",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    request_body = UpdateHostingRequest,
    responses(
        (status = 200, description = "Suscripción actualizada", body = HostingSubscriptionResponse),
        (status = 400, description = "Datos inválidos"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "No encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn update_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateHostingRequest>,
) -> Result<Json<HostingSubscriptionResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if auth.effective_role == UserRole::Client && sub.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden(
            "Sin permisos para editar esta suscripción".into(),
        ));
    }

    let plan_config = HostingRepository::get_plan_config(&state.pool, &req.plan)
        .await?
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Plan inválido: {}. Opciones disponibles en /hosting/public-plans",
                req.plan
            ))
        })?;

    let requested_domain = normalize_domain(req.domain.as_deref());
    let current_domain = normalize_domain(sub.domain.as_deref());
    let domain_changed = requested_domain != current_domain;

    if domain_changed && active_custom_domain(&sub).is_some() {
        if let Some(update) = compose_update_from_subscription(&sub, None, &plan_config) {
            let runtime_kind =
                crate::services::HostingRuntimeKind::from_persisted(&sub.runtime_kind);
            let config = HostingRuntimeService::optional_target_config_for(
                runtime_kind,
                state.coolify_config.as_ref(),
                "retirar dominios custom",
            )?;
            sync_custom_domain_route(&state.http_client, config, runtime_kind, update).await?;
        }
    }

    let (next_domain_verification_status, next_domain_verification_token, next_domain_verified_at) =
        if domain_changed {
            build_domain_verification_state(requested_domain.as_deref())
        } else {
            (
                sub.domain_verification_status.clone(),
                sub.domain_verification_token.clone(),
                sub.domain_verified_at,
            )
        };

    let updated = HostingRepository::update(
        &state.pool,
        id,
        UpdateHostingParams {
            plan: &req.plan,
            domain: requested_domain.as_deref(),
            monthly_price_cents: plan_config.monthly_price_cents,
            storage_limit_mb: plan_config.storage_limit_mb,
            domain_verification_status: &next_domain_verification_status,
            domain_verification_token: next_domain_verification_token.as_deref(),
            domain_verified_at: next_domain_verified_at,
        },
    )
    .await?;

    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        id,
        "updated",
        Some(serde_json::json!({
            "plan": req.plan,
            "domain": requested_domain,
            "domain_verification_status": updated.domain_verification_status,
            "by": auth.user_id.to_string()
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento updated para {id}: {e}");
    }

    Ok(Json(updated.into()))
}

/* [074A-65] Eliminar suscripción de hosting (solo admin) */
#[utoipa::path(
    delete,
    path = "/api/hosting/subscriptions/{id}",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 204, description = "Eliminada"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "No encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn delete_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if let (Some(deployment_id), Some(ref coolify_config)) =
        (sub.deployment_id_or_legacy(), &state.coolify_config)
    {
        let runtime_kind = crate::services::HostingRuntimeKind::from_persisted(&sub.runtime_kind);
        if let Err(e) = HostingRuntimeService::delete_deployment(
            &state.http_client,
            Some(coolify_config),
            Some(runtime_kind),
            deployment_id,
            true,
        )
        .await
        {
            tracing::warn!(
                "Error eliminando despliegue {} para suscripción {id}: {e}",
                deployment_id
            );
        }
    }

    HostingRepository::delete(&state.pool, id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/* [084A-4] Solicitar cancelación de hosting (cliente o admin).
 * El cliente puede cancelar sus propias suscripciones activas.
 * No se usa DELETE directamente para clientes; cambia el status a cancelled
 * y registra un evento, preservando el registro para auditoría. */
#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/cancel",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 204, description = "Cancelación procesada"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "No encontrada"),
        (status = 409, description = "Ya cancelada"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn request_cancel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if auth.effective_role == UserRole::Client && sub.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden(
            "Sin permisos para cancelar esta suscripción".into(),
        ));
    }

    if sub.status == "cancelled" {
        return Err(AppError::Conflict(
            "La suscripción ya está cancelada".into(),
        ));
    }

    HostingRepository::update_status(&state.pool, id, "cancelled").await?;

    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        id,
        "status_change",
        Some(serde_json::json!({
            "new_status": "cancelled",
            "reason": "Cancelación solicitada por el usuario",
            "by": auth.user_id.to_string()
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento cancellation para {id}: {e}");
    }

    Ok(StatusCode::NO_CONTENT)
}

/* [304A-3] Asigna una suscripción de hosting a un usuario por email.
 * Admin only. Permite vincular hostings creados manualmente a cuentas de clientes existentes.
 * Gotcha: el usuario debe existir en BD; no crea cuentas nuevas. */
#[utoipa::path(
    patch,
    path = "/api/hosting/subscriptions/{id}/assign",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    request_body = AssignHostingRequest,
    responses(
        (status = 200, description = "Suscripción asignada al usuario", body = HostingSubscriptionResponse),
        (status = 403, description = "Solo admin"),
        (status = 404, description = "Suscripción o usuario no encontrado"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn assign_hosting(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignHostingRequest>,
) -> Result<Json<HostingSubscriptionResponse>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    let user = UserRepository::find_by_email(&state.pool, &req.user_email)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Usuario con email '{}' no encontrado. Debe tener cuenta registrada.",
                req.user_email
            ))
        })?;

    let updated = HostingRepository::assign_user(&state.pool, id, Some(user.id)).await?;

    if let Err(e) = HostingRepository::add_event(
        &state.pool,
        id,
        "assigned",
        Some(serde_json::json!({
            "assigned_to": user.id.to_string(),
            "user_email": req.user_email,
            "by": auth.user_id.to_string(),
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento assigned para {id}: {e}");
    }

    Ok(Json(updated.into()))
}
