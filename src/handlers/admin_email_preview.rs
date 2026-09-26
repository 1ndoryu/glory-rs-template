/* [311A-INV] Handler admin para previsualizar plantillas de email renderizadas.
 * Endpoints:
 *   GET /admin/email-templates       → lista de plantillas con metadatos
 *   GET /admin/email-templates/:name → HTML renderizado con datos de muestra
 * Solo accesible por admins. No envía correos reales. */

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::services::email_preview::{self, TemplateMeta};
use crate::AppState;

/// Lista de plantillas disponibles
#[derive(Serialize)]
pub struct TemplatesListResponse {
    pub templates: Vec<TemplateMeta>,
}

/// Lista todas las plantillas de email con sus metadatos
#[utoipa::path(
    get,
    path = "/admin/email-templates",
    responses(
        (status = 200, description = "Lista de plantillas", body = TemplatesListResponse),
        (status = 403, description = "Sin permisos"),
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_templates(auth: AuthUser) -> Result<Json<TemplatesListResponse>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    Ok(Json(TemplatesListResponse {
        templates: email_preview::list_templates(),
    }))
}

/// Renderiza una plantilla con datos de muestra y devuelve HTML
#[utoipa::path(
    get,
    path = "/admin/email-templates/{name}",
    params(
        ("name" = String, Path, description = "Nombre interno de la plantilla"),
    ),
    responses(
        (status = 200, description = "HTML renderizado de la plantilla", content_type = "text/html"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Plantilla no encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn render_template(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<axum::response::Html<String>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    /* [311A-INV] Usamos EmailConfig::from_env() para obtener la config actual
     * (solo necesario para from_name/from_email en el footer del template).
     * Si no hay SMTP configurado, usamos valores por defecto. */
    let config =
        state
            .email_config
            .clone()
            .unwrap_or_else(|| crate::services::email::EmailConfig {
                host: String::new(),
                port: 587,
                user: String::new(),
                pass: String::new(),
                from_name: "Nakomi Studio".to_string(),
                from_email: "noreply@nakomi.studio".to_string(),
                bcc_email: None,
            });

    let html = email_preview::render_preview(&config, &name)
        .map_err(|_| AppError::NotFound(format!("Plantilla '{name}' no encontrada")))?;

    Ok(axum::response::Html(html))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/email-templates", get(list_templates))
        .route("/admin/email-templates/:name", get(render_template))
}
