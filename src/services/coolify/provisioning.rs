/* [07AA-14] Provisioning Coolify: provision/update/delete (split sin cambios desde coolify.rs). */

use base64::Engine;
use reqwest::Client;

use crate::errors::AppError;
use crate::models::HostingPlanConfig;

use super::compose::{
    build_hosting_compose_for_service, build_hosting_compose_for_service_with_ingress_network,
};
use super::config::CoolifyConfig;
use super::parsing::{generate_sftp_credentials, hosting_bootstrap_host};
use super::types::{CoolifyProvisionResult, HostingComposeUpdate, HostingProvisionPreferences};
use super::wordpress::{
    create_hosting_service, finalize_wordpress_install, WordpressInstallContext,
};
use super::CoolifyService;

impl CoolifyService {
    /// Provision completo: crea servicio `WordPress` en Coolify y lo arranca.
    /// Usa `docker_compose_raw` con `WordPress` + `MariaDB` + SSH/SFTP.
    /// El nombre de servicio garantiza idempotencia (Coolify rechaza duplicados).
    /// `sftp_port` debe generarse externamente con `HostingRepository::find_available_sftp_port`.
    /// `plan_config` determina límites de recursos y features extra (backup sidecar para ecommerce).
    ///
    /// # Errors
    /// Retorna `AppError` si la API falla o el JSON no parsea.
    #[allow(clippy::too_many_arguments)]
    pub async fn provision_hosting(
        http_client: &Client,
        config: &CoolifyConfig,
        service_name: &str,
        sftp_port: i32,
        plan_config: &HostingPlanConfig,
        client_name: &str,
        client_email: &str,
        preferences: Option<&HostingProvisionPreferences>,
    ) -> Result<CoolifyProvisionResult, AppError> {
        let (generated_sftp_user, generated_sftp_password) = generate_sftp_credentials();
        let sftp_user = preferences
            .and_then(|prefs| prefs.sftp_user.as_deref())
            .unwrap_or(&generated_sftp_user)
            .to_string();
        let sftp_password = preferences
            .and_then(|prefs| prefs.sftp_password.as_deref())
            .unwrap_or(&generated_sftp_password)
            .to_string();
        let wp_admin_username = preferences
            .and_then(|prefs| prefs.wp_admin_username.as_deref())
            .unwrap_or(&sftp_user)
            .to_string();
        let wp_admin_password = preferences
            .and_then(|prefs| prefs.wp_admin_password.as_deref())
            .unwrap_or(&sftp_password)
            .to_string();
        let wp_language = preferences
            .and_then(|prefs| prefs.wp_language.as_deref())
            .unwrap_or("")
            .to_string();
        let bootstrap_domain =
            hosting_bootstrap_host(&plan_config.plan_name, service_name, &config.server_ip);
        let compose_yaml = build_hosting_compose_for_service(
            service_name,
            &config.server_ip,
            None,
            &sftp_user,
            &sftp_password,
            sftp_port,
            plan_config,
        );
        let compose_b64 = base64::engine::general_purpose::STANDARD.encode(&compose_yaml);

        let created =
            create_hosting_service(http_client, config, service_name, compose_b64).await?;

        Self::update_compose_and_restart(
            http_client,
            config,
            HostingComposeUpdate {
                service_uuid: &created.uuid,
                service_name,
                custom_domain: None,
                sftp_user: &sftp_user,
                sftp_password: &sftp_password,
                sftp_port,
                plan_config,
            },
        )
        .await?;

        let coolify_domain = created.domains.into_iter().next();
        let domain = format!("http://{bootstrap_domain}");

        tracing::info!(
            "[Coolify] Servicio '{}' creado: uuid={}, bootstrap_domain={}, coolify_domain={:?}",
            service_name,
            created.uuid,
            domain,
            coolify_domain
        );
        let (wordpress_ready, wordpress_install_error) = finalize_wordpress_install(
            http_client,
            &domain,
            WordpressInstallContext {
                service_name,
                service_uuid: &created.uuid,
                plan_name: &plan_config.plan_name,
                client_name,
                client_email,
                admin_username: &wp_admin_username,
                admin_password: &wp_admin_password,
                language: &wp_language,
            },
        )
        .await;

        Ok(CoolifyProvisionResult {
            service_uuid: created.uuid,
            domain,
            server_ip: config.server_ip.clone(),
            sftp_user,
            sftp_password,
            sftp_port,
            wordpress_ready,
            wordpress_install_error,
        })
    }

