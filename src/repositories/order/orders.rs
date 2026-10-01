/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro:
 * reopen_after_rejection usa runtime query para no requerir sqlx prepare. */
//! Órdenes: CRUD, asignación y transiciones de estado.

use sqlx::PgPool;
use uuid::Uuid;

use super::{CreateOrderParams, OrderRepository};
use crate::models::{Order, OrderStatus, PaymentMode};

impl OrderRepository {
    /* ============================================================
    ÓRDENES
    ============================================================ */

    pub async fn create_order(
        pool: &PgPool,
        params: CreateOrderParams<'_>,
    ) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"INSERT INTO orders (client_id, service_id, plan_id, payment_mode,
             base_price_cents, discount_percent, final_price_cents, project_description, client_notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
               started_at, completed_at, cancelled_at, project_description, client_notes,
               internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            params.client_id,
            params.service_id,
            params.plan_id,
            params.payment_mode as PaymentMode,
            params.base_price_cents,
            params.discount_percent,
            params.final_price_cents,
            params.project_description,
            params.client_notes,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_order_by_id(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<Order>, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"SELECT id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees
             FROM orders WHERE id = $1"#,
            order_id,
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list_orders_for_client(
        pool: &PgPool,
        client_id: Uuid,
    ) -> Result<Vec<Order>, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"SELECT id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees
             FROM orders WHERE client_id = $1 ORDER BY created_at DESC"#,
            client_id,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_orders_for_employee(
        pool: &PgPool,
        employee_id: Uuid,
    ) -> Result<Vec<Order>, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"SELECT id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees
             FROM orders WHERE assigned_employee_id = $1 ORDER BY created_at DESC"#,
            employee_id,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_all_orders(pool: &PgPool) -> Result<Vec<Order>, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"SELECT id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees
             FROM orders ORDER BY created_at DESC"#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_unassigned_orders(pool: &PgPool) -> Result<Vec<Order>, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"SELECT id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees
             FROM orders WHERE status = 'awaiting_assignment' ORDER BY created_at ASC"#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update_order_status(
        pool: &PgPool,
        order_id: Uuid,
        status: OrderStatus,
    ) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders SET status = $2, updated_at = NOW() WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
            status as OrderStatus,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn assign_order(
        pool: &PgPool,
        order_id: Uuid,
        employee_id: Uuid,
    ) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders SET assigned_employee_id = $2, assigned_at = NOW(),
             status = 'in_progress', started_at = COALESCE(started_at, NOW()), updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
            employee_id,
        )
        .fetch_one(pool)
        .await
    }

    /* [016A-5] Desasignar empleado de una orden (admin). Vuelve el estado a awaiting_assignment. */
    pub async fn unassign_order(pool: &PgPool, order_id: Uuid) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders SET assigned_employee_id = NULL, assigned_at = NULL,
             status = 'awaiting_assignment', open_to_employees = true, updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
        )
        .fetch_one(pool)
        .await
    }

    /* [044A-38 Fase 2] Cancela una orden: marca cancelled + timestamp */
    pub async fn cancel_order(pool: &PgPool, order_id: Uuid) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders SET status = 'cancelled', cancelled_at = NOW(), updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
               started_at, completed_at, cancelled_at, project_description, client_notes,
               internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update_project_description(
        pool: &PgPool,
        order_id: Uuid,
        project_description: &str,
    ) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders
               SET project_description = $2, updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
               started_at, completed_at, cancelled_at, project_description, client_notes,
               internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
            project_description,
        )
        .fetch_one(pool)
        .await
    }

    /* [044A-38 Fase 2] Actualiza current_phase de una orden */
    pub async fn update_current_phase(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
    ) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders SET current_phase = $2, updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
            phase_number,
        )
        .fetch_one(pool)
        .await
    }

    /* [044A-38 Fase 2] Marca orden como completada */
    pub async fn complete_order(pool: &PgPool, order_id: Uuid) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders SET status = 'completed', completed_at = NOW(), updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
                    started_at, completed_at, cancelled_at, project_description, client_notes,
                    internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
        )
        .fetch_one(pool)
        .await
    }

    /* ============================================================
    [044A-38 Fase 4] ASIGNACIÓN Y AUTO-ASIGNACIÓN
    ============================================================ */

    /// Transiciona orden a `awaiting_assignment` y establece deadline de 48h para que empleados la tomen
    pub async fn set_awaiting_assignment(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Order, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"UPDATE orders SET status = 'awaiting_assignment',
             auto_assign_deadline = NOW() + INTERVAL '48 hours',
             updated_at = NOW() WHERE id = $1
             RETURNING id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
               started_at, completed_at, cancelled_at, project_description, client_notes,
               internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees"#,
            order_id,
        )
        .fetch_one(pool)
        .await
    }

    /* [124A-SENT-R1] Reabre una orden rechazada al pool de disponibles.
     * Difiere de set_awaiting_assignment: también unasigna al empleado y
     * no establece deadline de auto-asignación (queda null para no urgir).
     * runtime query (sin macro) para no requerir sqlx prepare. */
    pub async fn reopen_after_rejection(pool: &PgPool, order_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE orders \
             SET status = 'awaiting_assignment', \
                 assigned_employee_id = NULL, \
                 assigned_at = NULL, \
                 open_to_employees = true, \
                 updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(order_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Órdenes que pasaron su deadline de auto-asignación (24h en `awaiting_assignment`)
    pub async fn list_overdue_unassigned(pool: &PgPool) -> Result<Vec<Order>, sqlx::Error> {
        sqlx::query_as!(
            Order,
            r#"SELECT id, order_number, client_id, service_id, plan_id,
               payment_mode as "payment_mode: PaymentMode",
               base_price_cents, discount_percent, final_price_cents, currency,
               status as "status: OrderStatus",
               assigned_employee_id, assigned_at, auto_assign_deadline, current_phase,
               started_at, completed_at, cancelled_at, project_description, client_notes,
               internal_notes,
               created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees
             FROM orders WHERE status = 'awaiting_assignment'
             AND auto_assign_deadline IS NOT NULL AND auto_assign_deadline < NOW()
             ORDER BY auto_assign_deadline ASC"#,
        )
        .fetch_all(pool)
        .await
    }
}
