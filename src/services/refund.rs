/* [277A-7] Servicio de reembolsos: procesamiento con retry y backoff exponencial.
 * Extraído del handler para separar concerns y habilitar retry automático. */

use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{OrderStatus, PaymentStatus, RefundStatus};
use crate::repositories::{OrderRepository, PaymentRepository, RefundRepository};
use super::PaymentService;

pub struct RefundService;

impl RefundService {
    /* [277A-7] Procesa un reembolso aprobado contra Stripe.
     * Flujo: marcar processing → ejecutar refund en Stripe → completar o fallar con retry.
     * Usado tanto por el handler (aprobación inmediata) como por el worker de retry. */
    pub async fn process_approved_refund(
        pool: &PgPool,
        http_client: &Client,
        stripe_key: Option<&str>,
        refund_id: Uuid,
    ) -> Result<(), AppError> {
        let refund = RefundRepository::find_by_id(pool, refund_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Reembolso no encontrado".into()))?;

        if refund.status != RefundStatus::Approved
            && refund.status != RefundStatus::Failed
            && refund.status != RefundStatus::Processing
        {
            return Err(AppError::BadRequest(
                "El reembolso no está en estado procesable".into(),
            ));
        }

        /* [277A-7] Incrementar attempts UNA sola vez y usar ese valor en todo el flujo.
         * Antes se incrementaba en mark_processing Y en mark_failed_with_retry, causando
         * doble conteo. Ahora: computed_attempts = refund.attempts + 1 (consistente). */
        let computed_attempts = refund.attempts + 1;
        let _ = RefundRepository::mark_processing(pool, refund_id, computed_attempts).await?;

        /* Buscar el pago asociado */
        let payments = PaymentRepository::list_for_order(pool, refund.order_id).await?;
        let payment = payments
            .iter()
            .find(|p| p.id == refund.payment_id)
            .ok_or_else(|| {
                AppError::Internal("Pago asociado al reembolso no encontrado".into())
            })?;

        /* Ejecutar refund en Stripe */
        let stripe_refund_id = match stripe_key {
            None => {
                tracing::warn!(
                    "[277A-7] Stripe no configurado, simulando refund {}",
                    refund_id
                );
                format!("simulated_refund_{refund_id}")
            }
            Some(key) => match PaymentService::refund_payment(http_client, key, payment).await {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!(
                        "[277A-7] Stripe refund falló para {}: {e}",
                        refund_id
                    );
                    /* Determinar si se puede reintentar (usando computed_attempts ya incrementado) */
                    if computed_attempts >= refund.max_attempts {
                        /* Agotar intentos → marcar failed permanente (sin next_retry_at) */
                        let _ = RefundRepository::mark_failed_with_retry(
                            pool,
                            refund_id,
                            computed_attempts,
                            refund.max_attempts,
                            None,
                        )
                        .await?;
                        tracing::error!(
                            "[277A-7] Reembolso {} agotó {} intentos, marcado como failed permanente",
                            refund_id,
                            refund.max_attempts
                        );
                    } else {
                        /* Backoff exponencial capado: 1h, 4h, 16h, máx 72h */
                        #[allow(clippy::cast_sign_loss)]
                        let delay_hours = std::cmp::min(
                            4_i64.pow(u32::try_from((computed_attempts - 1).max(0)).unwrap_or(0)),
                            72,
                        );
                        let next = chrono::Utc::now() + chrono::Duration::hours(delay_hours);
                        let _ = RefundRepository::mark_failed_with_retry(
                            pool,
                            refund_id,
                            computed_attempts,
                            refund.max_attempts,
                            Some(next),
                        )
                        .await?;
                        tracing::warn!(
                            "[277A-7] Reembolso {} falló (intento {}/{}), retry en {delay_hours}h",
                            refund_id,
                            computed_attempts,
                            refund.max_attempts
                        );
                    }
                    return Err(e);
                }
            },
        };

        /* Marcar pago como refunded */
        PaymentRepository::update_status(pool, payment.id, PaymentStatus::Refunded).await?;

        /* Marcar reembolso como completado */
        RefundRepository::mark_completed(pool, refund_id, &stripe_refund_id).await?;

        /* Cancelar la orden */
        OrderRepository::update_order_status(pool, refund.order_id, OrderStatus::Cancelled)
            .await?;

        tracing::info!(
            "[277A-7] Reembolso {} completado (stripe: {stripe_refund_id})",
            refund_id
        );

        Ok(())
    }

    /* [277A-7] Worker de retry: busca reembolsos fallidos que deben reintentarse.
     * Ejecuta periódicamente desde background task. */
    pub async fn run_refund_retry_loop(pool: PgPool, stripe_key: Option<String>) {
        const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_mins(15);
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("refund retry HTTP client");

        tracing::info!("[refund-retry] Worker iniciado (intervalo: 15min)");

        loop {
            tokio::time::sleep(CHECK_INTERVAL).await;

            let pending = match RefundRepository::find_pending_retry(&pool).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("[refund-retry] Error consultando reembolsos pendientes: {e}");
                    continue;
                }
            };

            if pending.is_empty() {
                continue;
            }

            tracing::info!(
                "[refund-retry] {} reembolsos pendientes de retry",
                pending.len()
            );

            for refund in pending {
                if let Err(e) = Self::process_approved_refund(
                    &pool,
                    &http_client,
                    stripe_key.as_deref(),
                    refund.id,
                )
                .await
                {
                    tracing::error!(
                        "[refund-retry] Error procesando retry {}: {e}",
                        refund.id
                    );
                }
            }
        }
    }
}
