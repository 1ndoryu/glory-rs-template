/* wandori.us — Stripe Webhook Handler
 * Verifica la firma HMAC-SHA256 de Stripe y procesa eventos de pago.
 * Evento principal: checkout.session.completed → marca orden como pagada, envia email de descarga. */

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::errors::AppError;
use crate::repositories::product_repo::{OrderRepository, ProductRepository};
use crate::services::email::EmailService;
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Verifica la firma del webhook de Stripe usando HMAC-SHA256.
/// Formato del header: `t=timestamp,v1=signature`
fn verify_stripe_signature(
    payload: &str,
    signature_header: &str,
    webhook_secret: &str,
) -> Result<(), AppError> {
    let mut timestamp = "";
    let mut signature = "";

    for part in signature_header.split(',') {
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = t;
        } else if let Some(v) = part.strip_prefix("v1=") {
            signature = v;
        }
    }

    if timestamp.is_empty() || signature.is_empty() {
        return Err(AppError::BadRequest("Firma de Stripe invalida".into()));
    }

    /* Verificar que el timestamp no sea muy viejo (5 min max) */
    let now = chrono::Utc::now().timestamp();
    let ts: i64 = timestamp
        .parse()
        .map_err(|_| AppError::BadRequest("Timestamp invalido".into()))?;

    if (now - ts).unsigned_abs() > 300 {
        return Err(AppError::BadRequest(
            "Webhook timestamp demasiado viejo".into(),
        ));
    }

    /* Calcular HMAC */
    let signed_payload = format!("{timestamp}.{payload}");
    let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("Error inicializando HMAC: {e}")))?;
    mac.update(signed_payload.as_bytes());

    let expected_hex = hex::encode(mac.finalize().into_bytes());

    if expected_hex != signature {
        return Err(AppError::BadRequest("Firma de webhook invalida".into()));
    }

    Ok(())
}

async fn handle_completed(state: &AppState, session: &serde_json::Value) -> Result<(), AppError> {
    let order_id_raw = session["metadata"]["order_id"].as_str().unwrap_or("");
    if order_id_raw.is_empty() {
        tracing::warn!("Webhook sin order_id en metadata");
        return Ok(());
    }
    let Ok(order_id) = uuid::Uuid::parse_str(order_id_raw) else {
        tracing::warn!("order_id invalido: {order_id_raw}");
        return Ok(());
    };

    let payment_intent = session["payment_intent"].as_str();
    OrderRepository::mark_paid(&state.pool, order_id, payment_intent).await?;
    let Some(order) = OrderRepository::find_by_id(&state.pool, order_id).await? else {
        tracing::warn!("Orden pagada no encontrada: {order_id}");
        return Ok(());
    };
    let Some(product) = ProductRepository::find_by_id(&state.pool, order.product_id).await? else {
        tracing::warn!("Producto de orden no encontrado: {}", order.product_id);
        return Ok(());
    };

    if let (Some(api_key), Some(download_path)) = (&state.resend_api_key, &product.download_path) {
        let customer_email = session["customer_email"]
            .as_str()
            .or_else(|| session["customer_details"]["email"].as_str())
            .unwrap_or(&order.customer_email);
        let download_url = format!("{}{download_path}", state.site_url);
        EmailService::send_download_link(
            api_key,
            &state.email_from,
            customer_email,
            &product.name,
            &download_url,
        )
        .await?;
        OrderRepository::mark_delivered(&state.pool, order_id).await?;
        tracing::info!("Orden {order_id} pagada y descarga enviada a {customer_email}");
    }
    Ok(())
}

async fn handle_expired(state: &AppState, session: &serde_json::Value) -> Result<(), AppError> {
    let Some(order_id_raw) = session["metadata"]["order_id"].as_str() else {
        tracing::warn!("Webhook expirado sin order_id");
        return Ok(());
    };
    let Ok(order_id) = uuid::Uuid::parse_str(order_id_raw) else {
        tracing::warn!("order_id expirado invalido: {order_id_raw}");
        return Ok(());
    };
    OrderRepository::mark_failed(&state.pool, order_id).await?;
    tracing::info!("Orden {order_id} expirada");
    Ok(())
}

/// Endpoint del webhook de Stripe.
/// Recibe eventos y procesa `checkout.session.completed`.
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, AppError> {
    let webhook_secret = state
        .stripe_webhook_secret
        .as_ref()
        .ok_or_else(|| AppError::Internal("Webhook secret no configurado".into()))?;
    let signature_header = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing stripe-signature header".into()))?;
    verify_stripe_signature(&body, signature_header, webhook_secret)?;

    let event: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| AppError::BadRequest(format!("JSON invalido: {error}")))?;
    let session = &event["data"]["object"];
    match event["type"].as_str().unwrap_or("") {
        "checkout.session.completed" => handle_completed(&state, session).await?,
        "checkout.session.expired" => handle_expired(&state, session).await?,
        event_type => tracing::debug!("Evento Stripe no manejado: {event_type}"),
    }
    Ok(StatusCode::OK)
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/webhook/stripe", post(stripe_webhook))
}
