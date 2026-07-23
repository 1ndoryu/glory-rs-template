/* [237A-9] Repositorio de chat_response_cycles.
 * Gestiona los ciclos de respuesta de 10 min: cuando el staff toma la sesión
 * (human_priority) y el cliente envía un mensaje, se crea un ciclo con deadline.
 * Si nadie responde antes del deadline, el worker genera fallback IA.
 *
 * Todas las queries usan runtime (sin macros compile-time) porque la tabla
 * se crea en la migración 20260723100000. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

/// Duración del window de respuesta humana antes de fallback IA.
const RESPONSE_DEADLINE_MINS: i64 = 10;

pub struct ResponseCycleRepository;

impl ResponseCycleRepository {
    /// Crear un ciclo de respuesta cuando el cliente envía mensaje en modo human_priority.
    /// INSERT con ON CONFLICT para garantizar un solo ciclo "waiting" por sesión.
    /// Retorna Some(cycle_id) si se creó, None si ya existía uno waiting.
    pub async fn create_if_needed(
        pool: &PgPool,
        session_id: Uuid,
        opened_by_message_id: Uuid,
    ) -> Result<Option<Uuid>, AppError> {
        let row = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO chat_response_cycles \
               (session_id, opened_by_message_id, deadline_at) \
             VALUES ($1, $2, NOW() + make_interval(mins => $3)) \
             ON CONFLICT ON CONSTRAINT uq_chat_response_cycles_open DO NOTHING \
             RETURNING id",
        )
        .bind(session_id)
        .bind(opened_by_message_id)
        .bind(RESPONSE_DEADLINE_MINS as i32)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error creando response cycle: {e}")))?;

        Ok(row)
    }

    /// Reclamar ciclos expirados (deadline pasada y status='waiting').
    /// Usa FOR UPDATE SKIP LOCKED para concurrencia segura entre workers.
    /// Retorna los session_ids que necesitan fallback IA.
    pub async fn claim_expired(pool: &PgPool) -> Result<Vec<(Uuid, Uuid)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid)>(
            "UPDATE chat_response_cycles \
             SET status = 'claimed', claimed_at = NOW() \
             WHERE id IN ( \
               SELECT id FROM chat_response_cycles \
               WHERE status = 'waiting' AND deadline_at <= NOW() \
               ORDER BY deadline_at \
               FOR UPDATE SKIP LOCKED \
               LIMIT 10 \
             ) \
             RETURNING id, session_id",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error reclamando response cycles: {e}")))?;

        Ok(rows)
    }

    /// Marcar ciclo como respondido por humano (cuando staff/client envía mensaje).
    pub async fn mark_answered_human(
        pool: &PgPool,
        session_id: Uuid,
        answered_message_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE chat_response_cycles \
             SET status = 'answered_human', answered_message_id = $2 \
             WHERE session_id = $1 AND status = 'waiting'",
        )
        .bind(session_id)
        .bind(answered_message_id)
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
        sqlx::query(
            "UPDATE chat_response_cycles \
             SET status = 'answered_ai', answered_message_id = $2 \
             WHERE id = $1 AND status = 'claimed'",
        )
        .bind(cycle_id)
        .bind(ai_message_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error marcando cycle answered_ai: {e}")))?;
        Ok(())
    }

    /// Cancelar ciclos activos de una sesión (cuando se cierra o se reactiva IA).
    pub async fn cancel_for_session(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE chat_response_cycles \
             SET status = 'cancelled' \
             WHERE session_id = $1 AND status IN ('waiting', 'claimed')",
        )
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error cancelando cycles: {e}")))?;
        Ok(())
    }

    /// Verificar si la sesión tiene ai_mode='human_priority' y hay ciclo waiting.
    /// Retorna true si la IA NO debe responder automáticamente.
    pub async fn is_in_human_window(pool: &PgPool, session_id: Uuid) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS( \
               SELECT 1 FROM chat_response_cycles \
               WHERE session_id = $1 AND status = 'waiting' \
             )",
        )
        .bind(session_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error verificando human window: {e}")))?;

        Ok(exists)
    }
}
