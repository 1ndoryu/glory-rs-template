/* [044A-38 Fase 7] Repositorio de reembolsos.
 * [277A-7] Migrado a queries runtime (sqlx::query_as sin macro) porque las
 * columnas nuevas (attempts, max_attempts, next_retry_at) aún no tienen cache
 * en .sqlx/. Tras deploy + `cargo sqlx prepare`, se puede migrar de vuelta a
 * query_as! si se desea verificación en compilación.
 * CRUD sobre order_refunds: crear solicitud, listar, aprobar/rechazar/completar.
 * Solo un reembolso activo por orden (constraint en handler). */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::OrderRefund;

/* sentinel-disable-file sqlx-query-sin-macro: columnas nuevas sin cache .sqlx requieren queries runtime */

const SELECT_COLS: &str = r#"id, order_id, payment_id, requested_by, reviewed_by,
    amount_cents, reason, admin_response,
    status as "status: RefundStatus",
    stripe_refund_id, attempts, max_attempts, next_retry_at,
    requested_at, reviewed_at, completed_at"#;

pub struct RefundRepository;

impl RefundRepository {
    /// Crear solicitud de reembolso
    pub async fn create(
        pool: &PgPool,
        order_id: Uuid,
        payment_id: Uuid,
        requested_by: Uuid,
        amount_cents: i32,
        reason: &str,
    ) -> Result<OrderRefund, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            r#"INSERT INTO order_refunds (order_id, payment_id, requested_by, amount_cents, reason)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING {SELECT_COLS}"#
        ))
        .bind(order_id)
        .bind(payment_id)
        .bind(requested_by)
        .bind(amount_cents)
        .bind(reason)
        .fetch_one(pool)
        .await
    }

    /// Buscar reembolso por ID
    pub async fn find_by_id(
        pool: &PgPool,
        refund_id: Uuid,
    ) -> Result<Option<OrderRefund>, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "SELECT {SELECT_COLS} FROM order_refunds WHERE id = $1"
        ))
        .bind(refund_id)
        .fetch_optional(pool)
        .await
    }

    /// Buscar reembolso activo (no rejected/completed/failed) para una orden
    pub async fn find_active_for_order(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<OrderRefund>, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "SELECT {SELECT_COLS} FROM order_refunds
             WHERE order_id = $1
               AND status NOT IN ('rejected', 'completed', 'failed')
             ORDER BY requested_at DESC
             LIMIT 1"
        ))
        .bind(order_id)
        .fetch_optional(pool)
        .await
    }

    /// Listar todos los reembolsos pendientes (admin)
    pub async fn list_pending(pool: &PgPool) -> Result<Vec<OrderRefund>, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "SELECT {SELECT_COLS} FROM order_refunds
             WHERE status IN ('requested', 'under_review', 'approved', 'processing', 'failed')
             ORDER BY requested_at ASC"
        ))
        .fetch_all(pool)
        .await
    }

    /// Listar reembolsos de un cliente específico
    pub async fn list_for_user(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<OrderRefund>, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "SELECT {SELECT_COLS} FROM order_refunds
             WHERE requested_by = $1
             ORDER BY requested_at DESC"
        ))
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /// Buscar reembolso de una orden
    pub async fn find_for_order(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<OrderRefund>, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "SELECT {SELECT_COLS} FROM order_refunds
             WHERE order_id = $1
             ORDER BY requested_at DESC
             LIMIT 1"
        ))
        .bind(order_id)
        .fetch_optional(pool)
        .await
    }

    /// Aprobar reembolso (admin)
    pub async fn approve(
        pool: &PgPool,
        refund_id: Uuid,
        admin_id: Uuid,
        admin_response: Option<&str>,
    ) -> Result<OrderRefund, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "UPDATE order_refunds
             SET status = 'approved',
                 reviewed_by = $2,
                 admin_response = $3,
                 reviewed_at = NOW()
             WHERE id = $1
             RETURNING {SELECT_COLS}"
        ))
        .bind(refund_id)
        .bind(admin_id)
        .bind(admin_response)
        .fetch_one(pool)
        .await
    }

    /// Rechazar reembolso (admin)
    pub async fn reject(
        pool: &PgPool,
        refund_id: Uuid,
        admin_id: Uuid,
        admin_response: Option<&str>,
    ) -> Result<OrderRefund, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "UPDATE order_refunds
             SET status = 'rejected',
                 reviewed_by = $2,
                 admin_response = $3,
                 reviewed_at = NOW()
             WHERE id = $1
             RETURNING {SELECT_COLS}"
        ))
        .bind(refund_id)
        .bind(admin_id)
        .bind(admin_response)
        .fetch_one(pool)
        .await
    }

    /// Marcar reembolso como completado (Stripe refund exitoso)
    pub async fn mark_completed(
        pool: &PgPool,
        refund_id: Uuid,
        stripe_refund_id: &str,
    ) -> Result<OrderRefund, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "UPDATE order_refunds
             SET status = 'completed',
                 stripe_refund_id = $2,
                 completed_at = NOW()
             WHERE id = $1
             RETURNING {SELECT_COLS}"
        ))
        .bind(refund_id)
        .bind(stripe_refund_id)
        .fetch_one(pool)
        .await
    }

    /// Actualizar status a `under_review`
    pub async fn set_under_review(
        pool: &PgPool,
        refund_id: Uuid,
    ) -> Result<OrderRefund, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "UPDATE order_refunds
             SET status = 'under_review'
             WHERE id = $1
             RETURNING {SELECT_COLS}"
        ))
        .bind(refund_id)
        .fetch_one(pool)
        .await
    }

    /* [277A-7] Buscar reembolsos que necesitan retry:
     * 1. failed con next_retry_at alcanzado y attempts < max_attempts
     * 2. processing stuck >30min (proceso crash antes de completar) */
    pub async fn find_pending_retry(
        pool: &PgPool,
    ) -> Result<Vec<OrderRefund>, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "SELECT {SELECT_COLS} FROM order_refunds
             WHERE (status = 'failed'
                    AND attempts < max_attempts
                    AND (next_retry_at IS NULL OR next_retry_at <= NOW()))
                OR (status = 'processing'
                    AND updated_at < NOW() - INTERVAL '30 minutes')
             ORDER BY requested_at ASC"
        ))
        .fetch_all(pool)
        .await
    }

    /* [277A-7] Marcar reembolso como processing y registrar intento. */
    pub async fn mark_processing(
        pool: &PgPool,
        refund_id: Uuid,
        current_attempts: i32,
    ) -> Result<OrderRefund, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "UPDATE order_refunds
             SET status = 'processing',
                 attempts = $2,
                 next_retry_at = NULL
             WHERE id = $1
             RETURNING {SELECT_COLS}"
        ))
        .bind(refund_id)
        .bind(current_attempts)
        .fetch_one(pool)
        .await
    }

    /* [277A-7] Marcar reembolso como failed con next_retry_at explícito.
     * El caller calcula el backoff y pasa None si se agotaron los intentos. */
    pub async fn mark_failed_with_retry(
        pool: &PgPool,
        refund_id: Uuid,
        current_attempts: i32,
        max_attempts: i32,
        next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<OrderRefund, sqlx::Error> {
        sqlx::query_as::<_, OrderRefund>(&format!(
            "UPDATE order_refunds
             SET status = 'failed',
                 attempts = $2,
                 max_attempts = $3,
                 next_retry_at = $4
             WHERE id = $1
             RETURNING {SELECT_COLS}"
        ))
        .bind(refund_id)
        .bind(current_attempts)
        .bind(max_attempts)
        .bind(next_retry_at)
        .fetch_one(pool)
        .await
    }
}
