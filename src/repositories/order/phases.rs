/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro:
 * phase_order_id usa runtime query (sin macro). */
//! Fases de orden: CRUD, máquina de estados entrega/aprobación y gestión manual.

use sqlx::PgPool;
use uuid::Uuid;

use super::{CreatePhaseParams, OrderRepository};
use crate::models::{OrderPhase, PhaseStatus};

impl OrderRepository {
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
}
