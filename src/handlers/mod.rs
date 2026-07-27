/* sentinel-disable-file limite-lineas: router central de Axum.
 * [164A-17] Sigue siendo el orquestador único de rutas/estado global del backend. */
#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

mod admin_billing;
mod admin_seo;
mod admin_client_bootstrap;
mod admin_email_preview;
mod admin_emails;
mod admin_fixtures;
mod admin_seed;
mod admin_services;
mod admin_users;
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

use argon2::PasswordHasher;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderName, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::AppState;

/// Define el esquema de seguridad Bearer para Swagger UI
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        /* components existe porque el derive ya registra schemas */
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
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
        auth::quick_register,
        auth::login,
        notes::create_note,
        notes::get_note,
        notes::list_notes,
        notes::update_note,
        notes::delete_note,
        services::list_services,
        services::get_service,
        orders::create_order,
        orders::list_orders,
        orders::get_order,
        orders::update_order_project_description_handler,
        orders::update_order_phase_definition_handler,
        orders::assign_order,
        order_lifecycle::switch_role,
        order_lifecycle::cancel_order_handler,
        order_lifecycle::approve_phase,
        order_lifecycle::request_revision,
        order_lifecycle::toggle_ai_intermediary,
        order_lifecycle::get_order_activity,
        payments::initiate_payment,
        payments::stripe_webhook,
        payments::list_payments,
        payment_methods::create_setup_intent,
        payment_methods::list_payment_methods,
        payment_methods::save_payment_method,
        payment_methods::delete_payment_method,
        assignment::take_order,
        assignment::list_unassigned,
        assignment::list_employees,
        assignment::create_delegation,
        assignment::create_help_request,
        assignment::respond_delegation,
        assignment::list_delegations,
        chat::list_sessions,
        chat::get_messages,
        chat::create_session,
        chat::send_message,
        deliverables::deliver_phase_with_files,
        deliverables::list_deliverables,
        deliverables::download_deliverable,
        refunds::request_refund,
        refunds::review_refund,
        refunds::list_refunds,
        refunds::get_order_refund,
        reviews::create_review,
        reviews::respond_review,
        reviews::get_order_review,
        reviews::list_reviews,
        notifications::list_notifications,
        notifications::get_unread_count,
        notifications::mark_read,
        notifications::mark_all_read,
        dashboard::get_dashboard,
        profile::get_profile,
        profile::upload_avatar,
        public_config::get_public_config,
        admin_users::list_users,
        admin_users::create_user,
        admin_users::change_role,
        admin_users::change_status,
        admin_users::delete_user,
        admin_fixtures::get_fixture_status,
        admin_fixtures::trigger_sync,
        admin_services::list_all,
        admin_services::create,
        admin_services::update,
        admin_services::archive,
        admin_services::destroy,
        blog::list_published,
        blog::get_by_slug,
        blog::list_all,
        blog::create,
        blog::update,
        blog::archive,
        blog::destroy,
        uploads::upload_image,
        image_proxy::image_proxy,
        projects::list_published,
        projects::get_by_slug,
        projects::list_all,
        projects::create,
        projects::update,
        projects::archive,
        projects::destroy,
        team_members::list_published,
        team_members::list_all,
        team_members::create,
        team_members::update,
        team_members::archive,
        team_members::destroy,
        public_users::get_profile,
        public_users::get_reviews_received,
        public_users::get_reviews_given,
        public_users::get_rating_distribution,
        problems::report_problem,
        problems::list_problems,
        problems::list_order_problems,
        problems::resolve_problem,
        wallet::get_balance,
        wallet::list_transactions,
        cancellation::create_cancellation_request,
        cancellation::respond_cancellation_request,
        wallet::create_withdrawal,
        wallet::list_withdrawals,
        wallet::admin_list_withdrawals,
        wallet::admin_resolve_withdrawal,
        hosting::email_aliases::get_email_info,
        hosting::email_aliases::create_alias,
        hosting::email_aliases::delete_alias,
        admin_billing::list_billing_items,
        admin_billing::update_billing_status,
    ),
    components(schemas(
        health::HealthResponse,
        crate::models::RegisterRequest,
        crate::models::QuickRegisterRequest,
        crate::models::LoginRequest,
        crate::models::AuthResponse,
        crate::models::Note,
        crate::models::CreateNoteRequest,
        crate::models::UpdateNoteRequest,
        crate::models::PaginatedNotes,
        crate::models::UserRole,
        crate::models::ServiceDetailResponse,
        crate::models::ServicePlanResponse,
        crate::models::ServicePlanPhaseResponse,
        crate::models::CreateOrderRequest,
        crate::models::OrderResponse,
        crate::models::OrderPhaseResponse,
        crate::models::SwitchRoleRequest,
        crate::models::ToggleAiIntermediaryRequest,
        crate::models::PaymentMode,
        crate::models::OrderStatus,
        crate::models::PhaseStatus,
        crate::models::PaymentStatus,
        crate::models::InitiatePaymentRequest,
        crate::models::PaymentIntentResponse,
        crate::models::PaymentResponse,
        crate::models::PaymentMethodResponse,
        crate::models::SetupIntentResponse,
        crate::models::SavePaymentMethodRequest,
        crate::models::DelegationStatus,
        crate::models::DelegationResponse,
        crate::models::EmployeeListItem,
        crate::models::CreateDelegationRequest,
        crate::models::RespondDelegationRequest,
        crate::models::ChatSession,
        crate::models::ChatMessage,
        crate::models::ChatMessageResponse,
        crate::models::ChatSessionResponse,
        crate::models::ChatAttachment,
        crate::models::VisitorProfile,
        crate::models::CreateChatSessionRequest,
        crate::models::SendMessageRequest,
        crate::models::PhaseDeliverable,
        crate::models::DeliverPhaseResponse,
        crate::models::PhaseDeliverablesResponse,
        crate::models::RefundStatus,
        crate::models::RefundResponse,
        crate::models::RequestRefundBody,
        crate::models::ReviewRefundBody,
        crate::models::ReviewAction,
        crate::models::CreateReviewBody,
        crate::models::RespondReviewBody,
        crate::models::ReviewResponse,
        crate::models::NotificationResponse,
        crate::models::UnreadCountResponse,
        crate::models::MarkReadBody,
        crate::models::WsNotification,
        crate::models::DashboardResponse,
        crate::models::RevenueStats,
        crate::models::OrderCounts,
        crate::models::EmployeePerformance,
        crate::models::DashboardAlerts,
        crate::models::AdminUserItem,
        crate::models::PaginatedUsers,
        crate::models::ChangeRoleRequest,
        crate::models::ChangeStatusRequest,
        crate::models::AdminCreateUserRequest,
        crate::models::AdminServiceResponse,
        crate::models::CreateServiceRequest,
        crate::models::UpdateServiceRequest,
        crate::models::BlogPostResponse,
        crate::models::PaginatedBlogPosts,
        crate::models::CreateBlogPostRequest,
        crate::models::UpdateBlogPostRequest,
        crate::models::ProjectResponse,
        crate::models::ProjectLink,
        crate::models::ProjectSkill,
        crate::models::CreateProjectRequest,
        crate::models::UpdateProjectRequest,
        crate::models::TeamMemberResponse,
        crate::models::CreateTeamMemberRequest,
        crate::models::UpdateTeamMemberRequest,
        crate::models::PublicUserProfile,
        crate::models::PublicReviewItem,
        crate::models::PaginatedPublicReviews,
        crate::models::RatingDistribution,
        crate::models::ProblemStatus,
        crate::models::ProblemResponse,
        crate::models::ProblemAction,
        crate::models::ReportProblemRequest,
        crate::models::ResolveProblemRequest,
        crate::models::CancelOrderRequest,
        crate::models::WalletResponse,
        crate::models::WalletTransactionResponse,
        crate::models::WalletTransactionsPage,
        crate::models::CancellationRequestResponse,
        crate::models::CreateCancellationRequest,
        crate::models::RespondCancellationRequest,
        crate::models::WithdrawalRequestResponse,
        crate::models::WithdrawalRequestsPage,
        crate::models::CreateWithdrawalRequest,
        crate::models::ResolveWithdrawalRequest,
        crate::models::HostingEmailAlias,
        crate::models::HostingEmailMailbox,
        crate::models::EmailAliasResponse,
        crate::models::EmailMailboxResponse,
        crate::models::HostingEmailInfoResponse,
        crate::models::CreateEmailAliasRequest,
        crate::models::AdminBillingItemResponse,
        crate::models::AdminUpdateBillingStatusRequest,
        order_lifecycle::ActivityEntry,
        profile::AvatarResponse,
        public_config::PublicConfigResponse,
        uploads::UploadResponse,
        crate::errors::ErrorResponse,
        admin_fixtures::FixtureStatusResponse,
        admin_fixtures::FixtureTableSummary,
        admin_fixtures::FixtureSyncResult,
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

/* [164A-16] Security headers OWASP extraídos para mantener create_router dentro del límite clippy */
type SecurityHeaderLayer = SetResponseHeaderLayer<HeaderValue>;
fn security_headers() -> (
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

fn init_contabo_service() -> Option<crate::services::ContaboService> {
    crate::services::ContaboConfig::from_env().map(|cfg| {
        tracing::info!("Contabo API configurado para {}", cfg.api_user);
        crate::services::ContaboService::new(cfg, reqwest::Client::new())
    })
}

fn init_coolify_config() -> Option<crate::services::CoolifyConfig> {
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
fn init_coolify_config_vps1() -> Option<crate::services::CoolifyConfig> {
    let cfg = crate::services::CoolifyConfig::from_env_with_prefix("COOLIFY_VPS1_");
    if cfg.is_some() {
        tracing::info!("Coolify VPS1 configurado");
    } else {
        tracing::debug!("Coolify VPS1 no configurado (opcional — faltan vars COOLIFY_VPS1_*)");
    }
    cfg
}

fn init_email_config() -> Option<crate::services::EmailConfig> {
    let email_config = crate::services::EmailConfig::from_env();
    if email_config.is_some() {
        tracing::info!("Email SMTP configurado");
    } else {
        tracing::warn!("Email SMTP NO configurado (faltan vars SMTP_*) — emails desactivados");
    }
    email_config
}

fn init_fixture_manager(
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

/// Crea el router principal con CORS, tracing, Swagger UI y todas las rutas
/* sentinel-disable-next-line funcion-larga-rs: create_router concentra wiring global de estado, middlewares y servicios opcionales para no fragmentar el bootstrap del servidor. */
#[allow(clippy::too_many_lines)]
pub fn create_router(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    let chat_hub = crate::services::ChatHub::new(pool.clone());
    let notification_hub = crate::services::NotificationHub::new(pool.clone());
    let ai_config = crate::services::AiChatConfig::from_env();
    let contabo_service = init_contabo_service();
    let coolify_config = init_coolify_config();
    let coolify_config_vps1 = init_coolify_config_vps1();
    let email_config = init_email_config();
    let fixture_manager = init_fixture_manager(&pool);

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        static_dir: config.static_dir.clone(),
        /* [114A-6] Timeout global 30s para todo request HTTP saliente.
         * Previene deadlocks cuando APIs externas se cuelgan y retienen
         * conexiones DB, agotando el pool (max 10) y congelando la app. */
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("HTTP client"),
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

    let (hsts, nosniff, frame_deny, referrer, permissions) = security_headers();

    Router::new()
        .merge(health::root_routes())
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
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
        .merge(spa_shell_routes())
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

/* [044A-9] Monta el frontend React como SPA: archivos estaticos con fallback a index.html.
 * Solo se activa si STATIC_DIR esta configurado (en produccion). En desarrollo, Vite sirve el frontend.
 * [114A-19] Cache-Control diferenciado: assets con hash → 1 año immutable, index.html → no-cache.
 * Esto mejora PageSpeed: evita re-descargar 5+ MB de JS/CSS en visitas repetidas. */
pub fn create_app(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    let static_dir = config.static_dir.clone();
    let pool_for_prerender = pool.clone();
    let router = create_router(pool, config);

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

fn spa_shell_routes() -> Router<AppState> {
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

fn api_routes() -> Router<AppState> {
    /* [064A-73][225A-4][255A-1][255A-3][176A-1] Rate limiting: detrás de Coolify/Traefik
     * la IP peer puede ser la del proxy compartido. SmartIpKeyExtractor usa los
     * headers forwarded antes de caer al peer IP y evita 429 cruzados entre usuarios.
     *
     * CORRECCIÓN 176A-1: per_second(N) = reponer 1 token cada N segundos (NO N tokens/s).
     * Para tasas altas se usa per_millisecond(). Config correcta:
     *   auth: 30/s sostenido, burst 30 — login/registro no necesitan más
     *   api:  50/s sostenido, burst 200 — SPA carga ~20 polls concurrentes */
    let auth_governor = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_millisecond(33)
        .burst_size(30)
        .finish()
        .expect("rate limit config válida");

    let api_governor = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_millisecond(20)
        .burst_size(200)
        .finish()
        .expect("rate limit config válida");

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
