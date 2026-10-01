/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat usa runtime query_as
 * para soportar campos con #[sqlx(default)] (visitor_ip, visitor_user_agent). */
//! Notas internas de staff sobre sesiones de chat (064A-72).

use sqlx::PgPool;
use uuid::Uuid;

use super::ChatRepository;
use crate::models::ChatSessionNote;

impl ChatRepository {
    /* ============================================================
    NOTAS DE SESIÓN (064A-72)
    ============================================================ */

    /// Listar notas de una sesión
    pub async fn list_session_notes(
        pool: &PgPool,
        session_id: Uuid,
    ) -> Result<Vec<ChatSessionNote>, sqlx::Error> {
        sqlx::query_as::<_, ChatSessionNote>(
            "SELECT id, session_id, author_id, content, created_at \
             FROM chat_session_notes WHERE session_id = $1 \
             ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(pool)
        .await
    }

    /// Crear nota en una sesión
    pub async fn create_session_note(
        pool: &PgPool,
        session_id: Uuid,
        author_id: Uuid,
        content: &str,
    ) -> Result<ChatSessionNote, sqlx::Error> {
        sqlx::query_as::<_, ChatSessionNote>(
            "INSERT INTO chat_session_notes (session_id, author_id, content) \
             VALUES ($1, $2, $3) \
             RETURNING id, session_id, author_id, content, created_at",
        )
        .bind(session_id)
        .bind(author_id)
        .bind(content)
        .fetch_one(pool)
        .await
    }
}
