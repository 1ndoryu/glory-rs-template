use axum::routing::get;
use axum::Router;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;

use super::backups::{create_backup, delete_backup, list_backups, restore_backup};
use super::checkout::{admin_test_subscribe, create_checkout, subscribe_self};
use super::control::{restart_hosting, start_hosting, stop_hosting};
use super::deployments::{delete_deployment, list_deployments};
use super::domain::{dns_check, verify_domain};
use super::email_aliases::{create_alias, delete_alias, get_email_info};
use super::infrastructure::{
    deployment_metrics, list_infrastructure_servers, refresh_infrastructure_metrics,
    resource_usage_report,
};
use super::plans::{list_plan_configs, list_public_plans, update_plan_config};
use super::provisioning::{provision_subscription, refresh_hosting, rotate_credentials};
use super::stats::get_hosting_stats;
use super::subscriptions::{
    assign_hosting, create_subscription, delete_subscription, get_subscription, list_events,
    list_subscriptions, request_cancel, update_status, update_subscription,
};
use super::vps::{get_vps, list_vps};
use crate::AppState;

/* [01AA-4-F1] Los dos limiters de checkout/subscribe comparten config
 * (1 req/s, burst 5): un solo builder para no duplicar ~18 líneas. */
fn rate_limit_governor() -> GovernorConfig<SmartIpKeyExtractor> {
    /* [259A-1] Config fija valida por construccion; si falla es error de
     * programacion en arranque: salida explicita, nunca panic en produccion. */
    GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_second(1)
        .burst_size(5)
        .finish()
        .unwrap_or_else(|| {
            eprintln!("[fatal] subscribe rate limit config invalida");
            std::process::exit(1);
        })
}

fn subscription_routes() -> Router<AppState> {
    /* [255A-1] Checkout/suscripción también debe usar la IP real del cliente.
     * Si se limita por la IP interna del proxy, un pico ajeno puede bloquear compras.
     *
     * [176A-1] Corregido: per_second(N) = 1 token cada N segundos.
     * Checkout: 1 req/s, burst 5 — prevenir abuso sin bloquear compras legítimas. */
    let subscribe_gov = rate_limit_governor();
    let checkout_gov = rate_limit_governor();

    Router::new()
        .route(
            "/hosting/subscriptions",
            get(list_subscriptions).post(create_subscription),
        )
        .route(
            "/hosting/subscriptions/:id",
            get(get_subscription)
                .put(update_subscription)
                .delete(delete_subscription),
        )
        .route(
            "/hosting/subscriptions/:id/status",
            axum::routing::patch(update_status),
        )
        .route(
            "/hosting/subscriptions/:id/backups",
            get(list_backups).post(create_backup).delete(delete_backup),
        )
        .route(
            "/hosting/subscriptions/:id/assign",
            axum::routing::patch(assign_hosting),
        )
        .route("/hosting/subscriptions/:id/events", get(list_events))
        .route("/hosting/subscriptions/:id/stats", get(get_hosting_stats))
        .route(
            "/hosting/subscriptions/:id/cancel",
            axum::routing::post(request_cancel),
        )
        .route(
            "/hosting/subscriptions/:id/checkout",
            axum::routing::post(create_checkout).layer(GovernorLayer {
                config: std::sync::Arc::new(checkout_gov),
            }),
        )
        .route(
            "/hosting/subscribe",
            axum::routing::post(subscribe_self).layer(GovernorLayer {
                config: std::sync::Arc::new(subscribe_gov),
            }),
        )
        .route(
            "/hosting/subscriptions/:id/provision",
            axum::routing::post(provision_subscription),
        )
        .route(
            "/hosting/subscriptions/:id/rotate-credentials",
            axum::routing::post(rotate_credentials),
        )
        .route(
            "/hosting/subscriptions/:id/verify-domain",
            axum::routing::post(verify_domain),
        )
        .route("/hosting/subscriptions/:id/dns-check", get(dns_check))
        .route(
            "/hosting/subscriptions/:id/refresh",
            axum::routing::post(refresh_hosting),
        )
        .route(
            "/hosting/subscriptions/:id/restore",
            axum::routing::post(restore_backup),
        )
        .route(
            "/hosting/subscriptions/:id/restart",
            axum::routing::post(restart_hosting),
        )
        .route(
            "/hosting/subscriptions/:id/stop",
            axum::routing::post(stop_hosting),
        )
        .route(
            "/hosting/subscriptions/:id/start",
            axum::routing::post(start_hosting),
        )
        .route(
            "/hosting/admin-test-subscribe",
            axum::routing::post(admin_test_subscribe),
        )
        /* [265A-11] Alias de correo: listar, crear, eliminar */
        .route("/hosting/subscriptions/:id/email", get(get_email_info))
        .route(
            "/hosting/subscriptions/:id/email/aliases",
            axum::routing::post(create_alias),
        )
        .route(
            "/hosting/subscriptions/:id/email/aliases/:alias_id",
            axum::routing::delete(delete_alias),
        )
}

fn deployment_routes() -> Router<AppState> {
    Router::new()
        .route("/hosting/deployments", get(list_deployments))
        .route("/infrastructure/deployments", get(list_deployments))
        .route(
            "/hosting/deployments/:deployment_uuid/metrics",
            get(deployment_metrics),
        )
        .route(
            "/infrastructure/deployments/:deployment_uuid/metrics",
            get(deployment_metrics),
        )
        .route(
            "/hosting/deployments/:deployment_uuid",
            axum::routing::delete(delete_deployment),
        )
}

fn infrastructure_routes() -> Router<AppState> {
    Router::new()
        .route("/infrastructure/servers", get(list_infrastructure_servers))
        .route(
            "/infrastructure/metrics/refresh",
            axum::routing::post(refresh_infrastructure_metrics),
        )
        .route(
            "/infrastructure/resource-report",
            get(resource_usage_report),
        )
        .route("/hosting/vps", get(list_vps))
        .route("/hosting/vps/:instance_id", get(get_vps))
}

fn plan_routes() -> Router<AppState> {
    Router::new()
        .route("/hosting/plan-configs", get(list_plan_configs))
        .route("/hosting/public-plans", get(list_public_plans))
        .route(
            "/hosting/plan-configs/:plan",
            axum::routing::put(update_plan_config),
        )
}

#[allow(clippy::too_many_lines)]
pub fn hosting_routes() -> Router<AppState> {
    Router::new()
        .merge(subscription_routes())
        .merge(deployment_routes())
        .merge(infrastructure_routes())
        .merge(plan_routes())
}
