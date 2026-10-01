/* [259A-4e] Tools IA de reportes y resumen admin: `exec_list_my_reports`,
 * `exec_create_order_report` y `exec_admin_operational_summary` con queries
 * por alcance de rol. */

/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: queries runtime por rol.
 * Las consultas se construyen por alcance (admin/empleado/cliente) intencionalmente
 * para no depender de caché offline en consultas dinámicas por rol (259A-4e). */

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::UserRole;
use crate::repositories::ProblemRepository;

use super::ai_tools::{
    can_access_order, forbidden, not_found, require_auth, tool_json, tool_status, ToolAuthContext,
    ToolExecResult,
};

#[derive(sqlx::FromRow)]
struct ToolReportRow {
    id: Uuid,
    order_id: Uuid,
    order_number: i32,
    reporter_id: Uuid,
    reporter_name: String,
    reporter_role: String,
    reason: String,
    status: String,
    admin_response: Option<String>,
    created_at: DateTime<Utc>,
}

pub(crate) async fn exec_list_my_reports(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    match query_reports_for_scope(pool, auth).await {
        Ok(reports) => tool_json(json!({
            "status": "ok",
            "scope": auth.effective_role.to_string(),
            "reports": reports.iter().map(report_row_json).collect::<Vec<_>>(),
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando reportes para AI tool: {e}"
            );
            tool_status("error", "No se pudieron consultar los reportes")
        }
    }
}

async fn query_reports_for_scope(
    pool: &PgPool,
    auth: ToolAuthContext,
) -> Result<Vec<ToolReportRow>, sqlx::Error> {
    match auth.effective_role {
        UserRole::Admin => {
            let query = report_admin_query();
            sqlx::query_as::<_, ToolReportRow>(&query)
                .fetch_all(pool)
                .await
        }
        UserRole::Employee => {
            let query = report_employee_query();
            sqlx::query_as::<_, ToolReportRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
        UserRole::Client => {
            let query = report_client_query();
            sqlx::query_as::<_, ToolReportRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
    }
}

fn report_row_json(row: &ToolReportRow) -> Value {
    json!({
        "id": row.id,
        "order_id": row.order_id,
        "order_number": row.order_number,
        "reporter_id": row.reporter_id,
        "reporter_name": row.reporter_name,
        "reporter_role": row.reporter_role,
        "reason": row.reason,
        "status": row.status,
        "admin_response": row.admin_response,
        "created_at": row.created_at,
    })
}

const REPORT_SELECT_BASE: &str = "
SELECT prob.id,
       prob.order_id,
       o.order_number,
       prob.reporter_id,
       COALESCE(reporter.display_name, reporter.email) AS reporter_name,
       prob.reporter_role,
       prob.reason,
       prob.status::text AS status,
       prob.admin_response,
       prob.created_at
FROM order_problems prob
JOIN orders o ON o.id = prob.order_id
JOIN users reporter ON reporter.id = prob.reporter_id";

fn report_admin_query() -> String {
    format!("{REPORT_SELECT_BASE} ORDER BY prob.created_at DESC LIMIT 20")
}

fn report_employee_query() -> String {
    format!("{REPORT_SELECT_BASE} WHERE o.assigned_employee_id = $1 ORDER BY prob.created_at DESC LIMIT 20")
}

fn report_client_query() -> String {
    format!("{REPORT_SELECT_BASE} WHERE o.client_id = $1 ORDER BY prob.created_at DESC LIMIT 20")
}

#[derive(sqlx::FromRow)]
struct OrderAccessRow {
    id: Uuid,
    order_number: i32,
    client_id: Uuid,
    assigned_employee_id: Option<Uuid>,
}

pub(crate) async fn exec_create_order_report(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
    args: &Value,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    let reason = args["reason"].as_str().unwrap_or("").trim();
    if !(10..=2000).contains(&reason.chars().count()) {
        return tool_status(
            "error",
            "El reporte necesita una descripción entre 10 y 2000 caracteres.",
        );
    }
    let order = match resolve_order_for_report(pool, auth, args).await {
        Ok(order) => order,
        Err(result) => return result,
    };
    match ProblemRepository::create(
        pool,
        order.id,
        auth.user_id,
        &auth.effective_role.to_string(),
        reason,
    )
    .await
    {
        Ok(problem) => tool_json(json!({
            "status": "ok",
            "report_id": problem.id,
            "order_id": order.id,
            "order_number": order.order_number,
            "report_status": problem.status,
            "message": "Reporte creado. El equipo lo revisará desde el panel."
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                order_id = %order.id,
                "Error creando reporte desde AI tool: {e}"
            );
            tool_status("error", "No se pudo crear el reporte")
        }
    }
}

async fn resolve_order_for_report(
    pool: &PgPool,
    auth: ToolAuthContext,
    args: &Value,
) -> Result<OrderAccessRow, ToolExecResult> {
    let row = if let Some(order_id) = args["order_id"]
        .as_str()
        .and_then(|v| Uuid::parse_str(v).ok())
    {
        find_order_access_by_id(pool, order_id).await
    } else if let Some(order_number) = args["order_number"]
        .as_i64()
        .and_then(|v| i32::try_from(v).ok())
    {
        find_order_access_by_number(pool, order_number).await
    } else {
        return Err(tool_status(
            "error",
            "Indica order_id u order_number para crear el reporte.",
        ));
    }
    .map_err(|e| {
        tracing::error!("Error resolviendo pedido para reporte AI: {e}");
        tool_status("error", "No se pudo verificar el pedido")
    })?
    .ok_or_else(|| not_found("Pedido no encontrado"))?;

    if can_access_order(auth, row.client_id, row.assigned_employee_id) {
        Ok(row)
    } else {
        Err(forbidden(
            "No tienes permisos para crear reportes sobre ese pedido.",
        ))
    }
}

async fn find_order_access_by_id(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Option<OrderAccessRow>, sqlx::Error> {
    sqlx::query_as::<_, OrderAccessRow>(ORDER_ACCESS_SELECT_ID)
        .bind(order_id)
        .fetch_optional(pool)
        .await
}

async fn find_order_access_by_number(
    pool: &PgPool,
    order_number: i32,
) -> Result<Option<OrderAccessRow>, sqlx::Error> {
    sqlx::query_as::<_, OrderAccessRow>(ORDER_ACCESS_SELECT_NUMBER)
        .bind(order_number)
        .fetch_optional(pool)
        .await
}

const ORDER_ACCESS_SELECT_ID: &str = "
SELECT id, order_number, client_id, assigned_employee_id
FROM orders
WHERE id = $1";
const ORDER_ACCESS_SELECT_NUMBER: &str = "
SELECT id, order_number, client_id, assigned_employee_id
FROM orders
WHERE order_number = $1";

#[derive(sqlx::FromRow)]
struct AdminOrderStats {
    total: i64,
    payment_held: i64,
    awaiting_assignment: i64,
    in_progress: i64,
    under_review: i64,
    completed: i64,
    disputed: i64,
}

#[derive(sqlx::FromRow)]
struct AdminReportStats {
    open_reports: i64,
    in_review_reports: i64,
}

#[derive(sqlx::FromRow)]
struct StatusCountRow {
    status: String,
    total: i64,
}

pub(crate) async fn exec_admin_operational_summary(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    if !auth.is_effective_admin() {
        return forbidden(
            "Solo un administrador efectivo puede consultar el resumen operativo global.",
        );
    }

    let order_stats = match query_admin_order_stats(pool).await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::error!(user_id = %auth.user_id, "Error consultando stats admin de pedidos: {e}");
            return tool_status("error", "No se pudo consultar el resumen operativo");
        }
    };
    /* [01AA-3] stats de reportes y hosting son independientes entre sí
     * (ambas toleran error con fallback) → una sola ronda con join!. */
    let (report_stats_result, hosting_counts_result) = tokio::join!(
        query_admin_report_stats(pool),
        query_hosting_status_counts(pool)
    );
    let report_stats = report_stats_result.unwrap_or(AdminReportStats {
        open_reports: 0,
        in_review_reports: 0,
    });
    let hosting_by_status = hosting_counts_result.unwrap_or_default();

    tool_json(json!({
        "status": "ok",
        "orders": {
            "total": order_stats.total,
            "payment_held": order_stats.payment_held,
            "awaiting_assignment": order_stats.awaiting_assignment,
            "in_progress": order_stats.in_progress,
            "under_review": order_stats.under_review,
            "completed": order_stats.completed,
            "disputed": order_stats.disputed,
        },
        "reports": {
            "open": report_stats.open_reports,
            "in_review": report_stats.in_review_reports,
        },
        "hosting_by_status": hosting_by_status.into_iter().map(|row| json!({
            "status": row.status,
            "total": row.total,
        })).collect::<Vec<_>>(),
    }))
}

async fn query_admin_order_stats(pool: &PgPool) -> Result<AdminOrderStats, sqlx::Error> {
    sqlx::query_as::<_, AdminOrderStats>(
        "SELECT COUNT(*) AS total,
            COUNT(*) FILTER (WHERE status = 'payment_held') AS payment_held,
            COUNT(*) FILTER (WHERE status = 'awaiting_assignment') AS awaiting_assignment,
            COUNT(*) FILTER (WHERE status = 'in_progress') AS in_progress,
            COUNT(*) FILTER (WHERE status = 'under_review') AS under_review,
            COUNT(*) FILTER (WHERE status = 'completed') AS completed,
            COUNT(*) FILTER (WHERE status = 'disputed') AS disputed
         FROM orders",
    )
    .fetch_one(pool)
    .await
}

async fn query_admin_report_stats(pool: &PgPool) -> Result<AdminReportStats, sqlx::Error> {
    sqlx::query_as::<_, AdminReportStats>(
        "SELECT COUNT(*) FILTER (WHERE status = 'open') AS open_reports,
                COUNT(*) FILTER (WHERE status = 'in_review') AS in_review_reports
         FROM order_problems",
    )
    .fetch_one(pool)
    .await
}

async fn query_hosting_status_counts(pool: &PgPool) -> Result<Vec<StatusCountRow>, sqlx::Error> {
    sqlx::query_as::<_, StatusCountRow>(
        "SELECT status, COUNT(*) AS total
         FROM hosting_subscriptions
         GROUP BY status
         ORDER BY status",
    )
    .fetch_all(pool)
    .await
}
