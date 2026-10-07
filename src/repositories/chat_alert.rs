/* [237A-7d] Repositorio de chat_alert_outbox.
 * [279A-4] Migrado a macros compile-time (query!/query_as!/query_scalar!):
 * nakomi_dev tiene aplicadas todas las migraciones (incluida la tabla
 * 20260723100000), así que la verificación en compilación ya resuelve.
 * insert_tx() se llama dentro de la transacción del mensaje.
 * claim() usa FOR UPDATE SKIP LOCKED para concurrencia segura. */

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ChatAlertOutbox;

pub struct ChatAlertRepository;

/* [07AA-14] Entrada agrupada para insert_tx (parametros-excesivos-rs 9→2):
 * prestada (`&`) para no cambiar ni una asignación en los llamadores. */
pub struct EntradaAlertaChat<'a> {
    pub idempotency_key: &'a str,
    pub event_type: &'a str,
    pub channel: &'a str,
    pub recipient: &'a str,
    pub reference_type: Option<&'a str>,
    pub reference_id: Option<Uuid>,
    pub payload: &'a serde_json::Value,
}

/* Backoff en segundos: 5s, 30s, 2m, 10m, 30m → dead */
const BACKOFF_SEQUENCE: [i64; 5] = [5, 30, 120, 600, 1800];
const MAX_ATTEMPTS: i32 = 5;
const STALE_PROCESSING_SECS: i64 = 300;

impl ChatAlertRepository {
    /// Inserta una entrada en la outbox DENTRO de una transacción existente.
    /// INSERT ... ON CONFLICT DO NOTHING para idempotencia.
    /// Retorna true si se insertó (nuevo), false si ya existía (duplicado).
    pub async fn insert_tx(
        tx: &mut sqlx::PgConnection,
        entrada: EntradaAlertaChat<'_>,
    ) -> Result<bool, AppError> {
        let row = sqlx::query_scalar!(
            r#"INSERT INTO chat_alert_outbox
                (idempotency_key, event_type, channel, recipient,
                 reference_type, reference_id, payload)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING id"#,
            entrada.idempotency_key,
            entrada.event_type,
            entrada.channel,
            entrada.recipient,
            entrada.reference_type,
            entrada.reference_id,
            entrada.payload
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(format!("Error insertando outbox alert: {e}")))?;

        Ok(row.is_some())
    }

