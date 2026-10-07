/* [07AA-14] Bootstrap de la app extraído del router central (`mod.rs`):
 * security headers OWASP, inits de servicios opcionales
 * (Contabo/Coolify/Email/fixtures), `create_app` (SPA + prerender SEO)
 * y rutas shell SPA. `mod.rs` conserva el orquestador (`create_router` +
 * `api_routes`); el documento OpenAPI vive en `openapi_doc.rs`. */

use argon2::PasswordHasher;
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::AppState;

/* [164A-16] Security headers OWASP extraídos para mantener create_router dentro del límite clippy */
pub(crate) type SecurityHeaderLayer = SetResponseHeaderLayer<HeaderValue>;
pub(crate) fn security_headers() -> (
    SecurityHeaderLayer,
    SecurityHeaderLayer,
    SecurityHeaderLayer,
    SecurityHeaderLayer,
    SecurityHeaderLayer,
) {
    let hsts = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    let nosniff = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    let frame_deny = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    let referrer = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    let permissions = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    (hsts, nosniff, frame_deny, referrer, permissions)
}

pub(crate) fn init_contabo_service() -> Option<crate::services::ContaboService> {
    crate::services::ContaboConfig::from_env().map(|cfg| {
        tracing::info!("Contabo API configurado para {}", cfg.api_user);
        crate::services::ContaboService::new(cfg, reqwest::Client::new())
    })
}

pub(crate) fn init_coolify_config() -> Option<crate::services::CoolifyConfig> {
    let coolify_config = crate::services::CoolifyConfig::from_env();
    if coolify_config.is_some() {
        tracing::info!("Coolify provisioning configurado");
    } else {
        tracing::warn!(
            "Coolify NO configurado — provisioning de hosting desactivado (faltan vars COOLIFY_*)"
        );
    }
    coolify_config
}

/* [VPS1-support] Config de Coolify para la VPS principal, vars COOLIFY_VPS1_* */
pub(crate) fn init_coolify_config_vps1() -> Option<crate::services::CoolifyConfig> {
    let cfg = crate::services::CoolifyConfig::from_env_with_prefix("COOLIFY_VPS1_");
    if cfg.is_some() {
        tracing::info!("Coolify VPS1 configurado");
    } else {
        tracing::debug!("Coolify VPS1 no configurado (opcional — faltan vars COOLIFY_VPS1_*)");
    }
    cfg
}

pub(crate) fn init_email_config() -> Option<crate::services::EmailConfig> {
    let email_config = crate::services::EmailConfig::from_env();
    if email_config.is_some() {
        tracing::info!("Email SMTP configurado");
    } else {
        tracing::warn!("Email SMTP NO configurado (faltan vars SMTP_*) — emails desactivados");
    }
    email_config
}

pub(crate) fn init_fixture_manager(
    pool: &sqlx::PgPool,
) -> Option<std::sync::Arc<glory_rs::fixtures::ContentManager>> {
    let content_dir = std::env::var("CONTENT_DIR").unwrap_or_else(|_| "content".to_string());
    if std::path::Path::new(&content_dir).exists() {
        tracing::info!("Fixture manager configurado en '{content_dir}'");
        let password_hasher: glory_rs::fixtures::PasswordHasher = Box::new(|plain| {
            let salt = argon2::password_hash::SaltString::generate(
                &mut argon2::password_hash::rand_core::OsRng,
            );
            let hash = argon2::Argon2::default()
                .hash_password(plain.as_bytes(), &salt)
                .map_err(
                    |e: argon2::password_hash::Error| -> Box<dyn std::error::Error + Send + Sync> {
                        e.to_string().into()
                    },
                )?
                .to_string();
            Ok(hash)
        });
        Some(std::sync::Arc::new(
            glory_rs::fixtures::ContentManager::new(pool.clone(), &content_dir)
                .with_password_hasher(password_hasher),
        ))
    } else {
        tracing::warn!("Content dir '{content_dir}' no encontrado — fixture sync desactivado");
        None
    }
}

/* [044A-9] Monta el frontend React como SPA: archivos estaticos con fallback a index.html.
 * Solo se activa si STATIC_DIR esta configurado (en produccion). En desarrollo, Vite sirve el frontend.
 * [114A-19] Cache-Control diferenciado: assets con hash → 1 año immutable, index.html → no-cache.
 * Esto mejora PageSpeed: evita re-descargar 5+ MB de JS/CSS en visitas repetidas. */
pub fn create_app(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    let static_dir = config.static_dir.clone();
    let pool_for_prerender = pool.clone();
    let router = super::create_router(pool, config);

    if let Some(dir) = static_dir {
        let index_path = format!("{dir}/index.html");
        let assets_dir = format!("{dir}/assets");

        /* [114A-19] Assets con content-hash de Vite (ej: index-B5XA6NlK.js): cache 1 año immutable.
         * El hash cambia cada vez que el contenido cambia, así que es seguro. */
        let asset_service = tower::ServiceBuilder::new()
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("cache-control"),
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ))
            .service(ServeDir::new(&assets_dir));

        /* SPA fallback: index.html se revalida siempre para apuntar a los assets más recientes.
         * Otros archivos sin hash (favicon, fonts) obtienen 1 día de cache. */
        let spa_serve = ServeDir::new(&dir).not_found_service(ServeFile::new(&index_path));

        /* [214A-2] Middleware SEO dinámico: inyecta meta tags desde BD para crawlers.
         * Reemplaza el enfoque estático de 114A-SEO3 (Puppeteer) que no soportaba CMS editable. */
        let prerender_state = crate::middleware::prerender::PrerenderState {
            pool: pool_for_prerender,
            static_dir: dir.clone(),
            app_url: std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:5173".into()),
            seo_cache: crate::middleware::prerender::SeoCache::new(),
        };

        /* [185A-1] CompressionLayer aqui cubre /assets/ y SPA fallback (HTML).
         * El CompressionLayer de create_router solo cubre /api/ y /uploads/.
         * Los nest_service/fallback_service en create_app quedan fuera de ese layer.
         * tower-http no recomprime si Content-Encoding ya esta establecido. */
        router
            .nest_service("/assets", asset_service)
            .fallback_service(spa_serve)
            .layer(axum::middleware::from_fn_with_state(
                prerender_state,
                crate::middleware::prerender::prerender,
            ))
            .layer(CompressionLayer::new())
    } else {
        router
    }
}

pub(crate) fn spa_shell_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(spa_index))
        .route("/servicios", get(spa_index))
        .route("/servicios/:slug", get(spa_index))
        .route("/proyectos", get(spa_index))
        .route("/proyectos/:slug", get(spa_index))
        .route("/nosotros", get(spa_index))
        .route("/soluciones/hosting-wordpress", get(spa_index))
        .route("/soluciones/hosting", get(spa_index))
        .route("/soluciones/vps", get(spa_index))
        .route("/portal-vps", get(spa_index))
        .route("/politica-privacidad", get(spa_index))
        .route("/blog", get(spa_index))
        .route("/blog/:slug", get(spa_index))
        .route("/contacto", get(spa_index))
        .route("/usuario/:username", get(spa_index))
        .route("/panel", get(spa_index))
        .route("/panel/", get(spa_index))
        .route("/panel/chat", get(spa_index))
}

async fn spa_index(State(state): State<AppState>) -> Response {
    let Some(static_dir) = state.static_dir.as_deref() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let index_path = format!("{static_dir}/index.html");
    match tokio::fs::read(index_path).await {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(error) => {
            tracing::error!(%error, static_dir, "No se pudo leer index.html para SPA");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}
