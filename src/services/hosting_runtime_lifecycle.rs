/* [259A-4b] Ciclo de vida de despliegues (extraído de hosting_runtime.rs):
 * listar, provisionar, actualizar, eliminar, detener, iniciar, reiniciar.
 * Despacha por runtime persistido; los runners lightweight viven en el módulo
 * hermano `hosting_runtime_lightweight` y se consumen vía `Self::`. */

use reqwest::Client;

use crate::errors::AppError;
use crate::models::HostingPlanConfig;

use super::coolify::{
    CoolifyConfig, CoolifyService, HostingComposeUpdate, HostingProvisionPreferences,
};
use super::hosting_runtime::{
    HostingRuntimeDeploymentSummary, HostingRuntimeKind, HostingRuntimeProvisionResult,
    HostingRuntimeService, HostingRuntimeUpdate,
};
use super::hosting_runtime_lightweight_types::LightweightManagerProvisionStaticReport;

impl HostingRuntimeService {
    pub async fn list_deployments(
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        runtime_kind: Option<HostingRuntimeKind>,
    ) -> Result<Vec<HostingRuntimeDeploymentSummary>, AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let config = Self::require_target_config_for(
                    runtime_kind,
                    coolify_config,
                    "listar despliegues",
                )?;
                Ok(CoolifyService::list_services(http_client, config)
                    .await?
                    .into_iter()
                    .map(Into::into)
                    .collect())
            }
            HostingRuntimeKind::Lightweight => Self::list_lightweight_deployments().await,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn provision_hosting(
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_name: &str,
        access_port: i32,
        plan_config: &HostingPlanConfig,
        client_name: &str,
        client_email: &str,
        preferences: Option<&HostingProvisionPreferences>,
    ) -> Result<HostingRuntimeProvisionResult, AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let config = Self::require_target_config_for(
                    runtime_kind,
                    coolify_config,
                    "provisionar hostings",
                )?;
                Ok(CoolifyService::provision_hosting(
                    http_client,
                    config,
                    deployment_name,
                    access_port,
                    plan_config,
                    client_name,
                    client_email,
                    preferences,
                )
                .await?
                .into())
            }
            HostingRuntimeKind::Lightweight => {
                if !plan_config.plan_name.starts_with("normal-") {
                    return Err(AppError::ServiceUnavailable(
                        "El runtime de hosting 'lightweight' aún solo soporta planes normal-*; los planes WordPress siguen en el runtime legacy.".into(),
                    ));
                }

                let mut args = vec![
                    "--site".to_string(),
                    deployment_name.to_string(),
                    "--json".to_string(),
                ];

                if let Some(access_user) = preferences
                    .and_then(|prefs| prefs.sftp_user.as_deref())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    args.push("--access-user".to_string());
                    args.push(access_user.to_string());
                }

                if let Some(access_password) = preferences
                    .and_then(|prefs| prefs.sftp_password.as_deref())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    args.push("--access-password".to_string());
                    args.push(access_password.to_string());
                }

                let report: LightweightManagerProvisionStaticReport =
                    Self::run_lightweight_manager_json(
                        "provisionar hostings",
                        "provision-static",
                        &args,
                    )
                    .await?;

                Ok(HostingRuntimeProvisionResult {
                    runtime_kind,
                    deployment_id: report.deployment_id,
                    public_url: report.public_url,
                    server_ip: report.target_ip,
                    access_user: report.access_user,
                    access_password: report.access_password,
                    access_port: i32::from(report.access_port),
                    wordpress_ready: false,
                    wordpress_install_error: None,
                })
            }
        }
    }

    pub async fn update_deployment(
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        runtime_kind: Option<HostingRuntimeKind>,
        update: HostingRuntimeUpdate<'_>,
    ) -> Result<(), AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let config = Self::require_target_config_for(
                    runtime_kind,
                    coolify_config,
                    "actualizar despliegues",
                )?;
                CoolifyService::update_compose_and_restart(
                    http_client,
                    config,
                    HostingComposeUpdate {
                        service_uuid: update.deployment_id,
                        service_name: update.deployment_name,
                        custom_domain: update.custom_domain,
                        sftp_user: update.access_user,
                        sftp_password: update.access_password,
                        sftp_port: update.access_port,
                        plan_config: update.plan_config,
                    },
                )
                .await
            }
            HostingRuntimeKind::Lightweight => {
                /* [245A-8] El runtime lightweight deja de ser un stub en refresh/domain/rotate.
                 * Gotcha: si estos updates siguieran exigiendo CoolifyConfig, el sitio podria provisionarse
                 * pero no cambiar dominio ni credenciales una vez creado. */
                let mut args = vec![
                    "--site".to_string(),
                    update.deployment_id.to_string(),
                    "--action".to_string(),
                    "reconfigure".to_string(),
                    "--access-user".to_string(),
                    update.access_user.to_string(),
                    "--access-password".to_string(),
                    update.access_password.to_string(),
                    "--json".to_string(),
                ];
                if let Some(custom_domain) = update.custom_domain {
                    args.push("--fqdn".to_string());
                    args.push(custom_domain.to_string());
                }
                let _: serde_json::Value = Self::run_lightweight_manager_json(
                    "actualizar despliegues",
                    "light-site",
                    &args,
                )
                .await?;
                Ok(())
            }
        }
    }

    pub async fn delete_deployment(
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_id: &str,
        delete_volumes: bool,
    ) -> Result<(), AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let config = Self::require_target_config_for(
                    runtime_kind,
                    coolify_config,
                    "eliminar despliegues",
                )?;
                CoolifyService::delete_service(http_client, config, deployment_id, delete_volumes)
                    .await
            }
            HostingRuntimeKind::Lightweight => {
                Self::run_lightweight_site_action(
                    "eliminar despliegues",
                    deployment_id,
                    "delete",
                    delete_volumes,
                )
                .await
            }
        }
    }

    pub async fn stop_deployment(
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_id: &str,
    ) -> Result<(), AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let config = Self::require_target_config_for(
                    runtime_kind,
                    coolify_config,
                    "detener despliegues",
                )?;
                CoolifyService::stop_service(http_client, config, deployment_id).await
            }
            HostingRuntimeKind::Lightweight => {
                Self::run_lightweight_site_action(
                    "detener despliegues",
                    deployment_id,
                    "stop",
                    false,
                )
                .await
            }
        }
    }

    pub async fn start_deployment(
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_id: &str,
    ) -> Result<(), AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let config = Self::require_target_config_for(
                    runtime_kind,
                    coolify_config,
                    "iniciar despliegues",
                )?;
                CoolifyService::start_service(http_client, config, deployment_id).await
            }
            HostingRuntimeKind::Lightweight => {
                Self::run_lightweight_site_action(
                    "iniciar despliegues",
                    deployment_id,
                    "start",
                    false,
                )
                .await
            }
        }
    }

    pub async fn restart_deployment(
        http_client: &Client,
        coolify_config: Option<&CoolifyConfig>,
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_id: &str,
    ) -> Result<(), AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let config = Self::require_target_config_for(
                    runtime_kind,
                    coolify_config,
                    "reiniciar despliegues",
                )?;
                CoolifyService::restart_service(http_client, config, deployment_id).await
            }
            HostingRuntimeKind::Lightweight => {
                Self::run_lightweight_site_action(
                    "reiniciar despliegues",
                    deployment_id,
                    "restart",
                    false,
                )
                .await
            }
        }
    }
}
