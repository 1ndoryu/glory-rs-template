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
        return Err(AppError::BadRequest(
            "Firma de Stripe invalida".into(),
        ));
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

    /* Verificar firma */
    let signature_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing stripe-signature header".into()))?;

    verify_stripe_signature(&body, signature_header, webhook_secret)?;

    /* Parsear evento */
    let event: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::BadRequest(format!("JSON invalido: {e}")))?;

    let event_type = event["type"]
        .as_str()
        .unwrap_or("");

    match event_type {
        "checkout.session.completed" => {
            let session = &event["data"]["object"];

            let order_id_str = session["metadata"]["order_id"]
                .as_str()
                .unwrap_or("");
            let payment_intent = session["payment_intent"]
                .as_str()
                .unwrap_or("");
            let customer_email = session["customer_email"]
                .as_str()
                .or_else(|| session["customer_details"]["email"].as_str())
                .unwrap_or("");

            if order_id_str.is_empty() {
                tracing::warn!("Webhook sin order_id en metadata");
                return Ok(StatusCode::OK);
            }

            let order_id = match uuid::Uuid::parse_str(order_id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!("order_id invalido: {order_id_str}");
                    return Ok(StatusCode::OK);
                }
            };

            /* Marcar orden como pagada */
            let _ = crate::repositories::product_repo::OrderRepository::mark_paid(
                &state.pool,
                order_id,
                Some(payment_intent),
            )
            .await;

            /* Buscar la orden y el producto para enviar email de descarga */
            let order = sqlx::query_as::<_, crate::models::product::Order>(
                "SELECT * FROM orders WHERE id = $1",
            )
            .bind(order_id)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();

            if let Some(order) = order {
                let product = crate::repositories::product_repo::ProductRepository::find_by_id(
                    &state.pool,
                    order.product_id,
                )
                .await
                .ok()
                .flatten();

                if let Some(product) = product {
                    /* Enviar email de descarga si hay Resend configurado */
                    if let (Some(ref api_key), Some(ref download_path)) =
                        (&state.resend_api_key, &product.download_path)
                    {
                        let download_url = format!(
                            "{}{}",
                            state.site_url,
                            download_path
                        );

                        let email = if customer_email.is_empty() {
                            &order.customer_email
                        } else {
                            customer_email
                        };

                        let _ = EmailService::send_download_link(
                            api_key,
                            &state.email_from,
                            email,
                            &product.name,
                            &download_url,
                        )
                        .await;

                        let _ = crate::repositories::product_repo::OrderRepository::mark_delivered(
                            &state.pool,
                            order_id,
                        )
                        .await;

                        tracing::info!(
                            "Orden {order_id} pagada y descarga enviada a {email}"
                        );
                    }
                }
            }
        }
        "checkout.session.expired" => {
            let session = &event["data"]["object"];
            let order_id_str = session["metadata"]["order_id"]
                .as_str()
                .unwrap_or("");
            if let Ok(order_id) = uuid::Uuid::parse_str(order_id_str) {
                let _ = sqlx::query("UPDATE orders SET status = 'failed' WHERE id = $1")
                    .bind(order_id)
                    .execute(&state.pool)
                    .await;
                tracing::info!("Orden {order_id} expirada");
            }
        }
        _ => {
            /* Evento no manejado — ignorar silenciosamente */
        }
    }

    Ok(StatusCode::OK)
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/webhook/stripe", post(stripe_webhook))
}
