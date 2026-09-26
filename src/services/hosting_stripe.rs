/* [084A-24] Servicio de suscripciones Stripe para hosting.
 * Crea Checkout Sessions en modo subscription usando price_data dinámico desde BD.
 * Maneja webhooks de invoice.paid y customer.subscription.* para sincronizar estado.
 * Gotcha: NO usa PaymentIntents directos — usa Stripe Subscriptions nativas.
 * [094A-9] Auditoría de seguridad: validación estricta de campos JSON en webhooks,
 * idempotency key en checkout, verificación de status antes de activar. */

use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    CreateNotification, HostingPlanConfig, HostingSubscription, NOTIF_HOSTING_CANCELLED,
    NOTIF_HOSTING_SUSPENDED,
};
use crate::repositories::{HostingRepository, NotificationRepository, ServerInfo};
use crate::services::coolify::HostingProvisionPreferences;
use crate::services::{CoolifyConfig, HostingRuntimeProvisionResult, HostingRuntimeService};

/* Respuesta mínima de Stripe Checkout Session */
#[derive(Debug, Deserialize)]
struct CheckoutSession {
    id: String,
    url: Option<String>,
}

/// Parámetros para crear una Checkout Session de hosting
pub struct CheckoutParams<'a> {
    pub http_client: &'a Client,
    pub stripe_key: &'a str,
    pub subscription_id: Uuid,
    pub plan: &'a str,
    pub amount_cents: i32,
    pub customer_email: &'a str,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
    pub billing_cycle_months: i32,
}

pub struct HostingStripeService;

fn humanize_plan_name(plan: &str) -> &'static str {
    match plan.strip_prefix("normal-").unwrap_or(plan) {
        "basico" => "Basico",
        "pro" => "Pro",
        "ecommerce" => "Avanzado",
        _ => "Personalizado",
    }
}

