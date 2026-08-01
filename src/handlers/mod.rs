#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

pub mod articles;
pub mod auth;
pub mod download_handler;
mod health;
pub mod media_handler;
mod notes;
pub mod notifications;
pub mod preferences_handler;
pub mod products_handler;
pub mod projects_handler;
pub mod seo;
pub mod settings_handler;
pub mod stripe_webhook;
pub mod workspace_handler;
pub mod workspace_overlay_handler;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method};
use axum::Router;
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::AppState;

/// Define el esquema de seguridad de la sesión opaca para Swagger UI.
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        /* components existe porque el derive ya registra schemas */
        if let Some(components) = openapi.components.as_mut() {
            /* [018A-19] La cookie HttpOnly es la autoridad única; documentarla
             * como ApiKey de cookie evita que Swagger y los clientes generados
             * vuelvan a ofrecer un Bearer JWT retirado. CSRF se envía como
             * header en mutaciones y no se modela como autorización separada. */
            components.add_security_scheme(
                "session_cookie",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Cookie(
                        utoipa::openapi::security::ApiKeyValue::new("session_id"),
                    ),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::register,
        auth::login,
        auth::verify_email,
        auth::request_password_reset,
        auth::reset_password,
        preferences_handler::get_preferences,
        preferences_handler::update_preferences,
        workspace_overlay_handler::get_overlay,
        workspace_overlay_handler::update_overlay,
        notes::create_note,
        notes::get_note,
        notes::list_notes,
        notes::update_note,
        notes::delete_note,
    ),
    components(schemas(
        health::HealthResponse,
        crate::models::RegisterRequest,
        crate::models::LoginRequest,
        crate::models::RegistrationResponse,
        crate::models::VerifyEmailRequest,
        crate::models::PasswordResetRequest,
        crate::models::ConfirmPasswordResetRequest,
        crate::models::preferences::UserPreferences,
        crate::models::preferences::UpdateUserPreferencesRequest,
        crate::handlers::preferences_handler::UserPreferencesResponse,
        crate::models::workspace_overlay::WorkspaceOverlayDocument,
        crate::models::workspace_overlay::UpdateWorkspaceOverlayRequest,
        crate::handlers::workspace_overlay_handler::WorkspaceOverlayApiResponse,
        crate::models::Note,
        crate::models::CreateNoteRequest,
        crate::models::UpdateNoteRequest,
        crate::models::PaginatedNotes,
        crate::errors::ErrorResponse,
        crate::models::notification::Notification,
        crate::models::notification::NotificationList,
        crate::models::notification::CreateNotificationRequest,
        crate::models::notification::UpdateNotificationStatusRequest,
    )),
    modifiers(&SecurityAddon),
    info(
        title = "Glory RS API",
        version = "0.1.0",
        description = "Template API — Rust + Axum + OpenAPI"
    )
)]
#[allow(clippy::needless_for_each)]
pub struct ApiDoc;

/// Crea el router principal con CORS, tracing, Swagger UI y todas las rutas
pub fn create_router(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| "https://wandori.us".to_string());

    let state = AppState {
        pool,
        upload_dir: config.upload_dir,
        resend_api_key: config.resend_api_key,
        email_from: config.email_from,
        stripe_secret_key: config.stripe_secret_key,
        stripe_webhook_secret: config.stripe_webhook_secret,
        site_url,
        login_rate_limit: std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )),
    };

    /* [297A-7] CORS con allowlist de orígenes */
    let allowed_origins: Vec<HeaderValue> = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| {
            "https://wandori.us,http://localhost:5173,http://localhost:3000".to_string()
        })
        .split(',')
        .map(|s| s.trim().parse())
        .filter_map(Result::ok)
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-csrf-token"),
        ]);

    /* Servir archivos subidos estaticamente */
    /* [297A-7] Nota: uploads se mantiene público temporalmente para compatibilidad.
     * En 297A-10 se migrará a serving autorizado. */
    let uploads_service = ServeDir::new(&state.upload_dir);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api", api_routes())
        .nest_service("/uploads", uploads_service)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(DefaultBodyLimit::max(20 * 1024 * 1024)) /* 20MB para uploads de media */
                .into_inner(),
        )
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    /* [297A-14] Sintaxis de rutas: este build (axum 0.7.9 + matchit 0.7.3)
     * parsea parámetros con `:param`, NO con `{param}` (el `{id}` de la doc
     * de axum 0.7 devuelve 404 silencioso). Usar SIEMPRE `:id` en `.route()`;
     * los atributos `utoipa::path` sí conservan `{id}` (formato OpenAPI). */
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(notes::routes())
        .merge(articles::routes())
        .merge(media_handler::routes())
        .merge(notifications::routes())
        .merge(download_handler::routes())
        .merge(preferences_handler::routes())
        .merge(settings_handler::routes())
        .merge(products_handler::routes())
        .merge(projects_handler::routes())
        .merge(seo::routes())
        .merge(stripe_webhook::routes())
        .merge(workspace_handler::routes())
        .merge(workspace_overlay_handler::routes())
}
