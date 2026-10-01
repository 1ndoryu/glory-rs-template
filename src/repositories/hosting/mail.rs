use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{HostingEmailAlias, HostingEmailMailbox};

use super::HostingRepository;

impl HostingRepository {
    /* [265A-11] Alias de correo — listar por suscripcion */
    pub async fn list_aliases(
        pool: &PgPool,
        subscription_id: Uuid,
    ) -> Result<Vec<HostingEmailAlias>, AppError> {
        let rows = sqlx::query_as!(
            HostingEmailAlias,
            "SELECT id, subscription_id, alias, domain, destination, status, created_at, updated_at
             FROM hosting_email_aliases
             WHERE subscription_id = $1
             ORDER BY alias ASC",
            subscription_id
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [265A-11] Crear alias de correo */
    pub async fn create_alias(
        pool: &PgPool,
        subscription_id: Uuid,
        alias: &str,
        domain: &str,
        destination: &str,
    ) -> Result<HostingEmailAlias, AppError> {
        let row = sqlx::query_as!(
            HostingEmailAlias,
            "INSERT INTO hosting_email_aliases (subscription_id, alias, domain, destination)
             VALUES ($1, $2, $3, $4)
             RETURNING id, subscription_id, alias, domain, destination, status, created_at, updated_at",
            subscription_id,
            alias,
            domain,
            destination
        )
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /* [265A-11] Eliminar alias de correo por id */
    pub async fn delete_alias(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM hosting_email_aliases WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /* [265A-11] Obtener alias por id (para verificar propiedad antes de eliminar) */
    pub async fn get_alias_by_id(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<Option<HostingEmailAlias>, AppError> {
        let row = sqlx::query_as!(
            HostingEmailAlias,
            "SELECT id, subscription_id, alias, domain, destination, status, created_at, updated_at
             FROM hosting_email_aliases
             WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /* [265A-11] Contar aliases activos de una suscripcion */
    pub async fn count_active_aliases(
        pool: &PgPool,
        subscription_id: Uuid,
    ) -> Result<i64, AppError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM hosting_email_aliases
             WHERE subscription_id = $1 AND status = 'active'",
            subscription_id
        )
        .fetch_one(pool)
        .await?;
        Ok(count.unwrap_or(0))
    }

    /* [265A-12] Preparado para Fase 2 — buzones IMAP (NO activa aun) */
    #[allow(dead_code)]
    pub async fn list_mailboxes(
        pool: &PgPool,
        subscription_id: Uuid,
    ) -> Result<Vec<HostingEmailMailbox>, AppError> {
        let rows = sqlx::query_as!(
            HostingEmailMailbox,
            "SELECT id, subscription_id, email, password_hash, provider,
                    provider_mailbox_id, status, storage_used_mb, created_at, updated_at
             FROM hosting_email_mailboxes
             WHERE subscription_id = $1
             ORDER BY email ASC",
            subscription_id
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [265A-12] Obtener included_aliases del plan de una suscripcion */
    pub async fn get_plan_included_aliases(
        pool: &PgPool,
        plan_name: &str,
    ) -> Result<i32, AppError> {
        let row = sqlx::query_scalar!(
            "SELECT included_aliases FROM hosting_plan_configs WHERE plan_name = $1",
            plan_name
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.unwrap_or(0))
    }

    /* [265A-12] Obtener included_mailboxes del plan de una suscripcion */
    pub async fn get_plan_included_mailboxes(
        pool: &PgPool,
        plan_name: &str,
    ) -> Result<i32, AppError> {
        let row = sqlx::query_scalar!(
            "SELECT included_mailboxes FROM hosting_plan_configs WHERE plan_name = $1",
            plan_name
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.unwrap_or(0))
    }
}