fn read_checkout_config_string(details: &serde_json::Value, field: &str) -> Option<String> {
    let config = details.get("checkout_config").unwrap_or(details);
    config
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn provisioning_preferences_from_details(
    details: &serde_json::Value,
) -> Option<HostingProvisionPreferences> {
    let preferences = HostingProvisionPreferences {
        wp_admin_username: read_checkout_config_string(details, "wp_admin_username"),
        wp_admin_password: read_checkout_config_string(details, "wp_admin_password"),
        wp_language: read_checkout_config_string(details, "wp_language"),
        sftp_user: read_checkout_config_string(details, "sftp_user"),
        sftp_password: read_checkout_config_string(details, "sftp_password"),
    };

    if preferences.wp_admin_username.is_some()
        || preferences.wp_admin_password.is_some()
        || preferences.wp_language.is_some()
        || preferences.sftp_user.is_some()
        || preferences.sftp_password.is_some()
    {
        return Some(preferences);
    }

    None
}

fn hosting_product_copy(plan: &str) -> (String, String) {
    let plan_name = humanize_plan_name(plan);
    if plan.starts_with("normal-") {
        return (
            format!("Hosting {plan_name}"),
            format!("Plan {plan_name} de hosting administrado para sitios a medida y frontends"),
        );
    }

    (
        format!("Hosting WordPress {plan_name}"),
        format!("Plan {plan_name} de hosting WordPress administrado"),
    )
}

async fn load_auto_provision_request(
    pool: &PgPool,
    subscription: &HostingSubscription,
    activation_source: &str,
) -> Option<(String, i32, HostingPlanConfig)> {
    let hosting_id = subscription.id;
    let service_name = HostingRuntimeService::deployment_name_for(&hosting_id);
    let sftp_port = match HostingRepository::find_available_sftp_port(pool).await {
        Ok(port) => port,
        Err(error) => {
            tracing::warn!(
                "No se pudo generar puerto SFTP para {hosting_id} ({activation_source}): {error}"
            );
            return None;
        }
    };

    let plan_config = match HostingRepository::get_plan_config(pool, &subscription.plan).await {
        Ok(Some(config)) => config,
        Ok(None) => {
            tracing::warn!(
                "Plan config '{}' no encontrado para hosting {hosting_id} ({activation_source})",
                subscription.plan
            );
            return None;
        }
        Err(error) => {
            tracing::warn!(
                "Error obteniendo plan config para {hosting_id} ({activation_source}): {error}"
            );
            return None;
        }
    };

    Some((service_name, sftp_port, plan_config))
}

async fn record_auto_provision_success(
    pool: &PgPool,
    hosting_id: Uuid,
    service_name: &str,
    activation_source: &str,
    result: &HostingRuntimeProvisionResult,
) {
    if let Err(error) = HostingRepository::update_server_info(
        pool,
        hosting_id,
        &ServerInfo {
            runtime_kind: result.runtime_kind.as_str(),
            deployment_id: &result.deployment_id,
            coolify_site_name: service_name,
            server_uuid: &result.deployment_id,
            server_ip: &result.server_ip,
            sftp_user: &result.access_user,
            sftp_password: &result.access_password,
            sftp_port: result.access_port,
        },
    )
    .await
    {
        tracing::warn!(
            "Error guardando server_info para hosting {} ({}): {error}",
            hosting_id,
            activation_source
        );
    }
    if let Err(error) = HostingRepository::add_event(
        pool,
        hosting_id,
        "coolify_provisioned",
        Some(serde_json::json!({
            "runtime_kind": result.runtime_kind.as_str(),
            "deployment_id": result.deployment_id,
            "public_url": result.public_url,
            "server_ip": result.server_ip,
            "wordpress_ready": result.wordpress_ready,
            "wordpress_install_error": result.wordpress_install_error,
            "source": activation_source,
        })),
    )
    .await
    {
        tracing::warn!(
            "Error registrando evento coolify_provisioned para {} ({}): {error}",
            hosting_id,
            activation_source
        );
    }
}

async fn record_auto_provision_failure(
    pool: &PgPool,
    hosting_id: Uuid,
    activation_source: &str,
    error: &AppError,
) {
    if let Err(event_error) = HostingRepository::add_event(
        pool,
        hosting_id,
        "coolify_provision_failed",
        Some(serde_json::json!({
            "error": error.to_string(),
            "source": activation_source,
        })),
    )
    .await
    {
        tracing::warn!(
            "Error registrando evento coolify_provision_failed para {} ({}): {event_error}",
            hosting_id,
            activation_source
        );
    }
}

impl HostingStripeService {
    #[must_use]
    pub fn runtime_kind_for_plan(plan: &str) -> crate::services::HostingRuntimeKind {
        HostingRuntimeService::runtime_kind_for_plan(plan)
    }

    pub async fn load_provision_preferences(
        pool: &PgPool,
        hosting_id: Uuid,
    ) -> Option<HostingProvisionPreferences> {
        let events = match HostingRepository::list_events(pool, hosting_id, 20).await {
            Ok(events) => events,
            Err(error) => {
                tracing::warn!(
                    "No se pudieron leer preferencias de provisioning para {hosting_id}: {error}"
                );
                return None;
            }
        };

        events.into_iter().find_map(|event| {
            if event.event_type != "created" {
                return None;
            }
            event
                .details
                .as_ref()
                .and_then(provisioning_preferences_from_details)
        })
    }

    /* [165A-1] El checkout bypass de cuentas de prueba debe provisionar igual que el
     * webhook real de Stripe; si no, la suscripción queda `active` pero sin SSH/SFTP. */
    pub async fn try_auto_provision_subscription(
        pool: &PgPool,
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        subscription: &HostingSubscription,
        activation_source: &str,
    ) {
        let hosting_id = subscription.id;
        let runtime_kind =
            crate::services::HostingRuntimeKind::from_persisted(&subscription.runtime_kind);
        let Some((service_name, sftp_port, plan_config)) =
            load_auto_provision_request(pool, subscription, activation_source).await
        else {
            return;
        };
        let preferences = Self::load_provision_preferences(pool, hosting_id).await;

        match HostingRuntimeService::provision_hosting(
            http_client,
            coolify_config,
            Some(runtime_kind),
            &service_name,
            sftp_port,
            &plan_config,
            &subscription.client_name,
            &subscription.client_email,
            preferences.as_ref(),
        )
        .await
        {
            Ok(result) => {
                tracing::info!(
                    "Hosting {} provisionado en runtime {} via {}: deployment={}, url={}, ip={}",
                    hosting_id,
                    result.runtime_kind.as_str(),
                    activation_source,
                    result.deployment_id,
                    result.public_url,
                    result.server_ip
                );
                record_auto_provision_success(
                    pool,
                    hosting_id,
                    &service_name,
                    activation_source,
                    &result,
                )
                .await;
            }
            Err(error) => {
                tracing::warn!(
                    "El provisioning automático falló para hosting {} via {} (error: {}). Requiere setup manual.",
                    hosting_id,
                    activation_source,
                    error
                );
                record_auto_provision_failure(pool, hosting_id, activation_source, &error).await;
            }
        }
    }

    /// Crea una Stripe Checkout Session para suscripción de hosting.
    /// Retorna la URL a la que redirigir al cliente.
    pub async fn create_checkout_session(params: &CheckoutParams<'_>) -> Result<String, AppError> {
        if params.amount_cents <= 0 {
            return Err(AppError::Validation(
                "El checkout de hosting requiere un precio mensual mayor a 0".into(),
            ));
        }

        let (product_name, product_description) = hosting_product_copy(params.plan);

        let mut form = vec![
            ("mode", "subscription".to_string()),
            ("line_items[0][price_data][currency]", "usd".to_string()),
            (
                "line_items[0][price_data][unit_amount]",
                params.amount_cents.to_string(),
            ),
            (
                "line_items[0][price_data][product_data][name]",
                product_name,
            ),
            (
                "line_items[0][price_data][product_data][description]",
                product_description,
            ),
            (
                "line_items[0][price_data][recurring][interval]",
                "month".to_string(),
            ),
            ("line_items[0][quantity]", "1".to_string()),
            ("customer_email", params.customer_email.to_string()),
            ("success_url", params.success_url.to_string()),
            ("cancel_url", params.cancel_url.to_string()),
            ("metadata[resource_kind]", "hosting".to_string()),
            (
                "metadata[hosting_subscription_id]",
                params.subscription_id.to_string(),
            ),
            (
                "subscription_data[metadata][resource_kind]",
                "hosting".to_string(),
            ),
            (
                "subscription_data[metadata][hosting_subscription_id]",
                params.subscription_id.to_string(),
            ),
        ];
        if params.billing_cycle_months > 1 {
            form.push((
                "line_items[0][price_data][recurring][interval_count]",
                params.billing_cycle_months.to_string(),
            ));
        }

        let resp = params
            .http_client
            .post("https://api.stripe.com/v1/checkout/sessions")
            .basic_auth(params.stripe_key, Option::<&str>::None)
            /* [094A-9] Idempotency key: evita sesiones duplicadas si la request se reintenta */
            .header(
                "Idempotency-Key",
                format!("hosting-checkout-{}", params.subscription_id),
            )
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Stripe checkout request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("Stripe checkout error: {status} — {body}");
            return Err(AppError::Internal(format!(
                "Stripe checkout session failed: {status}"
            )));
        }

        let session: CheckoutSession = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Stripe parse error: {e}")))?;

        tracing::info!(
            "Checkout session creada: {} para hosting {}",
            session.id,
            params.subscription_id
        );

        session
            .url
            .ok_or_else(|| AppError::Internal("Stripe no retornó URL de checkout".into()))
    }

    /// Procesa webhooks de Stripe relacionados a hosting subscriptions.
    /// Eventos: checkout.session.completed, invoice.paid, customer.subscription.deleted
    pub async fn handle_webhook(
        pool: &PgPool,
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        event_type: &str,
        data: &serde_json::Value,
    ) -> Result<bool, AppError> {
        match event_type {
            "checkout.session.completed" => {
                Self::on_checkout_completed(pool, http_client, coolify_config, data).await
            }
            "invoice.paid" => Self::on_invoice_paid(pool, data).await,
            "customer.subscription.deleted" => {
                Self::on_subscription_deleted(pool, http_client, coolify_config, data).await
            }
            "invoice.payment_failed" => Self::on_payment_failed(pool, data).await,
            _ => Ok(false),
        }
    }

    /* Cliente completó el checkout — activar suscripción */
    #[allow(clippy::too_many_lines)]
    /* sentinel-disable-next-line funcion-larga-rs: activa hosting, intenta provisioning y deja auditoría consistente en un único punto transaccional de webhook. */
    async fn on_checkout_completed(
        pool: &PgPool,
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        data: &serde_json::Value,
    ) -> Result<bool, AppError> {
        let mode = data["object"]["mode"].as_str().unwrap_or("");
        if mode != "subscription" {
            return Ok(false);
        }

        let sub_id_str = data["object"]["metadata"]["hosting_subscription_id"]
            .as_str()
            .or_else(|| {
                data["object"]["subscription_data"]["metadata"]["hosting_subscription_id"].as_str()
            });

        let Some(hosting_id) = sub_id_str.and_then(|s| Uuid::parse_str(s).ok()) else {
            return Ok(false);
        };

        /* [094A-9] Validar que subscription ID no esté vacío — campo crítico para vincular Stripe */
        let stripe_sub_id = data["object"]["subscription"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                tracing::warn!("Webhook checkout.session.completed sin subscription ID para hosting {hosting_id}");
                AppError::BadRequest("Missing subscription field in checkout event".into())
            })?;

        /* [094A-9] Verificar que la suscripción existe y está en estado pendiente antes de activar */
        let existing = HostingRepository::find_by_id(pool, hosting_id).await?;
        let Some(existing) = existing else {
            tracing::warn!("Webhook: hosting subscription {hosting_id} no existe en BD");
            return Ok(false);
        };
        if existing.status != "pending" && existing.status != "provisioning" {
            tracing::warn!(
                "Webhook: hosting {hosting_id} tiene status '{}', esperaba 'pending' o 'provisioning'",
                existing.status
            );
            return Ok(false);
        }

        HostingRepository::set_stripe_subscription_id(pool, hosting_id, stripe_sub_id).await?;
        HostingRepository::update_status(pool, hosting_id, "active").await?;

        Self::try_auto_provision_subscription(
            pool,
            http_client,
            coolify_config,
            &existing,
            "stripe_checkout_completed",
        )
        .await;

        /* [094A-9] Log de errores en evento, no silenciar */
        if let Err(e) = HostingRepository::add_event(
            pool,
            hosting_id,
            "stripe_checkout_completed",
            Some(serde_json::json!({"stripe_subscription_id": stripe_sub_id})),
        )
        .await
        {
            tracing::warn!(
                "Error registrando evento stripe_checkout_completed para {hosting_id}: {e}"
            );
        }

        tracing::info!("Hosting {hosting_id} activado via Stripe checkout (sub: {stripe_sub_id})");
        Ok(true)
    }

    /* Factura pagada — renovación mensual exitosa */
    async fn on_invoice_paid(pool: &PgPool, data: &serde_json::Value) -> Result<bool, AppError> {
        /* [094A-9] Validar campo subscription explícitamente */
        let stripe_sub_id = match data["object"]["subscription"].as_str() {
            Some(id) if !id.is_empty() => id,
            _ => return Ok(false),
        };

        let Some(hosting) =
            HostingRepository::find_by_stripe_subscription(pool, stripe_sub_id).await?
        else {
            return Ok(false);
        };

        if hosting.status != "active" {
            HostingRepository::update_status(pool, hosting.id, "active").await?;
        }

        /* [094A-9] Log de errores en evento, no silenciar */
        if let Err(e) = HostingRepository::add_event(
            pool,
            hosting.id,
            "invoice_paid",
            Some(serde_json::json!({
                "invoice_id": data["object"]["id"].as_str(),
                "amount_paid": data["object"]["amount_paid"].as_i64(),
            })),
        )
        .await
        {
            tracing::warn!(
                "Error registrando evento invoice_paid para {}: {e}",
                hosting.id
            );
        }

        tracing::info!("Hosting {} — invoice paid", hosting.id);
        Ok(true)
    }

    /* Suscripción cancelada en Stripe */
    async fn on_subscription_deleted(
        pool: &PgPool,
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        data: &serde_json::Value,
    ) -> Result<bool, AppError> {
        /* [094A-9] Validar campo id explícitamente */
        let stripe_sub_id = match data["object"]["id"].as_str() {
            Some(id) if !id.is_empty() => id,
            _ => return Ok(false),
        };
        let Some(hosting) =
            HostingRepository::find_by_stripe_subscription(pool, stripe_sub_id).await?
        else {
            return Ok(false);
        };

        HostingRepository::update_status(pool, hosting.id, "cancelled").await?;

        /* [104A-42] Eliminar despliegue del runtime al cancelar — no-fatal */
        if let Some(deployment_id) = hosting.deployment_id_or_legacy() {
            let runtime_kind =
                crate::services::HostingRuntimeKind::from_persisted(&hosting.runtime_kind);
            if let Err(e) = HostingRuntimeService::delete_deployment(
                http_client,
                coolify_config,
                Some(runtime_kind),
                deployment_id,
                false,
            )
            .await
            {
                tracing::warn!(
                    "Error eliminando despliegue {} para hosting {}: {e}",
                    deployment_id,
                    hosting.id
                );
            } else {
                tracing::info!(
                    "Despliegue {} eliminado por cancelación de hosting {}",
                    deployment_id,
                    hosting.id
                );
            }
        }

        /* [Notificación cancelación] Notificar al propietario del hosting si tiene cuenta */
        if let Some(user_id) = hosting.user_id {
            let notif = CreateNotification {
                user_id,
                notification_type: NOTIF_HOSTING_CANCELLED.to_string(),
                title: "Tu plan de hosting fue cancelado".to_string(),
                body: Some(
                    "Tu suscripción de hosting ha sido cancelada. Si crees que es un error, contáctanos.".to_string(),
                ),
                link: Some("/panel/hosting".to_string()),
                reference_type: Some("hosting_subscription".to_string()),
                reference_id: Some(hosting.id),
            };
            if let Err(e) = NotificationRepository::create(pool, &notif).await {
                tracing::warn!(
                    "Error creando notificación hosting_cancelled para user {user_id}: {e}"
                );
            }
        }

        /* [094A-9] Log de errores en evento, no silenciar */
        if let Err(e) =
            HostingRepository::add_event(pool, hosting.id, "stripe_subscription_cancelled", None)
                .await
        {
            tracing::warn!(
                "Error registrando evento stripe_subscription_cancelled para {}: {e}",
                hosting.id
            );
        }

        tracing::info!("Hosting {} cancelado via Stripe webhook", hosting.id);
        Ok(true)
    }

    /* Pago fallido — suspender hosting */
    async fn on_payment_failed(pool: &PgPool, data: &serde_json::Value) -> Result<bool, AppError> {
        /* [094A-9] Validar campo subscription explícitamente */
        let stripe_sub_id = match data["object"]["subscription"].as_str() {
            Some(id) if !id.is_empty() => id,
            _ => return Ok(false),
        };
        let Some(hosting) =
            HostingRepository::find_by_stripe_subscription(pool, stripe_sub_id).await?
        else {
            return Ok(false);
        };

        HostingRepository::update_status(pool, hosting.id, "suspended").await?;

        /* [Notificación suspensión] Notificar al propietario si tiene cuenta */
        if let Some(user_id) = hosting.user_id {
            let notif = CreateNotification {
                user_id,
                notification_type: NOTIF_HOSTING_SUSPENDED.to_string(),
                title: "Tu hosting fue suspendido por pago fallido".to_string(),
                body: Some(
                    "No pudimos procesar tu pago. Tu hosting está suspendido. Actualiza tu método de pago para restaurarlo.".to_string(),
                ),
                link: Some("/panel/hosting".to_string()),
                reference_type: Some("hosting_subscription".to_string()),
                reference_id: Some(hosting.id),
            };
            if let Err(e) = NotificationRepository::create(pool, &notif).await {
                tracing::warn!(
                    "Error creando notificación hosting_suspended para user {user_id}: {e}"
                );
            }
        }

        /* [094A-9] Log de errores en evento, no silenciar */
        if let Err(e) = HostingRepository::add_event(
            pool,
            hosting.id,
            "payment_failed",
            Some(serde_json::json!({"invoice_id": data["object"]["id"].as_str()})),
        )
        .await
        {
            tracing::warn!(
                "Error registrando evento payment_failed para {}: {e}",
                hosting.id
            );
        }

        tracing::warn!("Hosting {} suspendido por pago fallido", hosting.id);
        Ok(true)
    }
}

/* ============================================================
TESTS — [094A-10] Validación de lógica Stripe hosting
============================================================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_plan_name_maps_known_slugs() {
        assert_eq!(humanize_plan_name("basico"), "Basico");
        assert_eq!(humanize_plan_name("pro"), "Pro");
        assert_eq!(humanize_plan_name("ecommerce"), "Avanzado");
        assert_eq!(humanize_plan_name("normal-pro"), "Pro");
        assert_eq!(humanize_plan_name("custom"), "Personalizado");
    }

    #[test]
    fn hosting_product_copy_distinguishes_normal_hosting() {
        let (normal_name, normal_desc) = hosting_product_copy("normal-basico");
        assert_eq!(normal_name, "Hosting Basico");
        assert!(normal_desc.contains("sitios a medida"));

        let (wordpress_name, wordpress_desc) = hosting_product_copy("basico");
        assert_eq!(wordpress_name, "Hosting WordPress Basico");
        assert!(wordpress_desc.contains("hosting WordPress"));
    }
}