    /// Reclama hasta `limit` filas pendientes usando FOR UPDATE SKIP LOCKED.
    pub async fn claim(pool: &PgPool, limit: i64) -> Result<Vec<ChatAlertOutbox>, AppError> {
        let now = Utc::now();
        let stale_cutoff = now - Duration::seconds(STALE_PROCESSING_SECS);

        let rows = sqlx::query_as!(
            ChatAlertOutbox,
            r#"UPDATE chat_alert_outbox
            SET status = 'processing',
                attempts = attempts + 1,
                locked_at = $1,
                updated_at = $1
            WHERE id IN (
                SELECT id FROM chat_alert_outbox
                WHERE (status = 'pending' AND available_at <= $1)
                   OR (status = 'processing' AND locked_at < $2)
                ORDER BY available_at
                FOR UPDATE SKIP LOCKED
                LIMIT $3
            )
            RETURNING id, idempotency_key, event_type, channel, recipient,
                      reference_type, reference_id, payload,
                      status, attempts, available_at, locked_at,
                      last_error, created_at, updated_at, sent_at"#,
            now,
            stale_cutoff,
            limit
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error reclamando outbox: {e}")))?;

        Ok(rows)
    }

    /// Marca una entrada como 'sent' después de confirmación exitosa.
    pub async fn mark_sent(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        let now = Utc::now();
        sqlx::query!(
            r#"UPDATE chat_alert_outbox SET status = 'sent', sent_at = $2, updated_at = $2, locked_at = NULL WHERE id = $1"#,
            id,
            now
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando outbox sent: {e}")))?;
        Ok(())
    }

    /// Marca como '`accepted_by_gateway`' (encolada en el gateway remoto).
    pub async fn mark_accepted_by_gateway(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        let now = Utc::now();
        sqlx::query!(
            r#"UPDATE chat_alert_outbox SET status = 'accepted_by_gateway', updated_at = $2, locked_at = NULL WHERE id = $1"#,
            id,
            now
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando outbox accepted: {e}")))?;
        Ok(())
    }

    /// Reintenta con backoff. Si se exceden `MAX_ATTEMPTS` → dead.
    pub async fn mark_retry(
        pool: &PgPool,
        id: Uuid,
        error: &str,
        attempt: i32,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let truncated_error: String = error.chars().take(500).collect();

        if attempt >= MAX_ATTEMPTS {
            sqlx::query!(
                r#"UPDATE chat_alert_outbox SET status = 'dead', last_error = $2, updated_at = $3, locked_at = NULL WHERE id = $1"#,
                id,
                &truncated_error,
                now
            )
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Error marcando outbox dead: {e}")))?;
        } else {
            /* [259A-1] Sin `as usize` (clippy cast_sign_loss): attempt viene de
             * BD (i32, podria ser <= 0 en filas anómalas). Clamp a >= 0. */
            let idx = usize::try_from(attempt.saturating_sub(1).max(0)).unwrap_or(0);
            let backoff_secs = BACKOFF_SEQUENCE.get(idx).copied().unwrap_or(1800);
            let available_at = now + Duration::seconds(backoff_secs);
            sqlx::query!(
                r#"UPDATE chat_alert_outbox SET status = 'pending', last_error = $2, available_at = $3, updated_at = $3, locked_at = NULL WHERE id = $1"#,
                id,
                &truncated_error,
                available_at
            )
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Error marcando outbox retry: {e}")))?;
        }
        Ok(())
    }

    /// Marca como dead (error de contrato, configuración ausente, etc.).
    pub async fn mark_dead(pool: &PgPool, id: Uuid, error: &str) -> Result<(), AppError> {
        let now = Utc::now();
        let truncated_error: String = error.chars().take(500).collect();
        sqlx::query!(
            r#"UPDATE chat_alert_outbox SET status = 'dead', last_error = $2, updated_at = $3, locked_at = NULL WHERE id = $1"#,
            id,
            &truncated_error,
            now
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando outbox dead: {e}")))?;
        Ok(())
    }

    pub async fn mark_cancelled(pool: &PgPool, id: Uuid, reason: &str) -> Result<(), AppError> {
        let truncated_reason: String = reason.chars().take(500).collect();
        sqlx::query!(
            r#"UPDATE chat_alert_outbox SET status = 'cancelled', last_error = $2,
             updated_at = NOW(), locked_at = NULL WHERE id = $1"#,
            id,
            truncated_reason
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error cancelando outbox: {e}")))?;
        Ok(())
    }

    /// Métricas: conteo por estado.
    pub async fn counts_by_status(pool: &PgPool) -> Result<Vec<(String, i64)>, AppError> {
        /* query_as! no acepta tuplas como tipo de salida: query! + map
         * manual. COUNT(*) nunca es nulo (i64); status es NOT NULL. */
        let rows = sqlx::query!(
            r#"SELECT status, COUNT(*) AS "count!" FROM chat_alert_outbox GROUP BY status"#
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error contando outbox: {e}")))?;
        Ok(rows.into_iter().map(|r| (r.status, r.count)).collect())
    }

    /// Edad del evento más antiguo pendiente (en segundos).
    pub async fn oldest_pending_age_secs(pool: &PgPool) -> Result<Option<i64>, AppError> {
        let row: Option<i64> = sqlx::query_scalar!(
            r#"SELECT EXTRACT(EPOCH FROM (now() - MIN(created_at)))::bigint
            FROM chat_alert_outbox WHERE status IN ('pending', 'processing')"#,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error consultando edad outbox: {e}")))?;
        Ok(row)
    }
}
