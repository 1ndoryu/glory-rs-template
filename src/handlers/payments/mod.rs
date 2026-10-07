/* sentinel-disable-file sqlx-query-sin-macro: payment handler usa runtime query para
 * UPDATE de estado tras webhook Stripe (tipo dinámico por contexto). */
/* [044A-38 Fase 3] Handlers de pagos: Stripe checkout, webhook, historial.
 * Webhook no requiere auth — se verifica con firma HMAC-SHA256. */

/* [01AA-4-f3n] Partido por area: handlers/payments.rs 610L superaba limite.
 * checkout = initiate + checkout-intent + historial; webhook = stripe_webhook
 * con su waiver funcion-larga-rs; routes() queda aqui como tabla de rutas.
 * Superficie externa intacta (handlers/mod.rs usa initiate/stripe_webhook/
 * list_payments/routes). Pendiente: deployments, hosting_domains,
 * ai_tools_misc, main + resto F3r/F3s + Fase 4. */

pub mod checkout;
pub mod webhook;

pub use checkout::{create_checkout_intent, initiate_payment, list_payments};
pub use webhook::stripe_webhook;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders/:order_id/pay", post(initiate_payment))
        .route("/orders/:order_id/payments", get(list_payments))
        .route("/webhooks/stripe", post(stripe_webhook))
        .route("/checkout/intent", post(create_checkout_intent))
}
