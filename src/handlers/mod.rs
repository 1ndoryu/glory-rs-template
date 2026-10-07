/* [07AA-14] Router central adelgazado: solo declara módulos, `create_router` y
 * `api_routes`. El documento OpenAPI vive en `openapi_doc.rs` y el bootstrap
 * (security headers, inits, `create_app`, SPA shell) en `app.rs`.
 * [164A-17] Sigue siendo el orquestador único de rutas/estado global del backend. */

mod admin_billing;
mod admin_client_bootstrap;
mod admin_email_preview;
mod admin_emails;
mod admin_fixtures;
mod admin_seed;
mod admin_seo;
mod admin_services;
mod admin_users;
mod app;
mod assignment;
mod auth;
mod billing;
mod blog;
mod cancellation;
mod chat;
mod dashboard;
mod deliverables;
mod health;
mod hosting;
mod hosting_domains;
mod image_proxy;
mod notes;
mod notifications;
mod openapi_doc;
mod order_lifecycle;
mod orders;
mod payment_methods;
mod payments;
mod problems;
mod profile;
mod projects;
mod public_config;
mod public_users;
mod refunds;
mod reviews;
mod seo;
mod services;
mod team_members;
mod uploads;
mod vps;
mod wallet;

use axum::http::{HeaderName, HeaderValue, Method};
use axum::Router;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::AppState;

pub use app::create_app;

/* [07AA-14] SecurityAddon + ApiDoc movidos a `openapi_doc.rs`. */

/* [07AA-14] security_headers + inits movidos a `app.rs`. */

/// Crea el router principal con CORS, tracing, Swagger UI y todas las rutas
/* sentinel-disable-next-line funcion-larga-rs: create_router concentra wiring global de estado, middlewares y servicios opcionales para no fragmentar el bootstrap del servidor. */
#[allow(clippy::too_many_lines)]
pub fn create_router(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    let chat_hub = crate::services::ChatHub::new(pool.clone());
    let notification_hub = crate::services::NotificationHub::new(pool.clone());
    let ai_config = crate::services::AiChatConfig::from_env();
    let contabo_service = app::init_contabo_service();
    let coolify_config = app::init_coolify_config();
    let coolify_config_vps1 = app::init_coolify_config_vps1();
    let email_config = app::init_email_config();
    let fixture_manager = app::init_fixture_manager(&pool);

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        static_dir: config.static_dir.clone(),
        /* [114A-6] Timeout global 30s para cada request HTTP saliente.
         * Previene deadlocks cuando APIs externas se cuelgan y retienen
         * conexiones DB, agotando el pool (max 10) y congelando la app. */
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            /* [259A-1] Arranque: builder con valores fijos; fallo = salida
             * explicita, nunca panic en produccion. */
            .unwrap_or_else(|e| {
                eprintln!("[fatal] no se pudo construir HTTP client: {e}");
                std::process::exit(1);
            }),
        stripe_publishable_key: config.stripe_publishable_key,
        stripe_secret_key: config.stripe_secret_key,
        stripe_webhook_secret: config.stripe_webhook_secret,
        chat_hub,
        ai_config,
        notification_hub,
        chat_timing: crate::services::ChatTimingService::new(),
        contabo_service,
        coolify_config,
        coolify_config_vps1,
        email_config,
        docker_stats_cache: crate::services::docker_stats::DockerStatsCache::new(),
        fixture_manager,
    };

    /* [064A-73] CORS: restringir orígenes en producción. Si GLORY_ALLOWED_ORIGINS vacío, allow all (dev). */
    let cors = if config.allowed_origins.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<HeaderValue> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("authorization"),
            ])
            .allow_credentials(true)
    };

    let (hsts, nosniff, frame_deny, referrer, permissions) = app::security_headers();

    Router::new()
        .merge(health::root_routes())
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", openapi_doc::ApiDoc::openapi()),
        )
        .merge(seo::routes())
        /* [044A-38 Fase 5] WebSocket routes at root level (not under /api) */
        .merge(chat::ws_routes())
        /* [044A-38 Fase 9] WebSocket de notificaciones en tiempo real */
        .merge(notifications::ws_routes())
        /* [044A-43] Servir archivos estáticos de uploads/ (avatares, etc.)
         * [154A-6] Cache agresivo: uploads no cambian una vez subidos (1 año) */
        .nest_service(
            "/uploads",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    HeaderName::from_static("cache-control"),
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("uploads")),
        )
        /* [235A-2][235A-3] Montar /api/img como ruta absoluta en el router raíz.
         * Con dos `nest("/api", ...)` Axum seguía dejando el proxy bajo el árbol
         * rate-limited en producción. La ruta debe vivir fuera de api_routes() de
         * forma estructural, no solo en un router anidado paralelo. */
        .merge(image_proxy::routes())
        .nest("/api", api_routes())
        .merge(app::spa_shell_routes())
        .layer(TraceLayer::new_for_http())
        /* [255A-1] Canonizar rutas publicas con slash final (`/panel/` -> `/panel`).
         * Axum caia al SPA fallback estatico y devolvia `index.html` con estado 404.
         * La normalizacion corrige el caso de forma estructural para toda la app. */
        .layer(NormalizePathLayer::trim_trailing_slash())
        /* [154A-6] Compresión HTTP gzip+brotli — reduce transferencia ~70% */
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(hsts)
        .layer(nosniff)
        .layer(frame_deny)
        .layer(referrer)
        .layer(permissions)
        .with_state(state)
}

