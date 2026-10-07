/* [07AA-14] WordPress Coolify: install/create/finalize (split sin cambios desde coolify.rs). */

use reqwest::Client;
use tokio::time::{sleep, Duration};

use crate::errors::AppError;

use super::config::CoolifyConfig;
use super::parsing::is_normal_hosting_plan;
use super::types::{CreateServiceBody, CreateServiceResponse};

const WORDPRESS_INSTALL_POLL_ATTEMPTS: usize = 30;
const WORDPRESS_INSTALL_POLL_DELAY: Duration = Duration::from_secs(4);

pub(super) fn build_wordpress_site_title(client_name: &str, service_name: &str) -> String {
    let trimmed = client_name.trim();
    if trimmed.is_empty() {
        return format!("Sitio {service_name}");
    }

    trimmed.chars().take(80).collect()
}

pub(super) fn wordpress_install_form_is_ready(body: &str) -> bool {
    body.contains("install.php?step=2")
        && body.contains("name=\"weblog_title\"")
        && body.contains("name=\"user_name\"")
        && body.contains("name=\"admin_password\"")
}

#[allow(clippy::too_many_arguments)]
async fn install_wordpress_instance(
    http_client: &Client,
    bootstrap_url: &str,
    service_name: &str,
    client_name: &str,
    client_email: &str,
    admin_username: &str,
    admin_password: &str,
    language: &str,
) -> Result<(), String> {
    let install_step_one_url = format!("{bootstrap_url}/wp-admin/install.php?step=1");
    let install_step_two_url = format!("{bootstrap_url}/wp-admin/install.php?step=2");
    let login_url = format!("{bootstrap_url}/wp-login.php");
    let mut last_observation = "sin respuesta".to_string();

    for attempt in 0..WORDPRESS_INSTALL_POLL_ATTEMPTS {
        let response = http_client
            .get(&install_step_one_url)
            .send()
            .await
            .map_err(|error| format!("GET install.php?step=1 falló: {error}"))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("Leyendo install.php?step=1 falló: {error}"))?;

        if status.is_success() && wordpress_install_form_is_ready(&body) {
            last_observation = format!("install.php listo en intento {}", attempt + 1);
            break;
        }

        last_observation = format!(
            "intento {}: status={} ready={}",
            attempt + 1,
            status,
            wordpress_install_form_is_ready(&body)
        );

        if attempt + 1 == WORDPRESS_INSTALL_POLL_ATTEMPTS {
            return Err(format!(
                "WordPress no mostró el formulario de instalación en {install_step_one_url} ({last_observation})"
            ));
        }

        sleep(WORDPRESS_INSTALL_POLL_DELAY).await;
    }

    let form = vec![
        (
            "weblog_title".to_string(),
            build_wordpress_site_title(client_name, service_name),
        ),
        ("user_name".to_string(), admin_username.to_string()),
        ("admin_password".to_string(), admin_password.to_string()),
        ("admin_password2".to_string(), admin_password.to_string()),
        ("admin_email".to_string(), client_email.to_string()),
        ("language".to_string(), language.to_string()),
        ("Submit".to_string(), "Install WordPress".to_string()),
    ];

    let response = http_client
        .post(&install_step_two_url)
        .form(&form)
        .send()
        .await
        .map_err(|error| format!("POST install.php?step=2 falló: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Leyendo install.php?step=2 falló: {error}"))?;

    if !status.is_success() {
        return Err(format!(
            "WordPress respondió {status} al instalar en {install_step_two_url}"
        ));
    }

    if wordpress_install_form_is_ready(&body) {
        return Err("WordPress devolvió de nuevo el formulario de instalación".to_string());
    }

    if body.contains("wp-login.php")
        || body.contains("Success")
        || body.contains("success")
        || body.to_ascii_lowercase().contains("already installed")
    {
        return Ok(());
    }

    let login_response = http_client
        .get(&login_url)
        .send()
        .await
        .map_err(|error| format!("GET wp-login.php falló tras instalar: {error}"))?;

    if login_response.status().is_success() {
        return Ok(());
    }

    Err(format!(
        "WordPress no confirmó la instalación automática ({last_observation})"
    ))
}

pub(super) async fn create_hosting_service(
    http_client: &Client,
    config: &CoolifyConfig,
    service_name: &str,
    compose_b64: String,
) -> Result<CreateServiceResponse, AppError> {
    let create_body = CreateServiceBody {
        name: service_name,
        project_uuid: &config.project_uuid,
        environment_name: "production",
        server_uuid: &config.server_uuid,
        docker_compose_raw: compose_b64,
        instant_deploy: true,
    };

    let create_url = format!("{}/api/v1/services", config.base_url);
    let create_resp = http_client
        .post(&create_url)
        .bearer_auth(&config.api_token)
        .json(&create_body)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Coolify create service request failed: {e}")))?;

    if !create_resp.status().is_success() {
        let status = create_resp.status();
        let body = create_resp.text().await.unwrap_or_default();
        tracing::error!(
            "[Coolify] Error creando servicio '{}': {} — {}",
            service_name,
            status,
            body
        );
        return Err(AppError::Internal(format!(
            "Coolify create service failed: {status} — {body}"
        )));
    }

    create_resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Coolify create service parse error: {e}")))
}

pub(super) struct WordpressInstallContext<'a> {
    pub(super) service_name: &'a str,
    pub(super) service_uuid: &'a str,
    pub(super) plan_name: &'a str,
    pub(super) client_name: &'a str,
    pub(super) client_email: &'a str,
    pub(super) admin_username: &'a str,
    pub(super) admin_password: &'a str,
    pub(super) language: &'a str,
}

pub(super) async fn finalize_wordpress_install(
    http_client: &Client,
    domain: &str,
    install_context: WordpressInstallContext<'_>,
) -> (bool, Option<String>) {
    /* [165A-13][165A-14] "WordPress preinstalado" solo es cierto si el wizard de
     * `install.php` queda resuelto. Dejamos la instalación automática separada del alta
     * del servicio para que el provisioning siga siendo legible y el panel pueda auditar
     * `wordpress_ready` sin ocultar un fallo parcial de bootstrap. */
    if is_normal_hosting_plan(install_context.plan_name) {
        return (true, None);
    }

    if let Err(error) = install_wordpress_instance(
        http_client,
        domain,
        install_context.service_name,
        install_context.client_name,
        install_context.client_email,
        install_context.admin_username,
        install_context.admin_password,
        install_context.language,
    )
    .await
    {
        tracing::warn!(
            "[Coolify] WordPress quedó levantado pero sin instalación automática para '{}' (uuid={}): {}",
            install_context.service_name,
            install_context.service_uuid,
            error
        );
        return (false, Some(error));
    }

    tracing::info!(
        "[Coolify] WordPress instalado automáticamente para '{}' usando las credenciales iniciales del hosting.",
        install_context.service_name
    );
    (true, None)
}
