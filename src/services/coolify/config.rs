/* [07AA-14] Configuración Coolify (split sin cambios de lógica desde coolify.rs). */

/* ============================================================
CONFIGURACIÓN
============================================================ */

/// Configuración de Coolify cargada desde variables de entorno
#[derive(Debug, Clone)]
pub struct CoolifyConfig {
    pub base_url: String,
    pub api_token: String,
    pub server_uuid: String,
    pub project_uuid: String,
    /// IP pública del servidor Coolify (usada como `server_ip` de las suscripciones)
    pub server_ip: String,
    /// [114A-15+] Ruta a la clave SSH para acceso al VPS (docker stats, monitoreo)
    pub ssh_key_path: Option<String>,
}

impl CoolifyConfig {
    /// Carga config desde variables de entorno COOLIFY_*.
    /// Retorna None si alguna variable requerida está ausente.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("COOLIFY_BASE_URL").ok()?;
        let api_token = std::env::var("COOLIFY_API_TOKEN").ok()?;
        let server_uuid = std::env::var("COOLIFY_SERVER_UUID").ok()?;
        let project_uuid = std::env::var("COOLIFY_PROJECT_UUID").ok()?;
        let server_ip = std::env::var("COOLIFY_SERVER_IP").ok()?;

        if base_url.is_empty()
            || api_token.is_empty()
            || server_uuid.is_empty()
            || project_uuid.is_empty()
            || server_ip.is_empty()
        {
            return None;
        }

        /* [114A-15+] SSH key opcional para docker stats / monitoreo */
        let ssh_key_path = std::env::var("COOLIFY_SSH_KEY_PATH")
            .ok()
            .filter(|s| !s.is_empty());

        Some(Self {
            base_url,
            api_token,
            server_uuid,
            project_uuid,
            server_ip,
            ssh_key_path,
        })
    }

    /* [VPS1-support] Carga config desde env vars con prefijo arbitrario
     * (`COOLIFY_VPS1_`, `COOLIFY_VPS2_`, etc.) para permitir varios destinos. */
    #[must_use]
    pub fn from_env_with_prefix(prefix: &str) -> Option<Self> {
        let base_url = std::env::var(format!("{prefix}BASE_URL")).ok()?;
        let api_token = std::env::var(format!("{prefix}API_TOKEN")).ok()?;
        let server_uuid = std::env::var(format!("{prefix}SERVER_UUID")).ok()?;
        let project_uuid = std::env::var(format!("{prefix}PROJECT_UUID")).ok()?;
        let server_ip = std::env::var(format!("{prefix}SERVER_IP")).ok()?;

        if base_url.is_empty()
            || api_token.is_empty()
            || server_uuid.is_empty()
            || project_uuid.is_empty()
            || server_ip.is_empty()
        {
            return None;
        }

        let ssh_key_path = std::env::var(format!("{prefix}SSH_KEY_PATH"))
            .ok()
            .filter(|s| !s.is_empty());

        Some(Self {
            base_url,
            api_token,
            server_uuid,
            project_uuid,
            server_ip,
            ssh_key_path,
        })
    }
}
