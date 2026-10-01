use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

/* Respuesta mínima de Stripe Checkout Session */
#[derive(Debug, Deserialize)]
pub(crate) struct CheckoutSession {
    pub(crate) id: String,
    pub(crate) url: Option<String>,
}

/// Parámetros para crear una Checkout Session de hosting
pub struct CheckoutParams<'a> {
    pub http_client: &'a Client,
    pub stripe_key: &'a str,
    pub subscription_id: Uuid,
    pub plan: &'a str,
    pub amount_cents: i32,
    pub customer_email: &'a str,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
    pub billing_cycle_months: i32,
}

pub struct HostingStripeService;
