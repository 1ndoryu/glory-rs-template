/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro limite-lineas:
 * order tiene 3 queries dinámicas legacy (DELETE services, check exists) que no usan macros.
 * limite-lineas: dividir OrderRepository entre archivos Rust requiere restructurar en
 * directorio-módulo (order/mod.rs + order/phases.rs) — complejidad sin beneficio funcional. */
/* [044A-38] Repositorio de órdenes: CRUD sobre orders, order_phases, services, service_plans.
 * [044A-44] Migrado a query_as! con verificación en compilación. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Order, OrderPhase, OrderStatus, PaymentMode, PhaseStatus};

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

    /* ============================================================
    FASES DE ORDEN
    ============================================================ */

    pub async fn create_order_phase(
        pool: &PgPool,
        params: CreatePhaseParams<'_>,
    ) -> Result<OrderPhase, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"INSERT INTO order_phases (order_id, phase_number, title, description,
             price_cents, status, max_revisions, estimated_days)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at"#,
            params.order_id,
            params.phase_number,
            params.title,
            params.description,
            params.price_cents,
            params.status as PhaseStatus,
            params.max_revisions,
            params.estimated_days,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn list_order_phases(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Vec<OrderPhase>, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"SELECT id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at
             FROM order_phases WHERE order_id = $1 ORDER BY phase_number"#,
            order_id,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update_phase_status(
        pool: &PgPool,
        phase_id: Uuid,
        status: PhaseStatus,
    ) -> Result<OrderPhase, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"UPDATE order_phases SET status = $2, updated_at = NOW() WHERE id = $1
             RETURNING id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at"#,
            phase_id,
            status as PhaseStatus,
        )
        .fetch_one(pool)
        .await
    }

    /* [044A-38 Fase 2] Busca una fase por order_id + phase_number */
    pub async fn find_phase_by_number(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
    ) -> Result<Option<OrderPhase>, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"SELECT id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at
             FROM order_phases WHERE order_id = $1 AND phase_number = $2"#,
            order_id,
            phase_number,
        )
        .fetch_optional(pool)
        .await
    }

    /* [044A-38 Fase 2] Marca fase como entregada */
    pub async fn deliver_phase(pool: &PgPool, phase_id: Uuid) -> Result<OrderPhase, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"UPDATE order_phases SET status = 'delivered', delivered_at = NOW(), updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at"#,
            phase_id,
        )
        .fetch_one(pool)
        .await
    }

    /* [044A-38 Fase 2] Marca fase como aprobada */
    pub async fn approve_phase(pool: &PgPool, phase_id: Uuid) -> Result<OrderPhase, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"UPDATE order_phases SET status = 'approved', approved_at = NOW(), updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at"#,
            phase_id,
        )
        .fetch_one(pool)
        .await
    }

    /* [044A-38 Fase 2] Incrementa revisiones y pone status revision_requested */
    pub async fn request_revision(
        pool: &PgPool,
        phase_id: Uuid,
    ) -> Result<OrderPhase, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"UPDATE order_phases SET status = 'revision_requested',
             revisions_used = revisions_used + 1, updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at"#,
            phase_id,
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

    /// Obtiene título, slug del servicio y nombre del plan para un order
    pub async fn get_order_display_info(
        pool: &PgPool,
        service_id: Uuid,
        plan_id: Uuid,
    ) -> Result<(String, String, String), sqlx::Error> {
        let (service_title, service_slug): (String, String) = {
            let row = sqlx::query!(
                r#"SELECT title, slug FROM services WHERE id = $1"#,
                service_id,
            )
            .fetch_one(pool)
            .await?;
            (row.title, row.slug)
        };

        let plan_name: String =
            sqlx::query_scalar!(r#"SELECT name FROM service_plans WHERE id = $1"#, plan_id,)
                .fetch_one(pool)
                .await?;

        Ok((service_title, service_slug, plan_name))
    }

    /* [064A-30] Obtiene display_name del empleado asignado a una orden.
     * Retorna None si employee_id es None o si el usuario no tiene display_name.
     * Usa query_scalar sin macro para no depender del cache offline. */
    pub async fn get_employee_display_name(
        pool: &PgPool,
        employee_id: Option<Uuid>,
    ) -> Result<Option<String>, sqlx::Error> {
        let Some(eid) = employee_id else {
            return Ok(None);
        };
        let name: Option<String> =
            sqlx::query_scalar("SELECT display_name FROM users WHERE id = $1")
                .bind(eid)
                .fetch_optional(pool)
                .await?
                .flatten();
        Ok(name)
    }

    /* [074A-53] Obtener nombre del cliente para la respuesta de órdenes */
    pub async fn get_client_display_name(
        pool: &PgPool,
        client_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        let name: Option<String> =
            sqlx::query_scalar("SELECT display_name FROM users WHERE id = $1")
                .bind(client_id)
                .fetch_optional(pool)
                .await?
                .flatten();
        Ok(name)
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

    pub async fn update_order_phase_definition(
        pool: &PgPool,
        phase_id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        price_cents: Option<i32>,
        estimated_days: Option<i32>,
        max_revisions: Option<i32>,
    ) -> Result<OrderPhase, sqlx::Error> {
        sqlx::query_as!(
            OrderPhase,
            r#"UPDATE order_phases
               SET title = COALESCE($2, title),
                   description = COALESCE($3, description),
                   price_cents = COALESCE($4, price_cents),
                   estimated_days = COALESCE($5, estimated_days),
                   max_revisions = COALESCE($6, max_revisions),
                   updated_at = NOW()
             WHERE id = $1
             RETURNING id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at"#,
            phase_id,
            title,
            description,
            price_cents,
            estimated_days,
            max_revisions,
        )
        .fetch_one(pool)
        .await
    }

    /* [T-10] Activa/desactiva IA intermediaria para una orden */
    pub async fn toggle_ai_intermediary(
        pool: &PgPool,
        order_id: Uuid,
        enabled: bool,
    ) -> Result<Order, sqlx::Error> {
        sqlx::query_as::<_, Order>(
            "UPDATE orders SET ai_intermediary_enabled = $2, updated_at = NOW() \
             WHERE id = $1 \
             RETURNING id, order_number, client_id, service_id, plan_id, \
             payment_mode, base_price_cents, discount_percent, final_price_cents, currency, \
             status, assigned_employee_id, assigned_at, auto_assign_deadline, current_phase, \
               started_at, completed_at, cancelled_at, project_description, client_notes, internal_notes, \
             created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees",
        )
        .bind(order_id)
        .bind(enabled)
        .fetch_one(pool)
        .await
    }

    /* [T-10] Actualiza el resumen IA de una orden (reemplaza, no acumula) */
    pub async fn update_ai_summary(
        pool: &PgPool,
        order_id: Uuid,
        summary: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Order>(
            "UPDATE orders SET ai_summary = $2, updated_at = NOW() WHERE id = $1 \
             RETURNING id, order_number, client_id, service_id, plan_id, \
             payment_mode, base_price_cents, discount_percent, final_price_cents, currency, \
             status, assigned_employee_id, assigned_at, auto_assign_deadline, current_phase, \
               started_at, completed_at, cancelled_at, project_description, client_notes, internal_notes, \
             created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees",
        )
        .bind(order_id)
        .bind(summary)
        .fetch_one(pool)
        .await?;
        Ok(())
    }

    /* [035A-30] Número de fase máximo para la orden — para agregar la siguiente fase. */
    pub async fn max_phase_number(pool: &PgPool, order_id: Uuid) -> Result<i32, sqlx::Error> {
        let result: Option<i32> = sqlx::query_scalar!(
            "SELECT MAX(phase_number) FROM order_phases WHERE order_id = $1",
            order_id,
        )
        .fetch_one(pool)
        .await?;
        Ok(result.unwrap_or(0))
    }

    /* [035A-30] Agrega una nueva fase bloqueada al final de la orden. */
    pub async fn add_order_phase(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
    ) -> Result<OrderPhase, sqlx::Error> {
        let title = format!("Fase {phase_number}");
        sqlx::query_as!(
            OrderPhase,
            r#"INSERT INTO order_phases (order_id, phase_number, title, description,
             price_cents, status, max_revisions, estimated_days)
             VALUES ($1, $2, $3, NULL, 0, 'locked', 2, 7)
             RETURNING id, order_id, phase_number, title, description, price_cents,
               status as "status: PhaseStatus",
               max_revisions, revisions_used, estimated_days,
               started_at, delivered_at, approved_at, deadline, created_at, updated_at"#,
            order_id,
            phase_number,
            title.as_str(),
        )
        .fetch_one(pool)
        .await
    }

    /* [035A-30] Elimina una fase bloqueada. Retorna filas afectadas (0 si no era locked). */
    pub async fn delete_order_phase(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
    ) -> Result<u64, sqlx::Error> {
        let rows = sqlx::query!(
            "DELETE FROM order_phases WHERE order_id = $1 AND phase_number = $2 AND status = 'locked'",
            order_id,
            phase_number,
        )
        .execute(pool)
        .await?
        .rows_affected();
        Ok(rows)
    }

    /* [124A-SENT-R1] order_id de una fase — usado en deliverables para verificar acceso.
     * runtime query (sin macro). */
    pub async fn phase_order_id(
        pool: &PgPool,
        phase_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>("SELECT order_id FROM order_phases WHERE id = $1")
            .bind(phase_id)
            .fetch_optional(pool)
            .await
    }

    /* [124A-SENT-R1] order_number de una orden — usado en chat/rest.rs.
     * runtime query (sin macro). */
    pub async fn order_number_by_id(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<i32>, sqlx::Error> {
        sqlx::query_scalar::<_, i32>("SELECT order_number FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(pool)
            .await
    }

    /* [124A-SENT-R1] client_id de una orden — usado en chat/rest_messages.rs.
     * runtime query (sin macro). */
    pub async fn client_id_by_id(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>("SELECT client_id FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(pool)
            .await
    }

    /* [01AA-3] Conteo de órdenes de un cliente — usado en
     * handlers/orders.rs (first-order-discount). runtime query (sin macro). */
    pub async fn count_for_client(pool: &PgPool, client_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders WHERE client_id = $1")
            .bind(client_id)
            .fetch_one(pool)
            .await
    }

    /* [124A-SENT-R1] Participantes de una orden (client_id + assigned_employee_id).
     * Devuelve (client_id, assigned_employee_id) para verificar acceso en chat.
     * runtime query (sin macro). */
    pub async fn get_order_participants(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<(Uuid, Option<Uuid>)>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            client_id: Uuid,
            assigned_employee_id: Option<Uuid>,
        }
        let row = sqlx::query_as::<_, Row>(
            "SELECT client_id, assigned_employee_id FROM orders WHERE id = $1",
        )
        .bind(order_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|r| (r.client_id, r.assigned_employee_id)))
    }
}
