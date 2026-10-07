/* [07AA-14] Tipos API Coolify (split sin cambios de lógica desde coolify.rs). */

use serde::{Deserialize, Serialize};

use crate::models::HostingPlanConfig;

/* ============================================================
TIPOS DE API
============================================================ */

/// Body de la llamada POST /api/v1/services (`docker_compose_raw` en base64).
/// `instant_deploy` evita un 500 opaco en Coolify al crear stacks compose.
#[derive(Debug, Serialize)]
pub(super) struct CreateServiceBody<'a> {
    pub(super) name: &'a str,
    pub(super) project_uuid: &'a str,
    pub(super) environment_name: &'static str,
    pub(super) server_uuid: &'a str,
    pub(super) docker_compose_raw: String,
    pub(super) instant_deploy: bool,
}

/// Respuesta de POST /api/v1/services (solo campos que usamos)
#[derive(Debug, Deserialize)]
pub(super) struct CreateServiceResponse {
    pub(super) uuid: String,
    pub(super) domains: Vec<String>,
}

/// Resultado de provisionar un hosting
#[derive(Debug, Clone)]
pub struct CoolifyProvisionResult {
    /// UUID del servicio en Coolify (para gestión posterior: suspend, delete)
    pub service_uuid: String,
    /// Dominio sslip.io asignado por Coolify
    pub domain: String,
    /// IP del servidor VPS (vendrá del `CoolifyConfig`)
    pub server_ip: String,
    /// Usuario SFTP generado para acceso a archivos del sitio
    pub sftp_user: String,
    /// Contraseña SFTP generada aleatoriamente
    pub sftp_password: String,
    /// Puerto del host mapeado al contenedor SFTP (único por hosting)
    pub sftp_port: i32,
    /// true si el stack `WordPress` quedó instalado y listo para el primer login.
    pub wordpress_ready: bool,
    /// Error no-fatal del instalador de `WordPress`, si la automatización no completó.
    pub wordpress_install_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HostingProvisionPreferences {
    pub wp_admin_username: Option<String>,
    pub wp_admin_password: Option<String>,
    pub wp_language: Option<String>,
    pub sftp_user: Option<String>,
    pub sftp_password: Option<String>,
}

pub struct HostingComposeUpdate<'a> {
    pub service_uuid: &'a str,
    pub service_name: &'a str,
    pub custom_domain: Option<&'a str>,
    pub sftp_user: &'a str,
    pub sftp_password: &'a str,
    pub sftp_port: i32,
    pub plan_config: &'a HostingPlanConfig,
}

/* [164A-19] Resumen de servicios reales devueltos por Coolify.
 * Se usa para poblar el panel admin con despliegues reales, no con la lista
 * de instancias del proveedor. Los campos opcionales vienen de la API y no siempre
 * están presentes según la versión de Coolify o el tipo de servicio. */
#[derive(Debug, Clone)]
pub struct CoolifyServiceSummary {
    pub uuid: String,
    pub name: String,
    pub status: String,
    pub fqdn: Option<String>,
    pub server_uuid: Option<String>,
    pub server_name: Option<String>,
    pub project_uuid: Option<String>,
    pub environment_name: Option<String>,
}

/* ============================================================
SERVICIO
============================================================ */

pub struct CoolifyService;
