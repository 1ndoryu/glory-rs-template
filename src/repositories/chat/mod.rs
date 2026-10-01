/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat usa runtime query_as
 * para soportar campos con #[sqlx(default)] (visitor_ip, visitor_user_agent). */
/* [044A-38 Fase 5] Repositorio de chat: CRUD sesiones y mensajes.
 * [064A-72] ChatSession queries migradas a runtime query_as para soportar
 * campos con #[sqlx(default)] (visitor_ip, visitor_user_agent).
 * Queries con prepared statements. Soporta sesiones anónimas y autenticadas. */

mod attachments;
mod messages;
mod notes;
mod sessions;
mod visitor;

use sqlx::PgPool;
use uuid::Uuid;

pub struct ChatRepository;

impl ChatRepository {
    /* [20CA-8] Buscar sesiones con mensajes sin responder >threshold minutos.
     * Un mensaje "sin responder" es el último de la sesión y fue enviado por
     * visitor/client (no staff/system). Retorna session_ids.
     * Usa sqlx::query_scalar() runtime (sin macro) para no requerir .sqlx/ cache entry. */
    pub async fn find_unanswered_sessions(
        pool: &PgPool,
        threshold_minutes: i64,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"SELECT cs.id
            FROM chat_sessions cs
            JOIN LATERAL (
                SELECT sender_type, content, created_at
                FROM chat_messages
                WHERE session_id = cs.id
                ORDER BY created_at DESC
                LIMIT 1
            ) lm ON TRUE
            WHERE cs.status = 'open'
              AND lm.sender_type IN ('visitor', 'user')
              AND lm.created_at < NOW() - make_interval(mins => $1)
              AND (cs.last_viewed_at IS NULL OR cs.last_viewed_at < lm.created_at)"#,
        )
        .bind(threshold_minutes as i32)
        .fetch_all(pool)
        .await
    }
}
