use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    CreateNotification, NOTIF_HOSTING_CANCELLED, NOTIF_HOSTING_SUSPENDED,
};
use crate::repositories::{HostingRepository, NotificationRepository};
use crate::services::{CoolifyConfig, HostingRuntimeService};

use super::helpers::hosting_product_copy;
use super::types::{CheckoutParams, CheckoutSession, HostingStripeService};

impl HostingStripeService {
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
