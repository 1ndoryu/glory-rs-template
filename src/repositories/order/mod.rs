/* [044A-38] Repositorio de órdenes: CRUD sobre orders, order_phases, services, service_plans.
 * [044A-44] Migrado a query_as! con verificación en compilación.
 * [01AA-4] Partido en directorio-módulo (orders/phases/meta); el waiver
 * `limite-lineas` anterior queda obsoleto porque cada parte está bajo el límite. */

mod meta;
mod orders;
mod phases;

use uuid::Uuid;

use crate::models::{PaymentMode, PhaseStatus};

/* Structs de parámetros para evitar too_many_arguments (clippy) */

pub struct CreateOrderParams<'a> {
    pub client_id: Uuid,
    pub service_id: Uuid,
    pub plan_id: Uuid,
    pub payment_mode: PaymentMode,
    pub base_price_cents: i32,
    pub discount_percent: i32,
    pub final_price_cents: i32,
    pub project_description: Option<&'a str>,
    pub client_notes: Option<&'a str>,
}

pub struct CreatePhaseParams<'a> {
    pub order_id: Uuid,
    pub phase_number: i32,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub price_cents: i32,
    pub status: PhaseStatus,
    pub max_revisions: i32,
    pub estimated_days: i32,
}

pub struct OrderRepository;
