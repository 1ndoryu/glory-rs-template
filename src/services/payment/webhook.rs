/* [01AA-4-F3b] Eventos de Stripe + firma HMAC (era parte de
 * services/payment.rs). */

use sqlx::PgPool;

use super::PaymentService;
use crate::errors::AppError;
use crate::models::{PaymentMode, PaymentStatus};
use crate::repositories::PaymentRepository;

impl PaymentService {
    /// Procesa webhook de Stripe (ya verificada la firma).
    /// `event_data` es opcional para mantener compatibilidad con llamadas existentes;
    /// cuando se pasa, permite detectar checkout flows por metadata.
    pub async fn handle_webhook(
        pool: &PgPool,
        event_type: &str,
        data: &serde_json::Value,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), AppError> {
        match event_type {
            "payment_intent.succeeded" => {
                let pi_id = data["object"]["id"]
                    .as_str()
                    .ok_or_else(|| AppError::BadRequest("Missing payment_intent id".into()))?;
                let charge_id = data["object"]["latest_charge"].as_str();

                /* [166A-2] Detectar checkout flow: si metadata.source == "checkout",
                 * crear la orden + pago desde cero. El intent NO tiene order_id asociado. */
                let meta_source =
                    event_data.and_then(|d| d["object"]["metadata"]["source"].as_str());

                if meta_source == Some("checkout") {
                    /* [20CA-1] Extraer email de metadata (en vez de user_id).
                     * El usuario se crea/encuentra dentro de handle_checkout_payment_succeeded. */
                    let email = event_data
                        .and_then(|d| d["object"]["metadata"]["email"].as_str())
                        .ok_or_else(|| {
                            AppError::BadRequest("Missing email in checkout metadata".into())
                        })?;
                    let service_slug = event_data
                        .and_then(|d| d["object"]["metadata"]["service_slug"].as_str())
                        .ok_or_else(|| {
                            AppError::BadRequest("Missing service_slug in metadata".into())
                        })?;
                    let plan_slug = event_data
                        .and_then(|d| d["object"]["metadata"]["plan_slug"].as_str())
                        .ok_or_else(|| {
                            AppError::BadRequest("Missing plan_slug in metadata".into())
                        })?;
                    let payment_mode_str = event_data
                        .and_then(|d| d["object"]["metadata"]["payment_mode"].as_str())
                        .unwrap_or("Full");
                    let payment_mode: PaymentMode =
                        serde_json::from_str(&format!("\"{}\"", payment_mode_str.to_lowercase()))
                            .unwrap_or(PaymentMode::Full);

                    let amount_cents = data["object"]["amount"].as_i64().unwrap_or(0) as i32;

                    Self::handle_checkout_payment_succeeded(
                        pool,
                        email,
                        service_slug,
                        plan_slug,
                        payment_mode,
                        pi_id,
                        charge_id,
                        amount_cents,
                    )
                    .await?;

                    return Ok(());
                }

                /* Flujo original: pago contra orden existente */
                let payment = PaymentRepository::find_by_stripe_intent(pool, pi_id)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Payment for intent {pi_id} not found"))
                    })?;

                PaymentRepository::update_status_held(pool, payment.id).await?;
                if let Some(cid) = charge_id {
                    PaymentRepository::update_charge_id(pool, payment.id, cid).await?;
                }

                Self::handle_payment_success(pool, &payment).await?;
            }
            "payment_intent.payment_failed" => {
                let pi_id = data["object"]["id"]
                    .as_str()
                    .ok_or_else(|| AppError::BadRequest("Missing payment_intent id".into()))?;

                if let Some(payment) = PaymentRepository::find_by_stripe_intent(pool, pi_id).await?
                {
                    PaymentRepository::update_status(pool, payment.id, PaymentStatus::Failed)
                        .await?;
                }
            }
            _ => {
                tracing::debug!("Evento Stripe no manejado: {event_type}");
            }
        }
        Ok(())
    }

    /// Verifica firma HMAC-SHA256 del webhook de Stripe (constant-time comparison)
    pub fn verify_webhook_signature(
        payload: &[u8],
        signature_header: &str,
        webhook_secret: &str,
    ) -> Result<(), AppError> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut timestamp = None;
        let mut expected_sig = None;

        for part in signature_header.split(',') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "t" => timestamp = Some(value),
                    "v1" => expected_sig = Some(value),
                    _ => {}
                }
            }
        }

        let ts = timestamp
            .ok_or_else(|| AppError::BadRequest("Missing timestamp in Stripe signature".into()))?;
        let sig = expected_sig
            .ok_or_else(|| AppError::BadRequest("Missing v1 in Stripe signature".into()))?;

        let signed_payload = format!("{ts}.{}", String::from_utf8_lossy(payload));

        let mut mac = Hmac::<Sha256>::new_from_slice(webhook_secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
        mac.update(signed_payload.as_bytes());

        let decoded_sig = hex::decode(sig)
            .map_err(|_| AppError::BadRequest("Invalid hex in Stripe signature".into()))?;

        mac.verify_slice(&decoded_sig)
            .map_err(|_| AppError::Forbidden("Invalid webhook signature".into()))?;

        /* [064A-73] Verificar freshness: máximo 2 minutos de tolerancia (antes 5 min).
         * Reducido para estrechar la ventana de replay attacks. */
        if let Ok(ts_num) = ts.parse::<i64>() {
            let now = chrono::Utc::now().timestamp();
            if (now - ts_num).unsigned_abs() > 120 {
                return Err(AppError::BadRequest("Webhook timestamp too old".into()));
            }
        }

        Ok(())
    }
}
