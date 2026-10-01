use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::HostingSubscription;

use super::HostingRepository;
use super::types::ServerInfo;

impl HostingRepository {
    /* [245A-6] Guardar identidad del runtime y datos de acceso tras provisioning.
     * [104A-18] También guarda credenciales SFTP generadas al provisionar. */
    pub async fn update_server_info(
        pool: &PgPool,
        id: Uuid,
        info: &ServerInfo<'_>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE hosting_subscriptions
             SET runtime_kind = $1, deployment_id = $2,
                 coolify_site_name = $3, server_uuid = $4, server_ip = $5,
                 sftp_user = $6, sftp_password = $7, sftp_port = $8, updated_at = NOW()
             WHERE id = $9",
            info.runtime_kind,
            info.deployment_id,
            info.coolify_site_name,
            info.server_uuid,
            info.server_ip,
            info.sftp_user,
            info.sftp_password,
            info.sftp_port,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [164A-16] Genera un puerto SFTP único en rango 10000-49151 (evita efímeros 49152-65535).
     * Verifica unicidad en BD antes de retornar. Reintenta hasta 10 veces.
     * Requiere UNIQUE constraint en sftp_port (migration 20260417000000). */
    pub async fn find_available_sftp_port(pool: &PgPool) -> Result<i32, AppError> {
        for _ in 0..10 {
            let port: i32 = rand::thread_rng().gen_range(10000..49152);
            let taken = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM hosting_subscriptions WHERE sftp_port = $1) AS \"exists!\"",
                port
            )
            .fetch_one(pool)
            .await?;
            if !taken {
                return Ok(port);
            }
        }
        Err(AppError::Internal(
            "No se pudo encontrar un puerto SFTP disponible tras 10 intentos".into(),
        ))
    }

    /* [114A-1] Rotación de credenciales SFTP: actualiza contraseña y timestamp.
     * No toca el usuario ni el puerto — solo la contraseña.
     * El caller debe también actualizar el compose en Coolify para que surta efecto. */
    pub async fn update_sftp_password(
        pool: &PgPool,
        id: Uuid,
        new_password: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE hosting_subscriptions
             SET sftp_password = $1, sftp_credentials_rotated_at = NOW(), updated_at = NOW()
             WHERE id = $2",
            new_password,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_domain_verification(
        pool: &PgPool,
        id: Uuid,
        status: &str,
        token: Option<&str>,
        verified_at: Option<DateTime<Utc>>,
    ) -> Result<HostingSubscription, AppError> {
        let row = sqlx::query_as!(
            HostingSubscription,
            "UPDATE hosting_subscriptions
             SET domain_verification_status = $1,
                 domain_verification_token = $2,
                 domain_verified_at = $3,
                 updated_at = NOW()
             WHERE id = $4
             RETURNING id, user_id, client_name, client_email, plan, domain,
                       domain_verification_status, domain_verification_token, domain_verified_at,
                       runtime_kind, deployment_id,
                       coolify_site_name, status, stripe_subscription_id,
                       monthly_price_cents, storage_limit_mb,
                       server_uuid, server_ip, sftp_user, sftp_password, sftp_port, created_at, updated_at",
            status,
            token,
            verified_at,
            id
        )
        .fetch_one(pool)
        .await?;
        Ok(row)
    }
}
