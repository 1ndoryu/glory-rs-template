/* [245A-3] Fachada de runtime de hosting para desacoplar el dominio de la API
 * directa de Coolify. En este bloque el provider `lightweight` queda declarado
 * pero no implementado para no mezclar la abstracción con el runtime nuevo.
 * [245A-6] `runtime_kind` y `deployment_id` ya se persisten en la suscripción,
 * así que control, borrado y refresh pueden despacharse por runtime guardado.
 * [259A-4b] Hub delgado: tipos, `HostingRuntimeKind`, dispatch de config y
 * tests. Ciclo de vida → `hosting_runtime_lifecycle`; backups →
 * `hosting_runtime_backup_ops`; runner lightweight → `hosting_runtime_lightweight`.
 * Gotcha: `resolved_kind`, `unsupported` y `LightweightManagerConfig` son
 * `pub(crate)` porque los módulos hermanos los consumen vía `Self::`. */

use uuid::Uuid;

use crate::errors::AppError;
use crate::models::HostingPlanConfig;

use super::coolify::{CoolifyConfig, CoolifyProvisionResult, CoolifyServiceSummary};
pub use super::hosting_runtime_backups::{
    HostingRuntimeBackupEntry, HostingRuntimeBackupReport, HostingRuntimeRestoreReport,
};

pub(crate) const HOSTING_RUNTIME_PROVIDER_ENV: &str = "HOSTING_RUNTIME_PROVIDER";
pub(crate) const HOSTING_RUNTIME_COOLIFY: &str = "coolify";
pub(crate) const HOSTING_RUNTIME_LIGHTWEIGHT: &str = "lightweight";
pub(crate) const HOSTING_LIGHTWEIGHT_MANAGER_BIN_ENV: &str = "HOSTING_LIGHTWEIGHT_MANAGER_BIN";
pub(crate) const HOSTING_LIGHTWEIGHT_MANAGER_CONFIG_ENV: &str =
    "HOSTING_LIGHTWEIGHT_MANAGER_CONFIG";
pub(crate) const HOSTING_LIGHTWEIGHT_TARGET_ENV: &str = "HOSTING_LIGHTWEIGHT_TARGET";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostingRuntimeKind {
    Coolify,
    Lightweight,
}

