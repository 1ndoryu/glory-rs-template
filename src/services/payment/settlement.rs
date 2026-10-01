/* [01AA-4-F3b] Captura, listado y reembolsos (era parte de
 * services/payment.rs). */

use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use super::PaymentService;
use crate::errors::AppError;
use crate::models::{OrderPayment, PaymentResponse, PaymentStatus};
use crate::repositories::{OrderRepository, PaymentRepository};

/* [044A-38 Fase 7] Tipo mínimo para parsear respuesta de Stripe Refund */
#[derive(Debug, serde::Deserialize)]
struct StripeRefundMin {
    id: String,
}

impl PaymentService {
    /// Captura todos los pagos retenidos de una orden (al completarse)
    pub async fn capture_held_payments(
        pool: &PgPool,
        http_client: &Client,
        stripe_key: &str,
        order_id: Uuid,
    ) -> Result<(), AppError> {
        let held = PaymentRepository::find_held_for_order(pool, order_id).await?;
        for payment in held {
            if let Some(ref pi_id) = payment.stripe_payment_intent_id {
                /* [155A-11] Los pagos de prueba usan intents sinteticos: avanzan la orden
                 * sin cobro real y no deben capturarse contra Stripe al completar. */
                if Self::is_test_bypass_intent(pi_id) {
                    PaymentRepository::update_status_released(pool, payment.id).await?;
                    continue;
                }
                Self::capture_stripe_intent(http_client, stripe_key, pi_id).await?;
                PaymentRepository::update_status_released(pool, payment.id).await?;
            }
        }
        Ok(())
    }

    /// Lista pagos de una orden con números de fase resueltos
    pub async fn list_payments(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Vec<PaymentResponse>, AppError> {
        let payments = PaymentRepository::list_for_order(pool, order_id).await?;
        let phases = OrderRepository::list_order_phases(pool, order_id).await?;

        Ok(payments
            .into_iter()
            .map(|p| {
                let bypassed = p
                    .stripe_payment_intent_id
                    .as_deref()
                    .is_some_and(Self::is_test_bypass_intent);
                let phase_number = p.phase_id.and_then(|pid| {
                    phases
                        .iter()
                        .find(|ph| ph.id == pid)
                        .map(|ph| ph.phase_number)
                });
                PaymentResponse {
                    id: p.id,
                    order_id: p.order_id,
                    phase_number,
                    amount_cents: p.amount_cents,
                    currency: p.currency,
                    status: p.status,
                    payment_mode: p.payment_mode,
                    description: p.description,
                    bypassed,
                    created_at: p.created_at,
                }
            })
            .collect())
    }

    /// [044A-38 Fase 7] Ejecutar reembolso en Stripe.
    /// Para pagos con `capture_method=manual` (held): cancela el `PaymentIntent`.
    /// Para pagos ya capturados (released): crea un `Refund`.
    pub async fn refund_payment(
        http_client: &Client,
        stripe_key: &str,
        payment: &OrderPayment,
    ) -> Result<String, AppError> {
        let pi_id = payment
            .stripe_payment_intent_id
            .as_deref()
            .ok_or_else(|| AppError::Internal("Pago sin PaymentIntent de Stripe".into()))?;

        match payment.status {
            PaymentStatus::Held => {
                /* Fondos retenidos → cancelar PaymentIntent libera el dinero */
                let url = format!("https://api.stripe.com/v1/payment_intents/{pi_id}/cancel");
                let resp = http_client
                    .post(&url)
                    .basic_auth(stripe_key, None::<&str>)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(format!("Error cancelando PaymentIntent Stripe: {e}"))
                    })?;

                if !resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("Stripe cancel falló: {body}");
                    return Err(AppError::Internal(format!("Stripe cancel error: {body}")));
                }
                /* Para cancel, el refund_id es el PI id mismo */
                Ok(format!("cancel_{pi_id}"))
            }
            PaymentStatus::Released => {
                /* Fondos ya capturados → crear Refund en Stripe */
                /* [277A-7] Idempotency-Key en refunds: previene reembolsos duplicados
                 * si el retry worker se ejecuta mientras Stripe aún procesa el anterior. */
                let idempotency_key = Uuid::new_v4().to_string();

                let resp = http_client
                    .post("https://api.stripe.com/v1/refunds")
                    .basic_auth(stripe_key, None::<&str>)
                    .header("Idempotency-Key", &idempotency_key)
                    .form(&[("payment_intent", pi_id)])
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Error creando refund Stripe: {e}")))?;

                if !resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("Stripe refund falló: {body}");
                    return Err(AppError::Internal(format!("Stripe refund error: {body}")));
                }

                let refund_resp = resp.json::<StripeRefundMin>().await.map_err(|e| {
                    AppError::Internal(format!("Error parseando refund Stripe: {e}"))
                })?;
                Ok(refund_resp.id)
            }
            _ => Err(AppError::BadRequest(
                "El pago no está en un estado reembolsable".into(),
            )),
        }
    }

    /// Captura un `PaymentIntent` en Stripe (libera fondos retenidos)
    async fn capture_stripe_intent(
        client: &Client,
        api_key: &str,
        payment_intent_id: &str,
    ) -> Result<(), AppError> {
        let url = format!("https://api.stripe.com/v1/payment_intents/{payment_intent_id}/capture");
        let resp = client
            .post(&url)
            .basic_auth(api_key, None::<&str>)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Error capturando pago Stripe: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            /* [064A-73] Log explícito de errores Stripe para debugging */
            tracing::error!("Stripe capture falló: {body}");
            return Err(AppError::Internal(format!("Stripe capture error: {body}")));
        }
        Ok(())
    }

    pub(crate) fn is_test_bypass_intent(intent_id: &str) -> bool {
        intent_id.starts_with("test_bypass_")
    }
}
