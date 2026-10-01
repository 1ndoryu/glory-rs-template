use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::normalize_cpu_scaling_policy;

pub(crate) fn validated_cpu_scaling_policy(
    value: Option<&str>,
) -> Result<Option<&'static str>, AppError> {
    value
        .map(|raw| {
            normalize_cpu_scaling_policy(raw).ok_or_else(|| {
                AppError::Validation(
                    "cpu_scaling_policy invalida; usa baseline_burst o contention_throttle"
                        .to_string(),
                )
            })
        })
        .transpose()
}

/* [164A-6] Struct para agrupar datos del servidor tras provisioning.
 * Evita pasar 8 argumentos sueltos a update_server_info (clippy::too_many_arguments). */
pub struct ServerInfo<'a> {
    pub runtime_kind: &'a str,
    pub deployment_id: &'a str,
    pub coolify_site_name: &'a str,
    pub server_uuid: &'a str,
    pub server_ip: &'a str,
    pub sftp_user: &'a str,
    pub sftp_password: &'a str,
    pub sftp_port: i32,
}

/* [054A-2] Parámetros agrupados para crear suscripción (evita clippy::too_many_arguments) */
pub struct CreateHostingParams<'a> {
    pub user_id: Option<Uuid>,
    pub client_name: &'a str,
    pub client_email: &'a str,
    pub plan: &'a str,
    pub domain: Option<&'a str>,
    pub domain_verification_status: &'a str,
    pub domain_verification_token: Option<&'a str>,
    pub domain_verified_at: Option<DateTime<Utc>>,
    pub runtime_kind: &'a str,
    pub deployment_id: Option<&'a str>,
    /* [304A-3] Permite vincular a despliegue Coolify existente al crear suscripción */
    pub coolify_site_name: Option<&'a str>,
    pub monthly_price_cents: i32,
    pub storage_limit_mb: i32,
}

pub struct UpdateHostingParams<'a> {
    pub plan: &'a str,
    pub domain: Option<&'a str>,
    pub monthly_price_cents: i32,
    pub storage_limit_mb: i32,
    pub domain_verification_status: &'a str,
    pub domain_verification_token: Option<&'a str>,
    pub domain_verified_at: Option<DateTime<Utc>>,
}