impl HostingRuntimeKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Coolify => HOSTING_RUNTIME_COOLIFY,
            Self::Lightweight => HOSTING_RUNTIME_LIGHTWEIGHT,
        }
    }

    #[must_use]
    pub fn parse(raw_value: &str) -> Option<Self> {
        match raw_value.trim().to_ascii_lowercase().as_str() {
            "" => None,
            HOSTING_RUNTIME_COOLIFY => Some(Self::Coolify),
            HOSTING_RUNTIME_LIGHTWEIGHT => Some(Self::Lightweight),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_persisted(raw_value: &str) -> Self {
        Self::parse(raw_value).unwrap_or(Self::Coolify)
    }

    #[must_use]
    pub fn from_env() -> Self {
        let Ok(raw_value) = std::env::var(HOSTING_RUNTIME_PROVIDER_ENV) else {
            return Self::Coolify;
        };

        if let Some(kind) = Self::parse(&raw_value) {
            kind
        } else {
            tracing::warn!(
                "Runtime de hosting desconocido '{}' en {}. Se usa 'coolify'.",
                raw_value,
                HOSTING_RUNTIME_PROVIDER_ENV
            );
            Self::Coolify
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostingRuntimeProvisionResult {
    pub runtime_kind: HostingRuntimeKind,
    pub deployment_id: String,
    pub public_url: String,
    pub server_ip: String,
    pub access_user: String,
    pub access_password: String,
    pub access_port: i32,
    pub wordpress_ready: bool,
    pub wordpress_install_error: Option<String>,
}

impl From<CoolifyProvisionResult> for HostingRuntimeProvisionResult {
    fn from(value: CoolifyProvisionResult) -> Self {
        Self {
            runtime_kind: HostingRuntimeKind::Coolify,
            deployment_id: value.service_uuid,
            public_url: value.domain,
            server_ip: value.server_ip,
            access_user: value.sftp_user,
            access_password: value.sftp_password,
            access_port: value.sftp_port,
            wordpress_ready: value.wordpress_ready,
            wordpress_install_error: value.wordpress_install_error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostingRuntimeDeploymentSummary {
    pub runtime_kind: HostingRuntimeKind,
    pub deployment_id: String,
    pub name: String,
    pub status: String,
    pub fqdn: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub project_id: Option<String>,
    pub environment_name: Option<String>,
}

impl From<CoolifyServiceSummary> for HostingRuntimeDeploymentSummary {
    fn from(value: CoolifyServiceSummary) -> Self {
        Self {
            runtime_kind: HostingRuntimeKind::Coolify,
            deployment_id: value.uuid,
            name: value.name,
            status: value.status,
            fqdn: value.fqdn,
            target_id: value.server_uuid,
            target_name: value.server_name,
            project_id: value.project_uuid,
            environment_name: value.environment_name,
        }
    }
}

pub struct HostingRuntimeUpdate<'a> {
    pub deployment_id: &'a str,
    pub deployment_name: &'a str,
    pub custom_domain: Option<&'a str>,
    pub access_user: &'a str,
    pub access_password: &'a str,
    pub access_port: i32,
    pub plan_config: &'a HostingPlanConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct LightweightManagerConfig {
    pub(crate) bin_path: String,
    pub(crate) config_path: Option<String>,
    pub(crate) target: String,
}

impl LightweightManagerConfig {
    pub(crate) fn configured() -> bool {
        std::env::var(HOSTING_LIGHTWEIGHT_TARGET_ENV).is_ok_and(|value| !value.trim().is_empty())
    }

    pub(crate) fn from_env() -> Result<Self, AppError> {
        let target = std::env::var(HOSTING_LIGHTWEIGHT_TARGET_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::ServiceUnavailable(format!(
                    "El runtime de hosting 'lightweight' requiere {HOSTING_LIGHTWEIGHT_TARGET_ENV} para ubicar el target."
                ))
            })?;

        let bin_path = std::env::var(HOSTING_LIGHTWEIGHT_MANAGER_BIN_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "coolify-manager".to_string());

        let config_path = std::env::var(HOSTING_LIGHTWEIGHT_MANAGER_CONFIG_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            bin_path,
            config_path,
            target,
        })
    }
}

pub struct HostingRuntimeService;

impl HostingRuntimeService {
    #[must_use]
    pub fn current_kind() -> HostingRuntimeKind {
        HostingRuntimeKind::from_env()
    }

    /* [245A-10] Las compras ya no pueden depender del provider global.
     * Gotcha: el corte actual mezcla `normal-*` en lightweight y WordPress en Coolify,
     * así que decidir por env rompería uno de los dos flujos comerciales. */
    #[must_use]
    pub fn runtime_kind_for_plan(plan: &str) -> HostingRuntimeKind {
        let normalized = plan.trim().to_ascii_lowercase();
        if normalized.starts_with("normal-") && Self::lightweight_manager_configured() {
            return HostingRuntimeKind::Lightweight;
        }

        HostingRuntimeKind::Coolify
    }

    #[must_use]
    pub fn lightweight_manager_configured() -> bool {
        LightweightManagerConfig::configured()
    }

    /* [245A-7] El runtime persistido debe gobernar las operaciones legacy.
     * Gotcha: si se resuelve por env global, cambiar a `lightweight` rompe
     * listados y controles sobre despliegues viejos aún gestionados por Coolify. */
    #[must_use]
    pub(crate) fn resolved_kind(runtime_kind: Option<HostingRuntimeKind>) -> HostingRuntimeKind {
        runtime_kind.unwrap_or_else(Self::current_kind)
    }

    #[must_use]
    pub fn deployment_name_for(subscription_id: &Uuid) -> String {
        let id_str = subscription_id.to_string();
        let short = &id_str[..8];
        format!("hosting-{short}")
    }

    pub fn require_target_config<'a>(
        coolify_config: Option<&'a CoolifyConfig>,
        operation: &str,
    ) -> Result<&'a CoolifyConfig, AppError> {
        Self::require_target_config_for(Self::current_kind(), coolify_config, operation)
    }

    pub fn require_target_config_for<'a>(
        runtime_kind: HostingRuntimeKind,
        coolify_config: Option<&'a CoolifyConfig>,
        operation: &str,
    ) -> Result<&'a CoolifyConfig, AppError> {
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                coolify_config.ok_or_else(|| {
                    AppError::ServiceUnavailable(format!(
                        "El runtime de hosting 'coolify' no está configurado para {operation}. Variables COOLIFY_* ausentes."
                    ))
                })
            }
            HostingRuntimeKind::Lightweight => {
                Err(Self::unsupported(HostingRuntimeKind::Lightweight, operation))
            }
        }
    }

    pub fn optional_target_config_for<'a>(
        runtime_kind: HostingRuntimeKind,
        coolify_config: Option<&'a CoolifyConfig>,
        operation: &str,
    ) -> Result<Option<&'a CoolifyConfig>, AppError> {
        match runtime_kind {
            HostingRuntimeKind::Coolify => Ok(Some(Self::require_target_config_for(
                runtime_kind,
                coolify_config,
                operation,
            )?)),
            HostingRuntimeKind::Lightweight => Ok(None),
        }
    }

    pub(crate) fn unsupported(runtime_kind: HostingRuntimeKind, operation: &str) -> AppError {
        AppError::ServiceUnavailable(format!(
            "El runtime de hosting '{}' aún no implementa {}.",
            runtime_kind.as_str(),
            operation
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{HostingRuntimeKind, HostingRuntimeService, HOSTING_LIGHTWEIGHT_TARGET_ENV};

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn runtime_kind_for_plan_keeps_wordpress_on_coolify() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var(HOSTING_LIGHTWEIGHT_TARGET_ENV, "vps2");

        assert_eq!(
            HostingRuntimeService::runtime_kind_for_plan("basico"),
            HostingRuntimeKind::Coolify
        );

        std::env::remove_var(HOSTING_LIGHTWEIGHT_TARGET_ENV);
    }

    #[test]
    fn runtime_kind_for_plan_uses_lightweight_for_normal_when_configured() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var(HOSTING_LIGHTWEIGHT_TARGET_ENV, "vps2");

        assert_eq!(
            HostingRuntimeService::runtime_kind_for_plan("normal-basico"),
            HostingRuntimeKind::Lightweight
        );

        std::env::remove_var(HOSTING_LIGHTWEIGHT_TARGET_ENV);
    }

    #[test]
    fn runtime_kind_for_plan_falls_back_to_coolify_without_lightweight_target() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var(HOSTING_LIGHTWEIGHT_TARGET_ENV);

        assert_eq!(
            HostingRuntimeService::runtime_kind_for_plan("normal-basico"),
            HostingRuntimeKind::Coolify
        );
    }
}
