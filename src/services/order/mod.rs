/* [044A-38] Servicio de ordenes: logica de negocio para crear ordenes, calcular
 * descuentos segun payment_mode, y generar fases automaticamente desde plantillas.
 * Descuentos: Full = 20%, HalfHalf = 10%, Phased = 0%. */

mod catalog;
mod lifecycle;
mod phases;

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{PaymentMode, PhaseStatus};

pub struct OrderService;

impl OrderService {
    /* ============================================================
    HELPERS PRIVADOS
    ============================================================ */

    /// Descuento por modo de pago: full=20%, `half_half`=10%, phased=0%
    fn discount_for_mode(mode: PaymentMode) -> i32 {
        match mode {
            PaymentMode::Full => 20,
            PaymentMode::HalfHalf => 10,
            PaymentMode::Phased => 0,
        }
    }

    /// [SEO-D] Descuento 50% para primer pedido. Retorna 0 si ya tiene órdenes previas.
    async fn first_order_discount_percent(pool: &PgPool, user_id: Uuid) -> Result<i32, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE client_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Error verificando órdenes previas: {e}")))?;

        if count == 0 {
            Ok(50)
        } else {
            Ok(0)
        }
    }

    /// Estado inicial de la primera fase segun modo de pago.
    /// [166A-2] Pública para uso desde `PaymentService` (checkout directo).
    pub fn initial_phase_status(mode: PaymentMode) -> PhaseStatus {
        match mode {
            PaymentMode::Full => PhaseStatus::Paid,
            PaymentMode::HalfHalf | PaymentMode::Phased => PhaseStatus::PendingPayment,
        }
    }
}

/* [154A-15c] Formato de precio en centavos -> string legible para emails */
#[must_use]
pub fn format_price_cents(cents: i32, currency: &str) -> String {
    let dollars = f64::from(cents) / 100.0;
    format!("${dollars:.2} {currency}")
}
