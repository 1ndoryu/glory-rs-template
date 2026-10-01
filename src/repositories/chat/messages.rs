/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat usa runtime query_as
 * para soportar campos con #[sqlx(default)] (visitor_ip, visitor_user_agent). */
//! Mensajes de chat: guardado con secuencia atómica, listado y typing preview.

use sqlx::PgPool;
use uuid::Uuid;

use super::ChatRepository;
use crate::models::ChatMessage;

impl ChatRepository {
    /* ============================================================
    MENSAJES
    ============================================================ */

    /// Guardar mensaje en BD con secuencia monotónica atómica.
    /* [237A-8] CTE incrementa next_message_sequence y lo asigna como sequence_num
     * en la misma transacción. Garantiza monotonicidad sin locks explícitos. */
    pub async fn save_message(
        pool: &PgPool,
        session_id: Uuid,
        sender_type: &str,
        sender_id: Option<&str>,
        content: &str,
    ) -> Result<ChatMessage, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            "WITH seq AS ( \
               UPDATE chat_sessions SET next_message_sequence = next_message_sequence + 1, \
                 updated_at = NOW() \
               WHERE id = $1 \
               RETURNING next_message_sequence \
             ) \
             INSERT INTO chat_messages (session_id, sender_type, sender_id, content, sequence_num) \
             VALUES ($1, $2, $3, $4, (SELECT next_message_sequence FROM seq)) \
             RETURNING id, session_id, sender_type, sender_id, content, created_at, \
                       message_type, metadata, sequence_num",
        )
        .bind(session_id)
        .bind(sender_type)
        .bind(sender_id)
        .bind(content)
        .fetch_one(pool)
        .await
    }

    /* [T-2][237A-8] Guardar mensaje rico con tipo, metadatos y secuencia atómica. */
    pub async fn save_rich_message(
        pool: &PgPool,
        session_id: Uuid,
        sender_type: &str,
        sender_id: Option<&str>,
        content: &str,
        message_type: &str,
        metadata: &serde_json::Value,
    ) -> Result<ChatMessage, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            "WITH seq AS ( \
               UPDATE chat_sessions SET next_message_sequence = next_message_sequence + 1, \
                 updated_at = NOW() \
               WHERE id = $1 \
               RETURNING next_message_sequence \
             ) \
             INSERT INTO chat_messages (session_id, sender_type, sender_id, content, message_type, metadata, sequence_num) \
             VALUES ($1, $2, $3, $4, $5, $6, (SELECT next_message_sequence FROM seq)) \
             RETURNING id, session_id, sender_type, sender_id, content, created_at, \
                       message_type, metadata, sequence_num",
        )
        .bind(session_id)
        .bind(sender_type)
        .bind(sender_id)
        .bind(content)
        .bind(message_type)
        .bind(metadata)
        .fetch_one(pool)
        .await
    }

    /// Últimos mensajes de una sesión, reordenados cronológicamente para render.
    pub async fn list_messages(
        pool: &PgPool,
        session_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        /* [237A-5] El LIMIT se aplica en orden descendente para no ocultar los
         * mensajes nuevos al superar el tamaño de página; la consulta exterior
         * restaura el orden ascendente esperado por la UI. `id` desempata fechas. */
        sqlx::query_as::<_, ChatMessage>(
            "SELECT id, session_id, sender_type, sender_id, content, created_at, \
                    message_type, metadata, sequence_num \
             FROM ( \
               SELECT id, session_id, sender_type, sender_id, content, created_at, \
                      message_type, metadata, sequence_num \
               FROM chat_messages \
               WHERE session_id = $1 \
               ORDER BY created_at DESC, id DESC \
               LIMIT $2 OFFSET $3 \
             ) AS recent_messages \
             ORDER BY created_at ASC, id ASC",
        )
        .bind(session_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// Último mensaje de múltiples sesiones (para preview en lista)
    pub async fn last_messages_for_sessions(
        pool: &PgPool,
        session_ids: &[Uuid],
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT DISTINCT ON (session_id) \
               id, session_id, sender_type, sender_id, content, created_at, \
               message_type, metadata, sequence_num \
             FROM chat_messages \
             WHERE session_id = ANY($1) \
             ORDER BY session_id, created_at DESC",
        )
        .bind(session_ids)
        .fetch_all(pool)
        .await
    }

    /* ============================================================
    TYPING
    ============================================================ */

    /// Actualizar typing preview
    pub async fn update_typing(
        pool: &PgPool,
        session_id: Uuid,
        content: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"INSERT INTO chat_typing (session_id, content, updated_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (session_id) DO UPDATE SET content = $2, updated_at = NOW()"#,
            session_id,
            content,
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
