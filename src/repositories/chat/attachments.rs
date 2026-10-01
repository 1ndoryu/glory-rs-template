/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat usa runtime query_as
 * para soportar campos con #[sqlx(default)] (visitor_ip, visitor_user_agent). */
//! Adjuntos de chat: archivos vinculados a mensajes (T-5).

use sqlx::PgPool;
use uuid::Uuid;

use super::ChatRepository;
use crate::models::ChatAttachment;

impl ChatRepository {
    /* ============================================================
    ATTACHMENTS (T-5 — Archivos en chat)
    ============================================================ */

    /* [T-5] Guardar adjunto vinculado a un mensaje de chat.
     * ai_description se actualiza después con el resultado de Vision/Whisper/PDF. */
    pub async fn save_attachment(
        pool: &PgPool,
        message_id: Uuid,
        file_name: &str,
        file_path: &str,
        mime_type: &str,
        file_size_bytes: i64,
    ) -> Result<ChatAttachment, sqlx::Error> {
        sqlx::query_as::<_, ChatAttachment>(
            "INSERT INTO chat_attachments (message_id, file_name, file_path, mime_type, file_size_bytes) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, message_id, file_name, file_path, mime_type, file_size_bytes, ai_description, created_at",
        )
        .bind(message_id)
        .bind(file_name)
        .bind(file_path)
        .bind(mime_type)
        .bind(file_size_bytes)
        .fetch_one(pool)
        .await
    }

    /* [T-5] Actualizar descripción generada por IA (Vision, Whisper, PDF extract) */
    pub async fn update_attachment_description(
        pool: &PgPool,
        attachment_id: Uuid,
        description: &str,
    ) -> Result<(), sqlx::Error> {
        /* [105A-4] La IA lee chat_messages; copiamos la descripción a metadata en el mismo roundtrip. */
        sqlx::query(
                        "WITH updated_attachment AS (UPDATE chat_attachments SET ai_description = $2 WHERE id = $1 RETURNING message_id) UPDATE chat_messages SET metadata = COALESCE(metadata, '{}'::jsonb) || jsonb_build_object('ai_description', $2::text) WHERE id IN (SELECT message_id FROM updated_attachment)",
        )
        .bind(attachment_id)
        .bind(description)
        .execute(pool)
        .await
        .map(drop)
    }

    /* [T-5] Listar adjuntos de un mensaje */
    pub async fn list_attachments_for_message(
        pool: &PgPool,
        message_id: Uuid,
    ) -> Result<Vec<ChatAttachment>, sqlx::Error> {
        sqlx::query_as::<_, ChatAttachment>(
            "SELECT id, message_id, file_name, file_path, mime_type, file_size_bytes, ai_description, created_at \
             FROM chat_attachments WHERE message_id = $1 ORDER BY created_at ASC",
        )
        .bind(message_id)
        .fetch_all(pool)
        .await
    }
}
