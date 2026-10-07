/* [07AA-14] Lifecycle Coolify: list/stop/start/restart/domain (split sin cambios desde coolify.rs). */

use reqwest::Client;
use serde_json::Value;

use crate::errors::AppError;

use super::config::CoolifyConfig;
use super::parsing::{parse_service_summary, service_matches_target};
use super::types::CoolifyServiceSummary;
use super::CoolifyService;

impl CoolifyService {
    /* [164A-19] Lista despliegues reales visibles en Coolify para un target configurado.
     * La tarea original confundía "instancia VPS" con "servicio desplegado"; esta función
     * corrige esa frontera leyendo directamente `/api/v1/services` y filtrando por target. */
    pub async fn list_services(
        http_client: &Client,
        config: &CoolifyConfig,
    ) -> Result<Vec<CoolifyServiceSummary>, AppError> {
        let url = format!("{}/api/v1/services", config.base_url);
        let resp = http_client
            .get(&url)
            .bearer_auth(&config.api_token)
            .send()
            .await
            .map_err(|e| {
                AppError::Internal(format!("Coolify list services request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                "[Coolify] Error listando servicios desde {}: {} — {}",
                url,
                status,
                body
            );
            return Err(AppError::Internal(format!(
                "Coolify list services failed ({url}): {status}"
            )));
        }

        let payload: Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Coolify list services parse error: {e}")))?;

        let services_raw = payload.as_array().ok_or_else(|| {
            AppError::Internal("Coolify list services parse error: expected array".into())
        })?;

        let mut services = Vec::new();
        let mut skipped = 0usize;

        for service_value in services_raw {
            let Some(service) = parse_service_summary(service_value) else {
                skipped += 1;
                continue;
            };

            if service_matches_target(&service, config) {
                services.push(service);
            }
        }

        if skipped > 0 {
            tracing::warn!(
                "[Coolify] {} servicio(s) omitidos del panel por respuesta incompleta.",
                skipped
            );
        }

        services.sort_by(|left, right| left.name.cmp(&right.name).then(left.uuid.cmp(&right.uuid)));

        Ok(services)
    }

    /// Genera un nombre de servicio Coolify a partir del hosting subscription UUID.
    /// Usa los primeros 8 chars del UUID para ser corto pero único.
    #[must_use]
    pub fn service_name_for(subscription_id: &uuid::Uuid) -> String {
        let id_str = subscription_id.to_string();
        let short = &id_str[..8];
        format!("hosting-{short}")
    }

    /* [154A-9] Control de servicio: stop / start / restart */

    pub async fn stop_service(
        http_client: &Client,
        config: &CoolifyConfig,
        service_uuid: &str,
    ) -> Result<(), AppError> {
        let url = format!("{}/api/v1/services/{}/stop", config.base_url, service_uuid);
        let resp = http_client
            .post(&url)
            .bearer_auth(&config.api_token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Coolify stop failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Coolify stop failed: {status} — {body}"
            )));
        }
        tracing::info!("[Coolify] Servicio {service_uuid} detenido.");
        Ok(())
    }

    pub async fn start_service(
        http_client: &Client,
        config: &CoolifyConfig,
        service_uuid: &str,
    ) -> Result<(), AppError> {
        let url = format!("{}/api/v1/services/{}/start", config.base_url, service_uuid);
        let resp = http_client
            .post(&url)
            .bearer_auth(&config.api_token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Coolify start failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Coolify start failed: {status} — {body}"
            )));
        }
        tracing::info!("[Coolify] Servicio {service_uuid} iniciado.");
        Ok(())
    }

    pub async fn restart_service(
        http_client: &Client,
        config: &CoolifyConfig,
        service_uuid: &str,
    ) -> Result<(), AppError> {
        let url = format!(
            "{}/api/v1/services/{}/restart",
            config.base_url, service_uuid
        );
        let resp = http_client
            .post(&url)
            .bearer_auth(&config.api_token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Coolify restart failed: {e}")))?;
        if resp.status().is_success() {
            tracing::info!("[Coolify] Servicio {service_uuid} reiniciado.");
        } else {
            /* Fallback: stop + start si restart endpoint no existe */
            tracing::warn!("[Coolify] Restart endpoint falló, intentando stop+start.");
            Self::stop_service(http_client, config, service_uuid).await?;
            Self::start_service(http_client, config, service_uuid).await?;
        }
        Ok(())
    }

    /* [154A-4] Actualizar el FQDN de un servicio Coolify cuando cambia el dominio.
     * Coolify v4 congela el FQDN al crear el servicio, así que intentamos:
     * 1. PATCH al endpoint de servicio con la nueva FQDN
     * 2. Si no funciona, se necesitan labels Traefik explícitas en el compose.
     * En cualquier caso, se hace redeploy para aplicar cambios. */
    pub async fn update_service_domain(
        http_client: &Client,
        config: &CoolifyConfig,
        service_uuid: &str,
        new_domain: &str,
    ) -> Result<(), AppError> {
        let url = format!("{}/api/v1/services/{service_uuid}", config.base_url);

        let fqdn = if new_domain.starts_with("http") {
            new_domain.to_string()
        } else {
            format!("https://{new_domain}")
        };

        let body = serde_json::json!({
            "domains": [fqdn],
        });

        let resp = http_client
            .patch(&url)
            .bearer_auth(&config.api_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Coolify FQDN update failed: {e}")))?;

        if resp.status().is_success() {
            tracing::info!("[Coolify] FQDN actualizado a {fqdn} para servicio {service_uuid}");
        } else {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            tracing::warn!(
                "[Coolify] FQDN update retornó {status}: {body_text}. \
                 Puede requerir labels Traefik manuales en el compose."
            );
        }

        /* Redeploy para que Coolify aplique la nueva config */
        Self::restart_service(http_client, config, service_uuid).await?;

        Ok(())
    }
}
