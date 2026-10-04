/* [01AA-4-f3p] Fachada de dominios Contabo (extraido de hosting_domains.rs).
 * availability = disponibilidad + checkout; domains = CRUD + auth-code;
 * handles = contactos WHOIS; dns = zonas + registros admin;
 * client_dns = registros por suscripción. */

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

mod availability;
mod client_dns;
mod dns;
mod domains;
mod handles;
mod helpers;

pub fn domain_routes() -> Router<AppState> {
    Router::new()
        /* Dominios */
        .route(
            "/hosting/domains/check/:domain",
            get(availability::check_domain_availability),
        )
        .route(
            "/hosting/domain-orders",
            get(availability::list_domain_orders),
        )
        .route(
            "/hosting/domains/checkout",
            post(availability::create_domain_checkout),
        )
        .route(
            "/hosting/domains",
            get(domains::list_domains).post(domains::order_domain),
        )
        .route(
            "/hosting/domains/:domain",
            get(domains::get_domain).patch(domains::update_domain),
        )
        .route(
            "/hosting/domains/:domain/cancel",
            axum::routing::post(domains::cancel_domain),
        )
        .route(
            "/hosting/domains/:domain/auth-code",
            axum::routing::post(domains::get_auth_code),
        )
        /* Handles */
        .route(
            "/hosting/handles",
            get(handles::list_handles).post(handles::create_handle),
        )
        /* DNS Zones (admin) */
        .route(
            "/hosting/dns/zones",
            get(dns::list_dns_zones).post(dns::create_dns_zone),
        )
        .route(
            "/hosting/dns/zones/:zone",
            axum::routing::delete(dns::delete_dns_zone),
        )
        /* DNS Records (admin) */
        .route(
            "/hosting/dns/zones/:zone/records",
            get(dns::list_dns_records).post(dns::create_dns_record),
        )
        .route(
            "/hosting/dns/zones/:zone/records/:record_id",
            axum::routing::patch(dns::update_dns_record).delete(dns::delete_dns_record),
        )
        /* DNS Records (client — por suscripción) */
        .route(
            "/hosting/subscriptions/:sub_id/dns",
            get(client_dns::client_list_dns_records).post(client_dns::client_create_dns_record),
        )
        .route(
            "/hosting/subscriptions/:sub_id/dns/:record_id",
            axum::routing::patch(client_dns::client_update_dns_record)
                .delete(client_dns::client_delete_dns_record),
        )
}
