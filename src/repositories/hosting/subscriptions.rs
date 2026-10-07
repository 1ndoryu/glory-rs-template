use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{HostingEvent, HostingSubscription};

use super::types::{BootstrapHostingParams, CreateHostingParams, UpdateHostingParams};
use super::HostingRepository;

impl HostingRepository {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<HostingSubscription>, AppError> {
        let rows = sqlx::query_as!(
            HostingSubscription,
            "SELECT id, user_id, client_name, client_email, plan, domain,
                    domain_verification_status, domain_verification_token, domain_verified_at,
                    runtime_kind, deployment_id,
                    coolify_site_name, status, stripe_subscription_id,
                    monthly_price_cents, storage_limit_mb,
                    server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at
             FROM hosting_subscriptions
             ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [T-9] Listar suscripciones de hosting de un usuario específico */
    pub async fn list_by_user_id(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<HostingSubscription>, AppError> {
        let rows = sqlx::query_as!(
            HostingSubscription,
            "SELECT id, user_id, client_name, client_email, plan, domain,
                    domain_verification_status, domain_verification_token, domain_verified_at,
                    runtime_kind, deployment_id,
                    coolify_site_name, status, stripe_subscription_id,
                    monthly_price_cents, storage_limit_mb,
                    server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at
             FROM hosting_subscriptions
             WHERE user_id = $1
             ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<Option<HostingSubscription>, AppError> {
        let row = sqlx::query_as!(
            HostingSubscription,
            "SELECT id, user_id, client_name, client_email, plan, domain,
                    domain_verification_status, domain_verification_token, domain_verified_at,
                    runtime_kind, deployment_id,
                    coolify_site_name, status, stripe_subscription_id,
                    monthly_price_cents, storage_limit_mb,
                    server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at
             FROM hosting_subscriptions
             WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    pub async fn create(
        pool: &PgPool,
        params: CreateHostingParams<'_>,
    ) -> Result<HostingSubscription, AppError> {
        let row = sqlx::query_as!(
            HostingSubscription,
            "INSERT INTO hosting_subscriptions (
                user_id, client_name, client_email, plan, domain,
                domain_verification_status, domain_verification_token, domain_verified_at,
                runtime_kind, deployment_id, coolify_site_name,
                monthly_price_cents, storage_limit_mb
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             RETURNING id, user_id, client_name, client_email, plan, domain,
                       domain_verification_status, domain_verification_token, domain_verified_at,
                       runtime_kind, deployment_id,
                       coolify_site_name, status, stripe_subscription_id,
                       monthly_price_cents, storage_limit_mb,
                       server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at",
            params.user_id,
            params.client_name,
            params.client_email,
            params.plan,
            params.domain,
            params.domain_verification_status,
            params.domain_verification_token,
            params.domain_verified_at,
            params.runtime_kind,
            params.deployment_id,
            params.coolify_site_name,
            params.monthly_price_cents,
            params.storage_limit_mb
        )
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /* [07AA-2] Upsert de bootstrap por dominio (caso Guillermo): inserta o
     * reactiva la suscripción legacy con precio/límites fijos. El SQL vivía en
     * admin_client_bootstrap.rs (handler-accede-bd-rs); el repositorio es su
     * casa (DIP). Semántica ON CONFLICT preservada verbatim. */
    pub async fn upsert_bootstrap(
        pool: &PgPool,
        params: BootstrapHostingParams<'_>,
    ) -> Result<Uuid, AppError> {
        let id = sqlx::query_scalar!(
            r#"INSERT INTO hosting_subscriptions (
                    user_id, client_name, client_email, plan, domain,
                    domain_verification_status, domain_verified_at, coolify_site_name,
                    status, stripe_subscription_id, monthly_price_cents, storage_limit_mb,
                    server_uuid, server_ip
                )
                VALUES ($1, $2, $3, 'normal-basico', $4, 'active', $5, $6, 'active', $7, 248, 5120, $8, '66.94.100.241')
                ON CONFLICT (domain) DO UPDATE SET
                    user_id = EXCLUDED.user_id,
                    client_name = EXCLUDED.client_name,
                    client_email = EXCLUDED.client_email,
                    plan = EXCLUDED.plan,
                    domain_verification_status = 'active',
                    domain_verified_at = COALESCE(hosting_subscriptions.domain_verified_at, EXCLUDED.domain_verified_at),
                    coolify_site_name = EXCLUDED.coolify_site_name,
                    status = 'active',
                    stripe_subscription_id = COALESCE(hosting_subscriptions.stripe_subscription_id, EXCLUDED.stripe_subscription_id),
                    monthly_price_cents = EXCLUDED.monthly_price_cents,
                    storage_limit_mb = EXCLUDED.storage_limit_mb,
                    server_uuid = EXCLUDED.server_uuid,
                    server_ip = EXCLUDED.server_ip,
                    updated_at = NOW()
                RETURNING id"#,
            params.user_id,
            params.client_name,
            params.client_email,
            params.domain,
            params.verified_at,
            params.coolify_site_name,
            params.paid_subscription_id,
            params.server_uuid,
        )
        .fetch_one(pool)
        .await?;
        Ok(id)
    }

    pub async fn update_status(pool: &PgPool, id: Uuid, status: &str) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE hosting_subscriptions SET status = $1, updated_at = NOW() WHERE id = $2",
            status,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [074A-65] Actualizar campos editables de una suscripción (plan, dominio, precio, almacenamiento) */
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        params: UpdateHostingParams<'_>,
    ) -> Result<HostingSubscription, AppError> {
        let row = sqlx::query_as!(
            HostingSubscription,
            "UPDATE hosting_subscriptions
             SET plan = $1,
                 domain = $2,
                 monthly_price_cents = $3,
                 storage_limit_mb = $4,
                 domain_verification_status = $5,
                 domain_verification_token = $6,
                 domain_verified_at = $7,
                 updated_at = NOW()
             WHERE id = $8
             RETURNING id, user_id, client_name, client_email, plan, domain,
                       domain_verification_status, domain_verification_token, domain_verified_at,
                       runtime_kind, deployment_id,
                       coolify_site_name, status, stripe_subscription_id,
                       monthly_price_cents, storage_limit_mb,
                       server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at",
            params.plan,
            params.domain,
            params.monthly_price_cents,
            params.storage_limit_mb,
            params.domain_verification_status,
            params.domain_verification_token,
            params.domain_verified_at,
            id
        )
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /* [074A-65] Eliminar suscripción de hosting */
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM hosting_subscriptions WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn add_event(
        pool: &PgPool,
        subscription_id: Uuid,
        event_type: &str,
        details: Option<serde_json::Value>,
    ) -> Result<HostingEvent, AppError> {
        let row = sqlx::query_as!(
            HostingEvent,
            "INSERT INTO hosting_events (subscription_id, event_type, details)
             VALUES ($1, $2, $3)
             RETURNING id, subscription_id, event_type, details, created_at",
            subscription_id,
            event_type,
            details
        )
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    pub async fn list_events(
        pool: &PgPool,
        subscription_id: Uuid,
        limit: i64,
    ) -> Result<Vec<HostingEvent>, AppError> {
        let rows = sqlx::query_as!(
            HostingEvent,
            "SELECT id, subscription_id, event_type, details, created_at
             FROM hosting_events
             WHERE subscription_id = $1
             ORDER BY created_at DESC
             LIMIT $2",
            subscription_id,
            limit
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [084A-24] Buscar suscripción por stripe_subscription_id (para webhooks) */
    pub async fn find_by_stripe_subscription(
        pool: &PgPool,
        stripe_sub_id: &str,
    ) -> Result<Option<HostingSubscription>, AppError> {
        let row = sqlx::query_as!(
            HostingSubscription,
            "SELECT id, user_id, client_name, client_email, plan, domain,
                    domain_verification_status, domain_verification_token, domain_verified_at,
                    runtime_kind, deployment_id,
                    coolify_site_name, status, stripe_subscription_id,
                    monthly_price_cents, storage_limit_mb,
                    server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at
             FROM hosting_subscriptions
             WHERE stripe_subscription_id = $1",
            stripe_sub_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /* [304A-3] Asignar (o desasignar) una suscripción de hosting a un usuario.
     * Admin only. Permite vincular hostings creados manualmente a cuentas de clientes. */
    pub async fn assign_user(
        pool: &PgPool,
        id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<HostingSubscription, AppError> {
        let row = sqlx::query_as!(
            HostingSubscription,
            "UPDATE hosting_subscriptions
             SET user_id = $1, updated_at = NOW()
             WHERE id = $2
             RETURNING id, user_id, client_name, client_email, plan, domain,
                       domain_verification_status, domain_verification_token, domain_verified_at,
                       runtime_kind, deployment_id,
                       coolify_site_name, status, stripe_subscription_id,
                       monthly_price_cents, storage_limit_mb,
                       server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at",
            user_id,
            id
        )
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /* [084A-24] Asignar stripe_subscription_id a una suscripción de hosting */
    pub async fn set_stripe_subscription_id(
        pool: &PgPool,
        id: Uuid,
        stripe_sub_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE hosting_subscriptions SET stripe_subscription_id = $1, updated_at = NOW() WHERE id = $2",
            stripe_sub_id,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