    /// Detiene y elimina un servicio de Coolify.
    /// Usado cuando se cancela o suspende definitivamente un hosting.
    ///
    /// # Errors
    /// Retorna `AppError` si la API falla. El caller decide si es fatal.
    pub async fn delete_service(
        http_client: &Client,
        config: &CoolifyConfig,
        service_uuid: &str,
        delete_volumes: bool,
    ) -> Result<(), AppError> {
        /* Detener primero para liberar recursos gradualmente */
        let stop_url = format!("{}/api/v1/services/{}/stop", config.base_url, service_uuid);
        let stop_resp = http_client
            .post(&stop_url)
            .bearer_auth(&config.api_token)
            .send()
            .await;

        match stop_resp {
            Ok(r) if r.status().is_success() => {
                tracing::info!("[Coolify] Servicio {} detenido.", service_uuid);
            }
            Ok(r) => {
                tracing::warn!(
                    "[Coolify] No se pudo detener servicio {}: {}",
                    service_uuid,
                    r.status()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "[Coolify] Error de red al detener servicio {}: {}",
                    service_uuid,
                    e
                );
            }
        }

        /* Eliminar el servicio (y volúmenes si se indica) */
        let delete_url = format!(
            "{}/api/v1/services/{}?delete_volumes={}&delete_networks=true&docker_cleanup=false",
            config.base_url,
            service_uuid,
            if delete_volumes { "true" } else { "false" }
        );

        let del_resp = http_client
            .delete(&delete_url)
            .bearer_auth(&config.api_token)
            .send()
            .await
            .map_err(|e| {
                AppError::Internal(format!("Coolify delete service request failed: {e}"))
            })?;

        if del_resp.status().is_success() {
            tracing::info!(
                "[Coolify] Servicio {} eliminado (delete_volumes={}).",
                service_uuid,
                delete_volumes
            );
        } else {
            let status = del_resp.status();
            let body = del_resp.text().await.unwrap_or_default();
            tracing::error!(
                "[Coolify] Error eliminando servicio {}: {} — {}",
                service_uuid,
                status,
                body
            );
            return Err(AppError::Internal(format!(
                "Coolify delete service failed: {status}"
            )));
        }

        Ok(())
    }

    /* [114A-1] Actualiza el compose YAML de un servicio y lo reinicia.
     * Usado para rotación de credenciales SFTP: el compose se regenera con la nueva
     * contraseña, se sube vía PATCH y se reinicia el servicio (stop+start).
     * [114A-3] Ahora usa HostingPlanConfig para límites dinámicos. */
    pub async fn update_compose_and_restart(
        http_client: &Client,
        config: &CoolifyConfig,
        update: HostingComposeUpdate<'_>,
    ) -> Result<(), AppError> {
        let compose = build_hosting_compose_for_service_with_ingress_network(
            update.service_name,
            &config.server_ip,
            update.custom_domain,
            update.sftp_user,
            update.sftp_password,
            update.sftp_port,
            update.plan_config,
            Some(update.service_uuid),
        );
        let compose_b64 = base64::engine::general_purpose::STANDARD.encode(&compose);

        /* PATCH: actualizar docker_compose_raw en Coolify */
        let patch_url = format!(
            "{}/api/v1/services/{}",
            config.base_url, update.service_uuid
        );
        let patch_resp = http_client
            .patch(&patch_url)
            .bearer_auth(&config.api_token)
            .json(&serde_json::json!({ "docker_compose_raw": compose_b64 }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Coolify PATCH compose failed: {e}")))?;

        if !patch_resp.status().is_success() {
            let status = patch_resp.status();
            let body = patch_resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Coolify PATCH compose failed: {status} — {body}"
            )));
        }

        /* Restart: stop + start para aplicar nuevo compose */
        let stop_url = format!(
            "{}/api/v1/services/{}/stop",
            config.base_url, update.service_uuid
        );
        if let Err(e) = http_client
            .post(&stop_url)
            .bearer_auth(&config.api_token)
            .send()
            .await
        {
            tracing::warn!(
                "[Coolify] Error deteniendo servicio {}: {e}",
                update.service_uuid
            );
        }
        let start_url = format!(
            "{}/api/v1/services/{}/start",
            config.base_url, update.service_uuid
        );
        let start_resp = http_client
            .post(&start_url)
            .bearer_auth(&config.api_token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Coolify restart failed: {e}")))?;

        if start_resp.status().is_success() {
            tracing::info!(
                "[Coolify] Servicio {} reiniciado con credenciales actualizadas.",
                update.service_uuid
            );
        } else {
            let status = start_resp.status();
            tracing::warn!(
                "[Coolify] Compose actualizado pero restart falló para {}: {status}",
                update.service_uuid
            );
        }

        Ok(())
    }
}
