/* [01AA-4-F3b] Máquina de estados post-pago (era parte de
 * services/payment.rs). */

use sqlx::PgPool;

use super::PaymentService;
use crate::errors::AppError;
use crate::models::{OrderPayment, OrderStatus, PaymentMode, PaymentStatus, PhaseStatus};
use crate::repositories::{OrderRepository, PaymentRepository};

impl PaymentService {
    /// Post-pago: avanza la máquina de estados de la orden según `payment_mode`
    pub(crate) async fn handle_payment_success(
        pool: &PgPool,
        payment: &OrderPayment,
    ) -> Result<(), AppError> {
        let order = OrderRepository::find_order_by_id(pool, payment.order_id)
            .await?
            .ok_or_else(|| AppError::Internal("Orden no encontrada post-pago".into()))?;

        match order.payment_mode {
            PaymentMode::Full => {
                /* Pago completo → todas las fases a Paid, orden a awaiting_assignment */
                OrderRepository::set_awaiting_assignment(pool, order.id).await?;
                let phases = OrderRepository::list_order_phases(pool, order.id).await?;
                for phase in phases {
                    if phase.status == PhaseStatus::PendingPayment
                        || phase.status == PhaseStatus::Locked
                    {
                        OrderRepository::update_phase_status(pool, phase.id, PhaseStatus::Paid)
                            .await?;
                    }
                }
            }
            PaymentMode::HalfHalf => {
                let all_payments = PaymentRepository::list_for_order(pool, order.id).await?;
                let held_count = all_payments
                    .iter()
                    .filter(|p| p.status == PaymentStatus::Held)
                    .count();

                if held_count >= 1 && order.status == OrderStatus::PaymentHeld {
                    /* Primer 50% → awaiting_assignment, desbloquear primera mitad de fases */
                    OrderRepository::set_awaiting_assignment(pool, order.id).await?;
                    let phases = OrderRepository::list_order_phases(pool, order.id).await?;
                    let midpoint = phases.len().div_ceil(2);
                    for (i, phase) in phases.iter().enumerate() {
                        if i < midpoint
                            && (phase.status == PhaseStatus::PendingPayment
                                || phase.status == PhaseStatus::Locked)
                        {
                            OrderRepository::update_phase_status(pool, phase.id, PhaseStatus::Paid)
                                .await?;
                        }
                    }
                }
                /* Segundo 50% → desbloquear fases restantes */
                if held_count >= 2 {
                    let phases = OrderRepository::list_order_phases(pool, order.id).await?;
                    for phase in phases {
                        if phase.status == PhaseStatus::Locked
                            || phase.status == PhaseStatus::PendingPayment
                        {
                            OrderRepository::update_phase_status(pool, phase.id, PhaseStatus::Paid)
                                .await?;
                        }
                    }
                }
            }
            PaymentMode::Phased => {
                /* Pago de fase individual → actualizar esa fase a Paid */
                if let Some(phase_id) = payment.phase_id {
                    OrderRepository::update_phase_status(pool, phase_id, PhaseStatus::Paid).await?;
                }
                /* Si la orden estaba en payment_held y la primera fase se pagó, avanzar */
                if order.status == OrderStatus::PaymentHeld {
                    OrderRepository::set_awaiting_assignment(pool, order.id).await?;
                }
            }
        }
        Ok(())
    }
}
