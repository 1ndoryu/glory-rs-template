/* [259A-4e] Tools IA de pedidos y pagos: `exec_list_my_orders`,
 * `exec_list_my_payments` con queries por alcance de rol. */

/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: queries runtime por rol.
 * Las consultas se construyen por alcance (admin/empleado/cliente) intencionalmente
 * para no depender de caché offline en consultas dinámicas por rol (259A-4e). */

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::UserRole;

use super::ai_tools::{require_auth, tool_json, tool_status, ToolAuthContext, ToolExecResult};

#[derive(sqlx::FromRow)]
struct ToolOrderRow {
    id: Uuid,
    order_number: i32,
    client_id: Uuid,
    service_title: String,
    service_slug: String,
    plan_name: String,
    status: String,
    current_phase: i32,
    total_phases: i64,
    final_price_cents: i32,
    currency: String,
    assigned_employee_id: Option<Uuid>,
    assigned_employee_name: Option<String>,
    open_reports: i64,
    deliverables_count: i64,
    phases: Value,
    updated_at: DateTime<Utc>,
}

pub(crate) async fn exec_list_my_orders(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    match query_orders_for_scope(pool, auth).await {
        Ok(orders) => tool_json(json!({
            "status": "ok",
            "scope": auth.effective_role.to_string(),
            "orders": orders.iter().map(order_row_json).collect::<Vec<_>>(),
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando pedidos para AI tool: {e}"
            );
            tool_status("error", "No se pudieron consultar los pedidos")
        }
    }
}

async fn query_orders_for_scope(
    pool: &PgPool,
    auth: ToolAuthContext,
) -> Result<Vec<ToolOrderRow>, sqlx::Error> {
    match auth.effective_role {
        UserRole::Admin => {
            let query = order_admin_query();
            sqlx::query_as::<_, ToolOrderRow>(&query)
                .fetch_all(pool)
                .await
        }
        UserRole::Employee => {
            let query = order_employee_query();
            sqlx::query_as::<_, ToolOrderRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
        UserRole::Client => {
            let query = order_client_query();
            sqlx::query_as::<_, ToolOrderRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
    }
}

fn order_row_json(row: &ToolOrderRow) -> Value {
    json!({
        "id": row.id,
        "order_number": row.order_number,
        "client_id": row.client_id,
        "service_title": row.service_title,
        "service_slug": row.service_slug,
        "plan_name": row.plan_name,
        "status": row.status,
        "current_phase": row.current_phase,
        "total_phases": row.total_phases,
        "final_price_cents": row.final_price_cents,
        "currency": row.currency,
        "assigned_employee_id": row.assigned_employee_id,
        "assigned_employee_name": row.assigned_employee_name,
        "open_reports": row.open_reports,
        "deliverables_count": row.deliverables_count,
        "phases": row.phases,
        "updated_at": row.updated_at,
    })
}

const ORDER_SELECT_BASE: &str = "
SELECT o.id,
       o.order_number,
       o.client_id,
       s.title AS service_title,
       s.slug AS service_slug,
       sp.name AS plan_name,
       o.status::text AS status,
       o.current_phase,
       (SELECT COUNT(*) FROM order_phases op WHERE op.order_id = o.id) AS total_phases,
       o.final_price_cents,
       o.currency,
       o.assigned_employee_id,
       COALESCE(employee.display_name, employee.email) AS assigned_employee_name,
       (SELECT COUNT(*) FROM order_problems prob WHERE prob.order_id = o.id AND prob.status IN ('open', 'in_review')) AS open_reports,
       (SELECT COUNT(*) FROM phase_deliverables d JOIN order_phases phase ON phase.id = d.phase_id WHERE phase.order_id = o.id) AS deliverables_count,
       COALESCE((
           SELECT jsonb_agg(jsonb_build_object(
               'phase_number', phase.phase_number,
               'title', phase.title,
               'status', phase.status::text,
               'revisions_used', phase.revisions_used,
               'max_revisions', phase.max_revisions,
               'deadline', phase.deadline
           ) ORDER BY phase.phase_number)
           FROM order_phases phase
           WHERE phase.order_id = o.id
       ), '[]'::jsonb) AS phases,
       o.updated_at
FROM orders o
JOIN services s ON s.id = o.service_id
JOIN service_plans sp ON sp.id = o.plan_id
LEFT JOIN users employee ON employee.id = o.assigned_employee_id";

fn order_admin_query() -> String {
    format!("{ORDER_SELECT_BASE} ORDER BY o.updated_at DESC LIMIT 12")
}

fn order_employee_query() -> String {
    format!(
        "{ORDER_SELECT_BASE} WHERE o.assigned_employee_id = $1 ORDER BY o.updated_at DESC LIMIT 12"
    )
}

fn order_client_query() -> String {
    format!("{ORDER_SELECT_BASE} WHERE o.client_id = $1 ORDER BY o.updated_at DESC LIMIT 12")
}

#[derive(sqlx::FromRow)]
struct ToolPaymentRow {
    payment_id: Uuid,
    order_id: Uuid,
    order_number: i32,
    phase_number: Option<i32>,
    amount_cents: i32,
    currency: String,
    status: String,
    payment_mode: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
}

pub(crate) async fn exec_list_my_payments(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    match query_payments_for_scope(pool, auth).await {
        Ok(payments) => tool_json(json!({
            "status": "ok",
            "scope": auth.effective_role.to_string(),
            "payments": payments.iter().map(payment_row_json).collect::<Vec<_>>(),
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando pagos para AI tool: {e}"
            );
            tool_status("error", "No se pudieron consultar los pagos")
        }
    }
}

async fn query_payments_for_scope(
    pool: &PgPool,
    auth: ToolAuthContext,
) -> Result<Vec<ToolPaymentRow>, sqlx::Error> {
    match auth.effective_role {
        UserRole::Admin => {
            let query = payment_admin_query();
            sqlx::query_as::<_, ToolPaymentRow>(&query)
                .fetch_all(pool)
                .await
        }
        UserRole::Employee => {
            let query = payment_employee_query();
            sqlx::query_as::<_, ToolPaymentRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
        UserRole::Client => {
            let query = payment_client_query();
            sqlx::query_as::<_, ToolPaymentRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
    }
}

fn payment_row_json(row: &ToolPaymentRow) -> Value {
    json!({
        "payment_id": row.payment_id,
        "order_id": row.order_id,
        "order_number": row.order_number,
        "phase_number": row.phase_number,
        "amount_cents": row.amount_cents,
        "currency": row.currency,
        "status": row.status,
        "payment_mode": row.payment_mode,
        "description": row.description,
        "created_at": row.created_at,
        "can_retry_from_panel": row.status == "pending" || row.status == "failed",
    })
}

const PAYMENT_SELECT_BASE: &str = "
SELECT p.id AS payment_id,
       p.order_id,
       o.order_number,
       phase.phase_number,
       p.amount_cents,
       p.currency,
       p.status::text AS status,
       p.payment_mode::text AS payment_mode,
       p.description,
       p.created_at
FROM order_payments p
JOIN orders o ON o.id = p.order_id
LEFT JOIN order_phases phase ON phase.id = p.phase_id";

fn payment_admin_query() -> String {
    format!("{PAYMENT_SELECT_BASE} ORDER BY p.created_at DESC LIMIT 20")
}

fn payment_employee_query() -> String {
    format!("{PAYMENT_SELECT_BASE} WHERE o.assigned_employee_id = $1 ORDER BY p.created_at DESC LIMIT 20")
}

fn payment_client_query() -> String {
    format!("{PAYMENT_SELECT_BASE} WHERE o.client_id = $1 ORDER BY p.created_at DESC LIMIT 20")
}
