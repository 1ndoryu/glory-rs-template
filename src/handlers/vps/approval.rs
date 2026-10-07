/* [07AA-14] Aprobación/rechazo VPS extraído de `vps.rs`: approve provisiona la
 * instancia Contabo (hostname sanitizado, password inicial, cloud-init con
 * docker+ufw+fail2ban) y notifica; reject cancela/reembolsa en Stripe. */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use rand::Rng;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{RejectVpsRequest, UserRole, VpsSubscriptionResponse};
use crate::repositories::VpsRepository;
use crate::services::{CreateInstanceParams, EmailService, VpsStripeService};
use crate::AppState;

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
