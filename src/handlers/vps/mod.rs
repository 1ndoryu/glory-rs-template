/* [07AA-14] Fachada VPS extraída de `vps.rs` (708 líneas, god-object):
 * catálogo público, lectura de suscripciones, auto-suscripción con checkout
 * Stripe y aprobación/rechazo con provisioning Contabo.
 * [164A-17] Handlers de reventa VPS. Separados de hosting compartido para no
 * mezclar inventario bruto de Contabo con ventas reales. El flujo es:
 * pending_payment -> pending_approval -> provisioning -> active / rejected. */

mod approval;
mod catalog;
mod subscribe;
mod subscriptions;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/vps/public-plans", get(catalog::list_public_plans))
        .route("/vps/subscriptions", get(subscriptions::list_subscriptions))
        .route(
            "/vps/subscriptions/:id",
            get(subscriptions::get_subscription),
        )
        .route("/vps/subscribe", post(subscribe::subscribe_self))
        .route(
            "/admin/vps/subscriptions/:id/approve",
            post(approval::approve_subscription),
        )
        .route(
            "/admin/vps/subscriptions/:id/reject",
            post(approval::reject_subscription),
        )
}