/* [07AA-14] create_app + SPA shell movidos a `app.rs` (re-exportado arriba). */

fn api_routes() -> Router<AppState> {
    /* [064A-73][225A-4][255A-1][255A-3][176A-1] Rate limiting: detrás de Coolify/Traefik
     * la IP peer puede ser la del proxy compartido. SmartIpKeyExtractor usa los
     * headers forwarded antes de caer al peer IP y evita 429 cruzados entre usuarios.
     *
     * CORRECCIÓN 176A-1: per_second(N) = reponer 1 token cada N segundos (NO N tokens/s).
     * Para tasas altas se usa per_millisecond(). Config correcta:
     *   auth: 30/s sostenido, burst 30 — login/registro no necesitan más
     *   api:  50/s sostenido, burst 200 — SPA carga ~20 polls concurrentes */
    /* [259A-1] Arranque: configs fijas validas por construccion; fallo =
     * salida explicita, nunca panic en produccion. */
    let auth_governor = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_millisecond(33)
        .burst_size(30)
        .finish()
        .unwrap_or_else(|| {
            eprintln!("[fatal] auth rate limit config invalida");
            std::process::exit(1);
        });

    let api_governor = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_millisecond(20)
        .burst_size(200)
        .finish()
        .unwrap_or_else(|| {
            eprintln!("[fatal] api rate limit config invalida");
            std::process::exit(1);
        });

    let auth_routes = auth::routes().layer(GovernorLayer {
        config: std::sync::Arc::new(auth_governor),
    });

    Router::new()
        .merge(health::routes())
        .merge(auth_routes)
        .merge(notes::routes())
        .merge(services::routes())
        /* assignment routes ANTES de orders: /orders/unassigned (literal) debe
         * registrarse antes de /orders/:order_id (parámetro) para evitar conflicto */
        .merge(assignment::routes())
        .merge(orders::routes())
        .merge(order_lifecycle::routes())
        .merge(payments::routes())
        .merge(billing::routes())
        .merge(payment_methods::routes())
        .merge(chat::rest_routes())
        .merge(deliverables::routes())
        .merge(refunds::routes())
        .merge(reviews::routes())
        .merge(notifications::routes())
        .merge(public_config::routes())
        .merge(dashboard::routes())
        .merge(profile::routes())
        .merge(admin_users::routes())
        .merge(admin_billing::routes())
        .merge(admin_services::routes())
        .merge(blog::public_routes())
        .merge(blog::admin_routes())
        .merge(projects::public_routes())
        .merge(projects::admin_routes())
        .merge(team_members::public_routes())
        .merge(team_members::admin_routes())
        .merge(public_users::public_routes())
        .merge(admin_client_bootstrap::routes())
        .merge(admin_email_preview::routes())
        .merge(admin_emails::routes())
        .merge(admin_fixtures::routes())
        .merge(admin_seed::seed_routes())
        .merge(hosting::hosting_routes())
        .merge(vps::routes())
        .merge(hosting_domains::domain_routes())
        .merge(problems::routes())
        .merge(wallet::wallet_routes())
        .merge(cancellation::cancellation_routes())
        .merge(wallet::withdrawal_admin_routes())
        .merge(admin_seo::routes())
        .merge(uploads::routes())
        .layer(GovernorLayer {
            config: std::sync::Arc::new(api_governor),
        })
}
