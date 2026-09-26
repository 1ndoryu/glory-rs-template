/* sentinel-disable-file limite-lineas: controlador REST de VPS con catálogo, checkout,
 * aprobación y provisioning en el mismo módulo legacy; extraer por subdominio queda como mejora.
 */
/* [164A-17] Handlers de reventa VPS.
 * Separados de hosting compartido para no mezclar inventario bruto de Contabo con ventas reales.
 * El flujo es: pending_payment -> pending_approval -> provisioning -> active / rejected. */

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    PublicVpsPlan, RejectVpsRequest, SelfSubscribeVpsRequest, SelfSubscribeVpsResponse, UserRole,
    VpsPlanConfig, VpsSubscription, VpsSubscriptionResponse,
};
use crate::repositories::{CreateVpsSubscriptionParams, UserRepository, VpsRepository};
use crate::services::{
    is_checkout_bypass_email, vps_stripe_fee_cents, CreateInstanceParams, EmailService,
    VpsCheckoutParams, VpsStripeService,
};
use crate::AppState;

fn resolve_public_base_url(headers: &HeaderMap) -> String {
    if let Some(origin) = headers.get("origin").and_then(|value| value.to_str().ok()) {
        let trimmed = origin.trim_end_matches('/');
        if !trimmed.is_empty() && !trimmed.contains("localhost") {
            return trimmed.to_string();
        }
    }

    if let Some(referer) = headers.get("referer").and_then(|value| value.to_str().ok()) {
        if let Some(scheme_index) = referer.find("://") {
            let after_scheme = &referer[scheme_index + 3..];
            let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
            let base_url = &referer[..scheme_index + 3 + host_end];
            if !base_url.contains("localhost") {
                return base_url.to_string();
            }
        }
    }

    if let Ok(env_url) = std::env::var("GLORY_PUBLIC_URL") {
        return env_url.trim_end_matches('/').to_string();
    }

    "http://localhost:5173".to_string()
}

fn vps_plan_features(config: &VpsPlanConfig) -> Vec<String> {
    let ram_gb = config.ram_mb / 1024;
    let storage_summary = if config.storage_options.is_empty() {
        format!("{} GB {}", config.disk_mb / 1024, config.storage_type)
    } else {
        config.storage_options.join(" o ")
    };
    vec![
        format!("{} vCPU dedicados", config.cpu_cores),
        format!("{} GB RAM", ram_gb),
        storage_summary,
        format!("Puerto de {} Mbit/s", config.port_speed_mbps),
        config.bandwidth_label.clone(),
        format!(
            "{} snapshot{} incluido{}",
            config.snapshot_count,
            if config.snapshot_count == 1 { "" } else { "s" },
            if config.snapshot_count == 1 { "" } else { "s" }
        ),
        "Acceso root y SSH".to_string(),
        "Hostname y MOTD white-label".to_string(),
        "Entrega verificada antes del provisioning".to_string(),
    ]
}

fn public_plan_from_config(config: VpsPlanConfig) -> PublicVpsPlan {
    let features = vps_plan_features(&config);
    PublicVpsPlan {
        tier_name: config.tier_name,
        display_name: config.display_name,
        description: config.description,
        monthly_price_cents: config.monthly_price_cents,
        setup_fee_cents: config.setup_fee_cents,
        cpu_cores: config.cpu_cores,
        ram_mb: config.ram_mb,
        disk_mb: config.disk_mb,
        storage_type: config.storage_type,
        storage_options: config.storage_options,
        port_speed_mbps: config.port_speed_mbps,
        bandwidth_label: config.bandwidth_label,
        snapshot_count: config.snapshot_count,
        region: config.region,
        region_extra_cents: config.region_extra_cents,
        storage_extra_cents: config.storage_extra_cents,
        features,
        approval_required: config.approval_required,
        recommended: false,
    }
}

