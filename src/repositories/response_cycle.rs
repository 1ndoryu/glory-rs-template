/* [237A-9] Repositorio de chat_response_cycles.
 * Gestiona los ciclos de respuesta de 10 min: cuando el staff toma la sesión
 * (human_priority) y el cliente envía un mensaje, se crea un ciclo con deadline.
 * Si nadie responde antes del deadline, el worker genera fallback IA.
 *
 * [279A-4] Migrado a macros compile-time: nakomi_dev tiene aplicadas todas
 * las migraciones (incluida la tabla 20260723100000). */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

/// Duración del window de respuesta humana antes de fallback IA.
const RESPONSE_DEADLINE_MINS: i64 = 10;

pub struct ResponseCycleRepository;

impl ResponseCycleRepository {
    /* [257A-9] Persiste el fallback y cierra el ciclo bajo el mismo lock. Si ya
     * existe una respuesta humana posterior al mensaje que abrió el ciclo, no
     * inserta nada. Así el worker no puede contestar después del humano durante
     * la pequeña ventana entre INSERT del mensaje y actualización del ciclo. */
    pub async fn save_fallback_if_still_claimed(
        pool: &PgPool,
        cycle_id: Uuid,
        session_id: Uuid,
        content: &str,
    ) -> Result<Option<crate::models::ChatMessage>, AppError> {
        let mut tx = pool.begin().await.map_err(|error| {
            AppError::Internal(format!("Error iniciando fallback transaccional: {error}"))
        })?;

        let status: Option<String> = sqlx::query_scalar!(
            r#"SELECT status FROM chat_response_cycles
             WHERE id = $1 AND session_id = $2 FOR UPDATE"#,
            cycle_id,
            session_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| AppError::Internal(format!("Error validando fallback: {error}")))?;

        if status.as_deref() != Some("claimed") {
            tx.rollback().await.ok();
            return Ok(None);
        }

        let human_answered: bool = sqlx::query_scalar!(
            r#"SELECT EXISTS(
               SELECT 1 FROM chat_response_cycles cycle
               JOIN chat_messages opening_message ON opening_message.id = cycle.opened_by_message_id
               JOIN chat_messages human_message ON human_message.session_id = cycle.session_id
               WHERE cycle.id = $1
                 AND human_message.created_at >= opening_message.created_at
                 AND human_message.sender_type IN ('admin', 'employee', 'staff')
             ) AS "human_answered!""#,
            cycle_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| AppError::Internal(format!("Error buscando respuesta humana: {error}")))?;

        if human_answered {
            sqlx::query!(
                r#"UPDATE chat_response_cycles SET status = 'answered_human' WHERE id = $1"#,
                cycle_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|error| AppError::Internal(format!("Error cerrando ciclo humano: {error}")))?;
            tx.commit().await.map_err(|error| {
                AppError::Internal(format!("Error confirmando ciclo humano: {error}"))
            })?;
            return Ok(None);
        }

        let message = sqlx::query_as!(
            crate::models::ChatMessage,
            r#"INSERT INTO chat_messages (session_id, sender_type, sender_id, content)
             VALUES ($1, 'ai', 'ai', $2)
             RETURNING id, session_id, sender_type, sender_id, content, created_at,
                       message_type, metadata, sequence_num"#,
            session_id,
            content
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| AppError::Internal(format!("Error guardando fallback: {error}")))?;

        sqlx::query!(
            r#"UPDATE chat_response_cycles SET status = 'answered_ai', answered_message_id = $2
             WHERE id = $1 AND status = 'claimed'"#,
            cycle_id,
            message.id
        )
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::Internal(format!("Error cerrando fallback: {error}")))?;

        tx.commit()
            .await
            .map_err(|error| AppError::Internal(format!("Error confirmando fallback: {error}")))?;
        Ok(Some(message))
    }

    /// Crear un ciclo de respuesta cuando el cliente envía mensaje en modo `human_priority`.
    /// INSERT con ON CONFLICT para garantizar un solo ciclo "waiting" por sesión.
    /// Retorna `Some(cycle_id)` si se creó, None si ya existía uno waiting.
    pub async fn create_if_needed(
        pool: &PgPool,
        session_id: Uuid,
        opened_by_message_id: Uuid,
    ) -> Result<Option<Uuid>, AppError> {
        let row = sqlx::query_scalar!(
            r#"INSERT INTO chat_response_cycles
               (session_id, opened_by_message_id, deadline_at)
             VALUES ($1, $2, NOW() + make_interval(mins => $3))
             ON CONFLICT (session_id) WHERE status = 'waiting' DO NOTHING
             RETURNING id"#,
            session_id,
            opened_by_message_id,
            RESPONSE_DEADLINE_MINS as i32
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error creando response cycle: {e}")))?;

        Ok(row)
    }

    /// Reclamar ciclos expirados (deadline pasada y status='waiting').
    /// Usa FOR UPDATE SKIP LOCKED para concurrencia segura entre workers.
    /// Retorna los `session_ids` que necesitan fallback IA.
    pub async fn claim_expired(pool: &PgPool) -> Result<Vec<(Uuid, Uuid)>, AppError> {
        /* query_as! no acepta tuplas: query! + map manual
         * (id/session_id son UUID NOT NULL). */
        let rows = sqlx::query!(
            r#"UPDATE chat_response_cycles
             SET status = 'claimed', claimed_at = NOW()
             WHERE id IN (
               SELECT id FROM chat_response_cycles
               WHERE status = 'waiting' AND deadline_at <= NOW()
               ORDER BY deadline_at
               FOR UPDATE SKIP LOCKED
               LIMIT 10
             )
             RETURNING id, session_id"#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error reclamando response cycles: {e}")))?;

        Ok(rows.into_iter().map(|r| (r.id, r.session_id)).collect())
    }

    /// Marcar ciclo como respondido por humano (cuando staff/client envía mensaje).
    pub async fn mark_answered_human(
        pool: &PgPool,
        session_id: Uuid,
        answered_message_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"UPDATE chat_response_cycles
             SET status = 'answered_human', answered_message_id = $2
             WHERE session_id = $1 AND status IN ('waiting', 'claimed')"#,
            session_id,
            answered_message_id
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando cycle answered: {e}")))?;
        Ok(())
    }

    /// Marcar ciclo como respondido por IA (fallback exitoso).
    pub async fn mark_answered_ai(
        pool: &PgPool,
        cycle_id: Uuid,
        ai_message_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"UPDATE chat_response_cycles
             SET status = 'answered_ai', answered_message_id = $2
             WHERE id = $1 AND status = 'claimed'"#,
            cycle_id,
            ai_message_id
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando cycle answered_ai: {e}")))?;
        Ok(())
    }

    /// Cancelar ciclos activos de una sesión (cuando se cierra o se reactiva IA).
    pub async fn cancel_for_session(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"UPDATE chat_response_cycles
             SET status = 'cancelled'
             WHERE session_id = $1 AND status IN ('waiting', 'claimed')"#,
            session_id
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error cancelando cycles: {e}")))?;
        Ok(())
    }

    /// Verificar si la sesión tiene `ai_mode`='`human_priority`' y hay ciclo waiting.
    /// Retorna true si la IA NO debe responder automáticamente.
    pub async fn is_in_human_window(pool: &PgPool, session_id: Uuid) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS(
               SELECT 1 FROM chat_response_cycles
               WHERE session_id = $1 AND status = 'waiting'
             ) AS "exists!""#,
            session_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error verificando human window: {e}")))?;

        Ok(exists)
    }
}
