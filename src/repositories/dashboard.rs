/* [044A-38 Fase 10] Repositorio del dashboard admin.
 * Queries analíticas para métricas, revenue y alertas.
 * Cada query es independiente para poder ejecutarlas en paralelo con tokio::join!. */

use sqlx::PgPool;

use crate::errors::AppError;

/* Row helpers para query_as! (tipos planos que mapean resultado SQL) */

#[derive(Debug, sqlx::FromRow)]
pub struct RevenueRow {
    pub total_revenue: Option<f64>,
    pub monthly_revenue: Option<f64>,
    pub held_amount: Option<f64>,
    pub refunded_amount: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrderCountsRow {
    pub total: Option<i64>,
    pub active: Option<i64>,
    pub completed: Option<i64>,
    pub cancelled: Option<i64>,
    pub awaiting_assignment: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EmployeeRow {
    pub employee_id: uuid::Uuid,
    pub email: String,
    pub active_orders: Option<i64>,
    pub completed_orders: Option<i64>,
    pub average_rating: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct AlertsRow {
    pub unassigned_orders: Option<i64>,
    pub pending_refunds: Option<i64>,
    pub overdue_orders: Option<i64>,
}

pub struct DashboardRepository;

impl DashboardRepository {
    /// Revenue: total capturado, mensual, retenido, reembolsado.
    /// Usa CTE para calcular los totales en una sola query.
    pub async fn get_revenue(pool: &PgPool) -> Result<RevenueRow, AppError> {
        let row = sqlx::query_as!(
            RevenueRow,
            r#"SELECT
                COALESCE(SUM(CASE WHEN status = 'released' THEN CAST(amount_cents AS DOUBLE PRECISION) / 100.0 ELSE 0 END), 0) AS "total_revenue: f64",
                COALESCE(SUM(CASE WHEN status = 'released' AND created_at >= date_trunc('month', NOW()) THEN CAST(amount_cents AS DOUBLE PRECISION) / 100.0 ELSE 0 END), 0) AS "monthly_revenue: f64",
                COALESCE(SUM(CASE WHEN status = 'held' THEN CAST(amount_cents AS DOUBLE PRECISION) / 100.0 ELSE 0 END), 0) AS "held_amount: f64",
                COALESCE(SUM(CASE WHEN status = 'refunded' THEN CAST(amount_cents AS DOUBLE PRECISION) / 100.0 ELSE 0 END), 0) AS "refunded_amount: f64"
            FROM order_payments"#
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo revenue: {e}")))?;

        Ok(row)
    }

    /// Contadores de órdenes por estado
    pub async fn get_order_counts(pool: &PgPool) -> Result<OrderCountsRow, AppError> {
        let row = sqlx::query_as!(
            OrderCountsRow,
            r#"SELECT
                COUNT(*) AS "total: i64",
                COUNT(*) FILTER (WHERE status IN ('in_progress', 'under_review', 'payment_held')) AS "active: i64",
                COUNT(*) FILTER (WHERE status = 'completed') AS "completed: i64",
                COUNT(*) FILTER (WHERE status = 'cancelled') AS "cancelled: i64",
                COUNT(*) FILTER (WHERE status = 'awaiting_assignment') AS "awaiting_assignment: i64"
            FROM orders"#
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo conteos: {e}")))?;

        Ok(row)
    }

    /// Rendimiento de empleados: órdenes activas, completadas, rating promedio
    pub async fn get_employee_performance(pool: &PgPool) -> Result<Vec<EmployeeRow>, AppError> {
        let rows = sqlx::query_as!(
            EmployeeRow,
            r#"SELECT
                u.id AS "employee_id!",
                u.email AS "email!",
                COALESCE(
                    (SELECT COUNT(*) FROM orders o WHERE o.assigned_employee_id = u.id
                     AND o.status IN ('in_progress', 'under_review', 'payment_held')),
                    0
                ) AS "active_orders: i64",
                COALESCE(
                    (SELECT COUNT(*) FROM orders o WHERE o.assigned_employee_id = u.id
                     AND o.status = 'completed'),
                    0
                ) AS "completed_orders: i64",
                ep.average_rating AS "average_rating: f64"
            FROM users u
            LEFT JOIN employee_profiles ep ON ep.user_id = u.id
            WHERE u.role = 'employee'
            ORDER BY u.email"#
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo empleados: {e}")))?;

        Ok(rows)
    }

    /// Alertas: órdenes sin asignar, reembolsos pendientes, órdenes vencidas
    pub async fn get_alerts(pool: &PgPool) -> Result<AlertsRow, AppError> {
        let row = sqlx::query_as!(
            AlertsRow,
            r#"SELECT
                (SELECT COUNT(*) FROM orders WHERE status = 'awaiting_assignment') AS "unassigned_orders: i64",
                (SELECT COUNT(*) FROM order_refunds WHERE status IN ('requested', 'under_review')) AS "pending_refunds: i64",
                (SELECT COUNT(*) FROM orders
                 WHERE status = 'awaiting_assignment'
                 AND auto_assign_deadline IS NOT NULL
                 AND auto_assign_deadline < NOW()) AS "overdue_orders: i64""#
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo alertas: {e}")))?;

        Ok(row)
    }
}