fn build_client_notes(
    storage: Option<&str>,
    region: Option<&str>,
    os: Option<&str>,
    server_password: Option<&str>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = storage {
        if !s.trim().is_empty() {
            parts.push(format!("Storage: {s}"));
        }
    }
    if let Some(r) = region {
        if !r.trim().is_empty() {
            parts.push(format!("Región: {r}"));
        }
    }
    if let Some(o) = os {
        if !o.trim().is_empty() {
            parts.push(format!("OS: {o}"));
        }
    }
    if let Some(p) = server_password {
        if !p.trim().is_empty() {
            parts.push(format!("Password: {p}"));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn selected_extra_cents(
    options: &serde_json::Value,
    selected: Option<&str>,
) -> Result<i32, AppError> {
    let amount = selected
        .and_then(|key| options.get(key))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);

    i32::try_from(amount)
        .map_err(|_| AppError::Validation("Extra de precio VPS fuera de rango".into()))
}

async fn complete_test_bypass_checkout(
    pool: &PgPool,
    subscription: VpsSubscription,
    tier: &str,
    user_id: Uuid,
    base_url: &str,
) -> Result<SelfSubscribeVpsResponse, AppError> {
    VpsRepository::update_status(pool, subscription.id, "pending_approval").await?;
    let _ = VpsRepository::add_event(
        pool,
        subscription.id,
        "test_checkout_bypassed",
        Some(serde_json::json!({
            "tier": tier,
            "source": "self-service",
            "by": user_id.to_string(),
        })),
    )
    .await;

    let updated = VpsRepository::find_by_id(pool, subscription.id)
        .await?
        .unwrap_or(subscription);
    let checkout_url = format!(
        "{base_url}/panel?vps=test-bypass&subscription_id={}",
        updated.id
    );

    Ok(SelfSubscribeVpsResponse {
        subscription: updated.into(),
        checkout_url,
    })
}

fn sanitize_hostname(requested_hostname: Option<&str>, subscription_id: Uuid) -> String {
    let fallback = format!("nakomi-vps-{}", &subscription_id.to_string()[..8]);
    let Some(raw_value) = requested_hostname else {
        return fallback;
    };

    let sanitized = raw_value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if sanitized.is_empty() {
        fallback
    } else {
        sanitized.chars().take(63).collect()
    }
}

fn generate_initial_password() -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

fn build_cloud_init(hostname: &str) -> String {
    format!(
        r#"#cloud-config
package_update: true
package_upgrade: true
packages:
  - docker.io
  - ufw
  - fail2ban
write_files:
  - path: /etc/motd
    permissions: '0644'
    content: |
      Bienvenido a Nakomi Studio
      Este VPS fue aprovisionado como infraestructura dedicada de cliente.
runcmd:
  - hostnamectl set-hostname {hostname}
  - systemctl enable docker
  - systemctl start docker
  - ufw allow OpenSSH
  - ufw allow 80/tcp
  - ufw allow 443/tcp
  - ufw --force enable
final_message: "Bootstrap Nakomi completado para {hostname}."
"#
    )
}

fn map_contabo_error(message: &str) -> AppError {
    let lower = message.to_ascii_lowercase();
    tracing::warn!("Contabo VPS flow failed: {message}");

    if lower.contains("invalid_grant") || lower.contains("unauthorized") {
        return AppError::ServiceUnavailable(
            "Contabo rechazó la autenticación del flujo VPS. Revisa las credenciales OAuth2."
                .into(),
        );
    }

    AppError::ServiceUnavailable(
        "No se pudo completar la operación con Contabo. Revisa la configuración del proveedor."
            .into(),
    )
}

#[utoipa::path(
    get,
    path = "/api/vps/public-plans",
    responses(
        (status = 200, description = "Catálogo público de VPS", body = Vec<PublicVpsPlan>),
    ),
    tag = "vps"
)]
pub async fn list_public_plans(
    State(state): State<AppState>,
) -> Result<Json<Vec<PublicVpsPlan>>, AppError> {
    let configs = VpsRepository::list_plan_configs(&state.pool).await?;
    Ok(Json(
        configs.into_iter().map(public_plan_from_config).collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/vps/subscriptions",
    responses(
        (status = 200, description = "Lista de suscripciones VPS", body = Vec<VpsSubscriptionResponse>),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<VpsSubscriptionResponse>>, AppError> {
    let subscriptions =
        if auth.effective_role == UserRole::Admin || auth.effective_role == UserRole::Employee {
            VpsRepository::list_all(&state.pool).await?
        } else {
            VpsRepository::list_by_user_id(&state.pool, auth.user_id).await?
        };

    Ok(Json(subscriptions.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/api/vps/subscriptions/{id}",
    params(("id" = Uuid, Path, description = "ID de la suscripción VPS")),
    responses(
        (status = 200, description = "Suscripción VPS", body = VpsSubscriptionResponse),
        (status = 404, description = "No encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<VpsSubscriptionResponse>, AppError> {
    let subscription = VpsRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción VPS no encontrada".into()))?;

    if auth.effective_role == UserRole::Client && subscription.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden(
            "Sin permisos para ver esta suscripción VPS".into(),
        ));
    }

    Ok(Json(subscription.into()))
}

#[utoipa::path(
    post,
    path = "/api/vps/subscribe",
    request_body = SelfSubscribeVpsRequest,
    responses(
        (status = 201, description = "Suscripción VPS creada + URL de checkout", body = SelfSubscribeVpsResponse),
        (status = 400, description = "Datos inválidos"),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn subscribe_self(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(req): Json<SelfSubscribeVpsRequest>,
) -> Result<(StatusCode, Json<SelfSubscribeVpsResponse>), AppError> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;

    let plan_config = VpsRepository::get_plan_config(&state.pool, &req.tier)
        .await?
        .filter(|config| config.is_active)
        .ok_or_else(|| AppError::Validation(format!("Tier inválido: {}", req.tier)))?;

    let user = UserRepository::find_by_id(&state.pool, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Usuario no encontrado".into()))?;

    let client_name = user.display_name.unwrap_or_else(|| user.email.clone());
    let client_email = user.email;

    let notes_str = build_client_notes(
        req.storage_preference.as_deref(),
        req.region_preference.as_deref(),
        req.os_preference.as_deref(),
        req.server_password.as_deref(),
    );

    /* [205A-1] Computar precio total real incluyendo extra de región y storage elegidos */
    let region_extra = selected_extra_cents(
        &plan_config.region_extra_cents,
        req.region_preference.as_deref(),
    )?;
    let storage_extra = selected_extra_cents(
        &plan_config.storage_extra_cents,
        req.storage_preference.as_deref(),
    )?;
    let total_monthly_cents = plan_config.monthly_price_cents + region_extra + storage_extra;

    let subscription = VpsRepository::create(
        &state.pool,
        CreateVpsSubscriptionParams {
            user_id: Some(auth.user_id),
            client_name: &client_name,
            client_email: &client_email,
            tier_name: &req.tier,
            requested_hostname: req.hostname.as_deref(),
            client_notes: notes_str.as_deref(),
            monthly_price_cents: total_monthly_cents,
        },
    )
    .await?;

    let _ = VpsRepository::add_event(
        &state.pool,
        subscription.id,
        "created",
        Some(serde_json::json!({
            "tier": req.tier,
            "source": "self-service",
            "by": auth.user_id.to_string(),
        })),
    )
    .await;

    let base_url = resolve_public_base_url(&headers);
    if is_checkout_bypass_email(&client_email) {
        let response = complete_test_bypass_checkout(
            &state.pool,
            subscription,
            &req.tier,
            auth.user_id,
            &base_url,
        )
        .await?;
        return Ok((StatusCode::CREATED, Json(response)));
    }

    let stripe_key = state
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::ServiceUnavailable("Stripe no configurado".into()))?;

    let success_url = format!("{base_url}/panel?vps=success&session_id={{CHECKOUT_SESSION_ID}}");
    let cancel_url = format!("{base_url}/panel?vps=cancelled");

    let total_first_payment = subscription.monthly_price_cents + plan_config.setup_fee_cents;
    let stripe_fee = vps_stripe_fee_cents(total_first_payment);

    let checkout_url = VpsStripeService::create_checkout_session(&VpsCheckoutParams {
        http_client: &state.http_client,
        stripe_key,
        subscription_id: subscription.id,
        tier_name: &subscription.tier_name,
        amount_cents: subscription.monthly_price_cents,
        setup_fee_cents: plan_config.setup_fee_cents,
        processing_fee_cents: stripe_fee,
        customer_email: &subscription.client_email,
        success_url: &success_url,
        cancel_url: &cancel_url,
    })
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(SelfSubscribeVpsResponse {
            subscription: subscription.into(),
            checkout_url,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/admin/vps/subscriptions/{id}/approve",
    params(("id" = Uuid, Path, description = "ID de la suscripción VPS")),
    responses(
        (status = 200, description = "VPS aprobado y provisionado", body = VpsSubscriptionResponse),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn approve_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<VpsSubscriptionResponse>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let subscription = VpsRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción VPS no encontrada".into()))?;

    if subscription.status != "pending_approval" {
        return Err(AppError::Validation(format!(
            "Solo se puede aprobar una suscripción en pending_approval. Estado actual: {}",
            subscription.status
        )));
    }

    let plan_config = VpsRepository::get_plan_config(&state.pool, &subscription.tier_name)
        .await?
        .filter(|config| config.is_active)
        .ok_or_else(|| {
            AppError::NotFound(format!("Tier '{}' no encontrado", subscription.tier_name))
        })?;

    let contabo_service = state
        .contabo_service
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Contabo API no configurada".into()))?;

    let requested_hostname =
        sanitize_hostname(subscription.requested_hostname.as_deref(), subscription.id);
    let access_username = "nakomi";
    let initial_password = generate_initial_password();
    let cloud_init = build_cloud_init(&requested_hostname);
    let image_id = std::env::var("CONTABO_DEFAULT_IMAGE_ID").ok();

    VpsRepository::mark_approved(&state.pool, subscription.id, auth.user_id).await?;

    let created_instance = match contabo_service
        .create_instance(&CreateInstanceParams {
            product_id: &plan_config.contabo_product_id,
            region: &plan_config.region,
            display_name: &requested_hostname,
            image_id: image_id.as_deref(),
            root_password: &initial_password,
            default_user: access_username,
            user_data: &cloud_init,
        })
        .await
    {
        Ok(instance) => instance,
        Err(error) => {
            VpsRepository::update_status(&state.pool, subscription.id, "pending_approval").await?;
            let _ = VpsRepository::add_event(
                &state.pool,
                subscription.id,
                "provision_failed",
                Some(serde_json::json!({"error": error, "by": auth.user_id.to_string()})),
            )
            .await;
            return Err(map_contabo_error(&error));
        }
    };

    VpsRepository::mark_provisioned(
        &state.pool,
        subscription.id,
        &crate::repositories::ProvisionedVpsInfo {
            contabo_instance_id: created_instance.instance_id,
            provisioning_ip: Some(&created_instance.ip),
            access_username,
            requested_hostname: Some(&requested_hostname),
        },
    )
    .await?;

    let _ = VpsRepository::add_event(
        &state.pool,
        subscription.id,
        "approved",
        Some(serde_json::json!({
            "by": auth.user_id.to_string(),
            "instance_id": created_instance.instance_id,
            "ip": created_instance.ip,
            "hostname": requested_hostname,
        })),
    )
    .await;

    VpsStripeService::notify_approved(
        &state.notification_hub,
        subscription.user_id,
        subscription.id,
        &subscription.tier_name,
        Some(&created_instance.ip),
    )
    .await;

    if let Some(config) = &state.email_config {
        EmailService::send_vps_approved(
            config,
            &state.pool,
            &subscription.client_email,
            &plan_config.display_name,
            Some(&created_instance.ip),
            access_username,
            &initial_password,
        )
        .await;
    }

    let updated = VpsRepository::find_by_id(&state.pool, subscription.id)
        .await?
        .ok_or_else(|| AppError::Internal("Suscripción VPS perdida tras aprobar".into()))?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/vps/subscriptions/{id}/reject",
    params(("id" = Uuid, Path, description = "ID de la suscripción VPS")),
    request_body = RejectVpsRequest,
    responses(
        (status = 204, description = "VPS rechazado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn reject_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RejectVpsRequest>,
) -> Result<StatusCode, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;

    let subscription = VpsRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción VPS no encontrada".into()))?;

    if subscription.status != "pending_approval" {
        return Err(AppError::Validation(format!(
            "Solo se puede rechazar una suscripción en pending_approval. Estado actual: {}",
            subscription.status
        )));
    }

    if let (Some(stripe_key), Some(stripe_subscription_id)) = (
        state.stripe_secret_key.as_deref(),
        subscription.stripe_subscription_id.as_deref(),
    ) {
        VpsStripeService::cancel_and_refund_subscription(
            &state.http_client,
            stripe_key,
            stripe_subscription_id,
        )
        .await?;
    }

    VpsRepository::mark_rejected(&state.pool, subscription.id, &req).await?;
    let _ = VpsRepository::add_event(
        &state.pool,
        subscription.id,
        "rejected",
        Some(serde_json::json!({"reason": req.reason, "by": auth.user_id.to_string()})),
    )
    .await;

    VpsStripeService::notify_rejected(
        &state.notification_hub,
        subscription.user_id,
        subscription.id,
        &subscription.tier_name,
        &req.reason,
    )
    .await;

    if let Some(config) = &state.email_config {
        let plan_name = VpsRepository::get_plan_config(&state.pool, &subscription.tier_name)
            .await?
            .map_or_else(|| subscription.tier_name.clone(), |plan| plan.display_name);
        EmailService::send_vps_rejected(
            config,
            &state.pool,
            &subscription.client_email,
            &plan_name,
            &req.reason,
        )
        .await;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/vps/public-plans", get(list_public_plans))
        .route("/vps/subscriptions", get(list_subscriptions))
        .route("/vps/subscriptions/:id", get(get_subscription))
        .route("/vps/subscribe", axum::routing::post(subscribe_self))
        .route(
            "/admin/vps/subscriptions/:id/approve",
            axum::routing::post(approve_subscription),
        )
        .route(
            "/admin/vps/subscriptions/:id/reject",
            axum::routing::post(reject_subscription),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_hostname_generates_safe_default() {
        let value = sanitize_hostname(None, Uuid::nil());
        assert!(value.starts_with("nakomi-vps-"));
    }

    #[test]
    fn sanitize_hostname_normalizes_input() {
        let value = sanitize_hostname(Some("Cliente Demo!!.nakomi"), Uuid::nil());
        assert!(value.contains("cliente-demo"));
        assert!(!value.contains('.'));
    }
}
