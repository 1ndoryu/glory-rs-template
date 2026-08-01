/* wandori.us — Commerce repositories
 * Persistencia de idempotencia, eventos de Stripe, entitlements y outbox.
 * La autoridad de acceso permanece en PostgreSQL; ningún token se decide en
 * el navegador. [297A-15] */

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct DownloadGrant {
    pub file_path: Option<String>,
    pub product_name: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
}

pub struct StripeEventRepository;

impl StripeEventRepository {
    /// Reclama un evento del proveedor. Un evento procesado previamente se
    /// ignora; uno fallido (`processed_at` NULL) puede reintentarse.
    pub async fn begin(
        pool: &PgPool,
        provider_event_id: &str,
        event_type: &str,
        payload: &Value,
    ) -> Result<bool, sqlx::Error> {
        let inserted = sqlx::query(
            "INSERT INTO stripe_events (provider_event_id, event_type, payload) \
             VALUES ($1, $2, $3) ON CONFLICT (provider_event_id) DO NOTHING",
        )
        .bind(provider_event_id)
        .bind(event_type)
        .bind(payload)
        .execute(pool)
        .await?
        .rows_affected();

        if inserted > 0 {
            return Ok(true);
        }

        let processed: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT processed_at FROM stripe_events WHERE provider_event_id = $1",
        )
        .bind(provider_event_id)
        .fetch_optional(pool)
        .await?
        .flatten();
        Ok(processed.is_none())
    }

    pub async fn mark_processed(pool: &PgPool, provider_event_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE stripe_events SET processed_at = NOW() WHERE provider_event_id = $1")
            .bind(provider_event_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

pub struct ProductVersionRepository;

impl ProductVersionRepository {
    pub async fn latest_for_product(
        pool: &PgPool,
        product_id: Uuid,
    ) -> Result<Option<(Uuid, String)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, file_path FROM product_versions \
             WHERE product_id = $1 ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(product_id)
        .fetch_optional(pool)
        .await
    }
}

pub struct EntitlementRepository;

impl EntitlementRepository {
    pub async fn create_active(
        pool: &PgPool,
        order_id: Uuid,
        product_id: Uuid,
        product_version_id: Option<Uuid>,
        customer_email: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, sqlx::Error> {
        let inserted = sqlx::query(
            "INSERT INTO entitlements \
             (order_id, product_id, product_version_id, customer_email, token_hash, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (order_id) DO NOTHING",
        )
        .bind(order_id)
        .bind(product_id)
        .bind(product_version_id)
        .bind(customer_email)
        .bind(token_hash)
        .bind(expires_at)
        .execute(pool)
        .await?
        .rows_affected();
        Ok(inserted > 0)
    }

    pub async fn find_by_token_hash(
        pool: &PgPool,
        token_hash: &str,
    ) -> Result<Option<DownloadGrant>, sqlx::Error> {
        sqlx::query_as::<_, DownloadGrant>(
            "SELECT COALESCE(pv.file_path, p.download_path) AS file_path, \
                    p.name AS product_name, e.status, e.expires_at \
             FROM entitlements e \
             INNER JOIN products p ON p.id = e.product_id \
             LEFT JOIN product_versions pv ON pv.id = e.product_version_id \
             WHERE e.token_hash = $1 LIMIT 1",
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await
    }

    pub async fn expire(pool: &PgPool, token_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE entitlements SET status = 'expired' \
             WHERE token_hash = $1 AND status = 'active'",
        )
        .bind(token_hash)
        .execute(pool)
        .await?;
        Ok(())
    }
}

pub struct CommerceOutboxRepository;

impl CommerceOutboxRepository {
    pub async fn enqueue(
        pool: &PgPool,
        event_type: &str,
        aggregate_id: Uuid,
        dedupe_key: &str,
        payload: &Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO commerce_outbox (event_type, aggregate_id, dedupe_key, payload) \
             VALUES ($1, $2, $3, $4) ON CONFLICT (dedupe_key) DO NOTHING",
        )
        .bind(event_type)
        .bind(aggregate_id)
        .bind(dedupe_key)
        .bind(payload)
        .execute(pool)
        .await?;
        Ok(())
    }
}
