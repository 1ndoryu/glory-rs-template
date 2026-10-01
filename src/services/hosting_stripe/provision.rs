use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{HostingPlanConfig, HostingSubscription};
use crate::repositories::{HostingRepository, ServerInfo};
use crate::services::coolify::HostingProvisionPreferences;
use crate::services::{CoolifyConfig, HostingRuntimeProvisionResult, HostingRuntimeService};

use super::helpers::provisioning_preferences_from_details;
use super::types::HostingStripeService;

pub(crate) async fn load_auto_provision_request(
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

pub(crate) async fn record_auto_provision_success(
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

pub(crate) async fn record_auto_provision_failure(
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
}
