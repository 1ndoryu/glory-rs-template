/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: repositorio nuevo con filtros opcionales.
 * [205A-1] Queries preparadas con bind para cobros pendientes del panel. */
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::BillingItem;

pub struct BillingRepository;

/* [07AA-2] Params del upsert bootstrap de cobros (caso Guillermo): 12 campos
 * agrupados en struct. El SQL vivía en admin_client_bootstrap.rs. */
pub struct BootstrapBillingItemParams<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub resource_type: &'a str,
    pub resource_id: Option<Uuid>,
    pub title: &'a str,
    pub description: &'a str,
    pub amount_cents: i32,
    pub billing_period: &'a str,
    pub due_at: DateTime<Utc>,
    pub grace_period_ends_at: DateTime<Utc>,
    pub metadata: Value,
    pub initial_status: &'a str,
}

impl BillingRepository {
    pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<BillingItem>, AppError> {
        sqlx::query_as::<_, BillingItem>(
            r"SELECT id, user_id, resource_type, resource_id, title, description,
                      amount_cents, currency, billing_period, status, due_at,
                      grace_period_ends_at, paid_at, stripe_session_id, metadata,
                      created_at, updated_at
              FROM billing_items
              WHERE user_id = $1
              ORDER BY CASE WHEN status = 'pending' THEN 0 ELSE 1 END, due_at ASC, created_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn pending_for_checkout(
        pool: &PgPool,
        user_id: Uuid,
        item_ids: Option<&[Uuid]>,
    ) -> Result<Vec<BillingItem>, AppError> {
        let ids: Option<Vec<Uuid>> = item_ids.map(<[Uuid]>::to_vec);
        sqlx::query_as::<_, BillingItem>(
            r"SELECT id, user_id, resource_type, resource_id, title, description,
                      amount_cents, currency, billing_period, status, due_at,
                      grace_period_ends_at, paid_at, stripe_session_id, metadata,
                      created_at, updated_at
              FROM billing_items
              WHERE user_id = $1
                AND status = 'pending'
                AND ($2::uuid[] IS NULL OR id = ANY($2))
              ORDER BY due_at ASC, created_at ASC",
        )
        .bind(user_id)
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    /* [07AA-2] Upsert de cobro bootstrap por id (caso Guillermo): inserta o
     * actualiza sin revertir pagos. El SQL vivía en admin_client_bootstrap.rs
     * (handler-accede-bd-rs); el repositorio es su casa (DIP).
     * [205A-4] $12 = initial_status ('paid' | 'pending').
     * ON CONFLICT: si el item ya es 'paid' O el bootstrap lo marca paid, queda paid.
     * Esto permite re-ejecutar el bootstrap sin revertir pagos ya registrados
     * y sin crear deuda falsa para cobros ya saldados (e.g. guillechatbots.es). */
    pub async fn upsert_bootstrap_item(
        pool: &PgPool,
        params: BootstrapBillingItemParams<'_>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"INSERT INTO billing_items (
                    id, user_id, resource_type, resource_id, title, description,
                    amount_cents, currency, billing_period, status, due_at,
                    grace_period_ends_at, metadata
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, 'USD', $8, $12, $9, $10, $11)
                ON CONFLICT (id) DO UPDATE SET
                    user_id = EXCLUDED.user_id,
                    resource_type = EXCLUDED.resource_type,
                    resource_id = EXCLUDED.resource_id,
                    title = EXCLUDED.title,
                    description = EXCLUDED.description,
                    amount_cents = EXCLUDED.amount_cents,
                    currency = EXCLUDED.currency,
                    billing_period = EXCLUDED.billing_period,
                    status = CASE
                        WHEN $12 = 'paid' OR billing_items.status = 'paid' THEN 'paid'
                        ELSE 'pending'
                    END,
                    due_at = EXCLUDED.due_at,
                    grace_period_ends_at = EXCLUDED.grace_period_ends_at,
                    paid_at = CASE
                        WHEN $12 = 'paid' THEN COALESCE(billing_items.paid_at, NOW())
                        WHEN billing_items.status = 'paid' THEN billing_items.paid_at
                        ELSE NULL
                    END,
                    stripe_session_id = CASE WHEN billing_items.status = 'paid' THEN billing_items.stripe_session_id ELSE NULL END,
                    metadata = EXCLUDED.metadata,
                    updated_at = NOW()"#,
            params.id,
            params.user_id,
            params.resource_type,
            params.resource_id,
            params.title,
            params.description,
            params.amount_cents,
            params.billing_period,
            params.due_at,
            params.grace_period_ends_at,
            params.metadata,
            params.initial_status,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_checkout_session(
        pool: &PgPool,
        item_ids: &[Uuid],
        stripe_session_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"UPDATE billing_items
              SET stripe_session_id = $1, updated_at = NOW()
              WHERE id = ANY($2) AND status = 'pending'",
        )
        .bind(stripe_session_id)
        .bind(item_ids)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn mark_paid_by_session(
        pool: &PgPool,
        stripe_session_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"UPDATE billing_items
              SET status = 'paid', paid_at = COALESCE(paid_at, NOW()), updated_at = NOW()
              WHERE stripe_session_id = $1 AND status = 'pending'",
        )
        .bind(stripe_session_id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    /* [026B-1] Admin: listar todos los billing_items con email del usuario. */
    pub async fn list_all_for_admin(
        pool: &PgPool,
        status_filter: Option<&str>,
    ) -> Result<Vec<AdminBillingItem>, AppError> {
        sqlx::query_as::<_, AdminBillingItem>(
            r"SELECT bi.id, bi.user_id, u.email AS user_email, bi.resource_type,
                      bi.resource_id, bi.title, bi.description,
                      bi.amount_cents, bi.currency, bi.billing_period, bi.status,
                      bi.due_at, bi.grace_period_ends_at, bi.paid_at,
                      bi.stripe_session_id, bi.metadata,
                      bi.created_at, bi.updated_at
              FROM billing_items bi
              JOIN users u ON u.id = bi.user_id
              WHERE ($1::text IS NULL OR bi.status = $1)
              ORDER BY CASE WHEN bi.status = 'pending' THEN 0 ELSE 1 END,
                       bi.due_at ASC, bi.created_at DESC",
        )
        .bind(status_filter)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    /* [026B-1] Admin: actualizar status de un billing_item (paid ↔ pending). */
    pub async fn admin_update_status(
        pool: &PgPool,
        item_id: Uuid,
        new_status: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"UPDATE billing_items
              SET status = $1,
                  paid_at = CASE WHEN $1 = 'paid' THEN NOW() ELSE NULL END,
                  updated_at = NOW()
              WHERE id = $2",
        )
        .bind(new_status)
        .bind(item_id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }
}

/* [026B-1] BillingItem enriquecido con email del usuario (admin view). */
#[derive(Debug, FromRow)]
pub struct AdminBillingItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub amount_cents: i32,
    pub currency: String,
    pub billing_period: String,
    pub status: String,
    pub due_at: chrono::DateTime<chrono::Utc>,
    pub grace_period_ends_at: chrono::DateTime<chrono::Utc>,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
    pub stripe_session_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
