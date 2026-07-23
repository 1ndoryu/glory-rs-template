/* [044A-38 Fase 9] Repositorio de notificaciones.
 * CRUD con query_as! para verificación en compilación.
 * Todas las queries usan prepared statements (seguridad SQL). */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{CreateNotification, Notification};

pub struct NotificationRepository;

impl NotificationRepository {
    /// Crea una notificación y la retorna
    pub async fn create(
        pool: &PgPool,
        params: &CreateNotification,
    ) -> Result<Notification, AppError> {
        let row = sqlx::query_as!(
            Notification,
            r#"INSERT INTO notifications
                (user_id, notification_type, title, body, link, reference_type, reference_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, user_id, notification_type, title, body, link,
                      read, reference_type, reference_id, created_at"#,
            params.user_id,
            params.notification_type,
            params.title,
            params.body,
            params.link,
            params.reference_type,
            params.reference_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error creando notificación: {e}")))?;

        Ok(row)
    }

    /* [237A-7d] Versión transaccional: inserta dentro de un tx existente.
     * Se usa cuando el mensaje y la notificación deben persistir juntos.
     * ON CONFLICT DO NOTHING previene duplicados por constraint único.
     * Usa query_as runtime (sin macro) porque el índice parcial se crea en
     * la migración 20260723100000 y no existe en la BD local de compilación. */
    pub async fn create_tx(
        conn: &mut sqlx::PgConnection,
        params: &CreateNotification,
    ) -> Result<Option<Notification>, AppError> {
        let row = sqlx::query_as::<_, Notification>(
            "INSERT INTO notifications
                (user_id, notification_type, title, body, link, reference_type, reference_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id, notification_type, reference_type, reference_id)
                DO NOTHING
            RETURNING id, user_id, notification_type, title, body, link,
                      read, reference_type, reference_id, created_at",
        )
        .bind(params.user_id)
        .bind(&params.notification_type)
        .bind(&params.title)
        .bind(&params.body)
        .bind(&params.link)
        .bind(&params.reference_type)
        .bind(params.reference_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| AppError::Internal(format!("Error creando notificación (tx): {e}")))?;

        Ok(row)
    }

    /// Lista notificaciones de un usuario, paginadas, más recientes primero
    pub async fn list_for_user(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Notification>, AppError> {
        let rows = sqlx::query_as!(
            Notification,
            r#"SELECT id, user_id, notification_type, title, body, link,
                      read, reference_type, reference_id, created_at
            FROM notifications
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3"#,
            user_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error listando notificaciones: {e}")))?;

        Ok(rows)
    }

    /// Cuenta notificaciones no leídas de un usuario
    pub async fn count_unread(pool: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
        let row = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM notifications
            WHERE user_id = $1 AND read = false"#,
            user_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error contando no leídas: {e}")))?;

        Ok(row)
    }

    /// Marca una lista de notificaciones como leídas (solo las del usuario)
    pub async fn mark_read(pool: &PgPool, user_id: Uuid, ids: &[Uuid]) -> Result<u64, AppError> {
        let result = sqlx::query!(
            r#"UPDATE notifications SET read = true
            WHERE user_id = $1 AND id = ANY($2)"#,
            user_id,
            ids
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando leídas: {e}")))?;

        Ok(result.rows_affected())
    }

    /// Marca todas las notificaciones del usuario como leídas
    pub async fn mark_all_read(pool: &PgPool, user_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query!(
            r#"UPDATE notifications SET read = true
            WHERE user_id = $1 AND read = false"#,
            user_id
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando todas leídas: {e}")))?;

        Ok(result.rows_affected())
    }
}
