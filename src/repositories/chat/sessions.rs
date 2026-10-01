/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat usa runtime query_as
 * para soportar campos con #[sqlx(default)] (visitor_ip, visitor_user_agent). */
//! Sesiones de chat: CRUD, staff, IA, cierre y vínculo con usuario.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::ChatRepository;
use crate::models::ChatSession;

impl ChatRepository {
    /* ============================================================
    SESIONES
    ============================================================ */

    /// Crear sesión de chat (pre-venta o vinculada a orden)
    pub async fn create_session(
        pool: &PgPool,
        visitor_id: Option<&str>,
        visitor_name: Option<&str>,
        user_id: Option<Uuid>,
        order_id: Option<Uuid>,
    ) -> Result<ChatSession, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "INSERT INTO chat_sessions (visitor_id, visitor_name, user_id, order_id) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at",
        )
        .bind(visitor_id)
        .bind(visitor_name)
        .bind(user_id)
        .bind(order_id)
        .fetch_one(pool)
        .await
    }

    pub async fn find_session_by_id(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<Option<ChatSession>, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "SELECT id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated, ai_mode, ai_generation_epoch \
             FROM chat_sessions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Sesiones con historial para un usuario autenticado.
    pub async fn list_sessions_for_user(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<ChatSession>, sqlx::Error> {
        /* [074A-30] Filtrar sesiones sin mensajes.
         * [154A-12] FIX: Agregar last_viewed_at y visitor_last_connected_at para que
         * el frontend pueda calcular correctamente qué sesiones tienen mensajes sin leer.
         * [237A-5] Las cerradas siguen visibles: `closed` es archivo, no borrado. */
        sqlx::query_as::<_, ChatSession>(
            "SELECT id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               last_viewed_at, visitor_last_connected_at \
             FROM chat_sessions \
             WHERE (user_id = $1 OR assigned_staff_id = $1) \
             AND EXISTS (SELECT 1 FROM chat_messages WHERE session_id = chat_sessions.id) \
             ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /// Sesión persistente por orden, incluso si está archivada.
    pub async fn find_session_by_order(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<ChatSession>, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "SELECT id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               last_viewed_at, visitor_last_connected_at \
             FROM chat_sessions \
             WHERE order_id = $1 \
             ORDER BY created_at ASC LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(pool)
        .await
    }

    /// Sesión activa por `visitor_id` (anónimo)
    pub async fn find_session_by_visitor(
        pool: &PgPool,
        visitor_id: &str,
    ) -> Result<Option<ChatSession>, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "SELECT id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated \
             FROM chat_sessions \
             WHERE visitor_id = $1 AND status != 'closed' \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(visitor_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_session_by_id_and_visitor(
        pool: &PgPool,
        session_id: Uuid,
        visitor_id: &str,
    ) -> Result<Option<ChatSession>, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "SELECT id, visitor_id, visitor_name, user_id, order_id, status,
               assigned_staff_id, ai_enabled, created_at, updated_at,
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at,
               visitor_country, is_escalated
             FROM chat_sessions
             WHERE id = $1 AND visitor_id = $2 AND status != 'closed'",
        )
        .bind(session_id)
        .bind(visitor_id)
        .fetch_optional(pool)
        .await
    }

    /// Sesiones activas con historial (panel staff).
    /// [277A-4] Solo sesiones no cerradas. Las cerradas se cargan bajo demanda
    /// mediante `list_all_sessions_incl_archived()` para no hidratar todo el
    /// historial en cada reconexión de admin.
    pub async fn list_sessions(pool: &PgPool) -> Result<Vec<ChatSession>, sqlx::Error> {
        /* [074A-30] Filtrar sesiones sin mensajes — no tiene sentido mostrarlas.
         * [277A-4] Solo sesiones activas/open para reducir carga en WS admin. */
        sqlx::query_as::<_, ChatSession>(
            "SELECT id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated \
             FROM chat_sessions \
             WHERE status != 'closed' \
             AND EXISTS (SELECT 1 FROM chat_messages WHERE session_id = chat_sessions.id) \
             ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await
    }

    /// Todas las sesiones incluyendo archivadas (para búsqueda/admin).
    pub async fn list_all_sessions_incl_archived(
        pool: &PgPool,
    ) -> Result<Vec<ChatSession>, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "SELECT id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated \
             FROM chat_sessions \
             WHERE EXISTS (SELECT 1 FROM chat_messages WHERE session_id = chat_sessions.id) \
             ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await
    }

    /// Staff toma una sesión (solo asigna ID, NO desactiva `ai_enabled`).
    /* [124A-CHAT1] Separar "staff ve la sesión" de "staff desactiva IA".
     * La IA se controla exclusivamente via toggle_ai; el join solo establece routing de notifs. */
    pub async fn assign_staff(
        pool: &PgPool,
        session_id: Uuid,
        staff_id: Uuid,
    ) -> Result<ChatSession, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "UPDATE chat_sessions SET assigned_staff_id = $2, \
             updated_at = NOW() WHERE id = $1 \
             RETURNING id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated",
        )
        .bind(session_id)
        .bind(staff_id)
        .fetch_one(pool)
        .await
    }

    /// Toggle IA en una sesión. También sincroniza `ai_mode`.
    /* [237A-9] Al reactivar IA → ai_mode='automatic'; al desactivar → ai_mode='manual_pause'.
     * El staff puede usar set_ai_mode('human_priority') para modo intermedio. */
    pub async fn toggle_ai(
        pool: &PgPool,
        session_id: Uuid,
        enabled: bool,
    ) -> Result<ChatSession, sqlx::Error> {
        let new_status = if enabled {
            "ai_handling"
        } else {
            "staff_handling"
        };
        let new_mode = if enabled { "automatic" } else { "manual_pause" };
        let mut tx = pool.begin().await?;
        let session = sqlx::query_as::<_, ChatSession>(
            "UPDATE chat_sessions SET ai_enabled = $2, status = $3, ai_mode = $4, \
             ai_generation_epoch = ai_generation_epoch + 1, updated_at = NOW() WHERE id = $1 \
             RETURNING id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated, ai_mode, ai_generation_epoch",
        )
        .bind(session_id)
        .bind(enabled)
        .bind(new_status)
        .bind(new_mode)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE chat_response_cycles SET status = 'cancelled' \
             WHERE session_id = $1 AND status IN ('waiting', 'claimed')",
        )
        .bind(session_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(session)
    }

    /* [237A-9] Cambiar ai_mode sin tocar ai_enabled.
     * Usado cuando staff envía mensaje (→ human_priority) o cuando el worker
     * expira el response cycle (→ automatic para fallback IA). */
    pub async fn set_ai_mode(
        pool: &PgPool,
        session_id: Uuid,
        mode: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE chat_sessions SET ai_mode = $2, \
             ai_generation_epoch = ai_generation_epoch + 1, updated_at = NOW() WHERE id = $1",
        )
        .bind(session_id)
        .bind(mode)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Cerrar sesión
    pub async fn close_session(
        pool: &PgPool,
        session_id: Uuid,
    ) -> Result<ChatSession, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "UPDATE chat_sessions SET status = 'closed', \
             updated_at = NOW() WHERE id = $1 \
             RETURNING id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated",
        )
        .bind(session_id)
        .fetch_one(pool)
        .await
    }

    /* [114A-13][237A-5] Archivar únicamente sesiones anónimas inactivas.
     * Las conversaciones de pedidos o usuarios autenticados son contractuales
     * y nunca deben cambiar de estado por un TTL general. */
    pub async fn close_inactive_sessions(
        pool: &PgPool,
        inactivity_hours: i32,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE chat_sessions SET status = 'closed', updated_at = NOW() \
              WHERE status != 'closed' \
              AND order_id IS NULL \
              AND user_id IS NULL \
              AND updated_at < NOW() - make_interval(hours => $1)",
        )
        .bind(inactivity_hours)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /* [237A-5] Una orden posee una sola conversación. El upsert depende del
     * índice único parcial creado por la migración 20260723000000 y recupera
     * explícitamente una conversación archivada sin fragmentar su historial. */
    pub async fn get_or_reopen_order_session(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
    ) -> Result<ChatSession, sqlx::Error> {
        sqlx::query_as::<_, ChatSession>(
            "INSERT INTO chat_sessions (user_id, order_id, status) \
             VALUES ($1, $2, 'active') \
             ON CONFLICT (order_id) WHERE order_id IS NOT NULL \
             DO UPDATE SET \
               status = CASE \
                 WHEN chat_sessions.status = 'closed' THEN 'active' \
                 ELSE chat_sessions.status \
               END, \
               updated_at = CASE \
                 WHEN chat_sessions.status = 'closed' THEN NOW() \
                 ELSE chat_sessions.updated_at \
               END \
             RETURNING id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated",
        )
        .bind(user_id)
        .bind(order_id)
        .fetch_one(pool)
        .await
    }

    /* [104A-39] Marcar sesión como vista por staff — actualiza last_viewed_at = NOW().
     * Permite que el badge de ChatBell solo cuente sesiones con mensajes no leídos. */
    pub async fn mark_session_viewed(pool: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE chat_sessions SET last_viewed_at = NOW() WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /* [104A-40] Actualizar timestamp de última conexión WS del visitante.
     * Llamado en ws_visitor.rs al conectar. Devuelve el timestamp actualizado
     * para poder brodcastearlo inmediatamente al canal de staff. */
    pub async fn update_visitor_last_connected(
        pool: &PgPool,
        session_id: Uuid,
    ) -> Result<DateTime<Utc>, sqlx::Error> {
        let row: (DateTime<Utc>,) = sqlx::query_as(
            "UPDATE chat_sessions SET visitor_last_connected_at = NOW() \
             WHERE id = $1 RETURNING visitor_last_connected_at",
        )
        .bind(session_id)
        .fetch_one(pool)
        .await?;
        Ok(row.0)
    }

    /* [095A-16] Vincular la sesión visitor con el usuario autenticado detectado por JWT.
     * Sin esto, recargas o reconexiones preservan el chat pero no dejan identidad persistente
     * para panel, historial ni futuras herramientas con permisos de usuario. */
    pub async fn link_session_to_user(
        pool: &PgPool,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE chat_sessions SET user_id = $2, updated_at = NOW() \
             WHERE id = $1 AND (user_id IS NULL OR user_id != $2)",
        )
        .bind(session_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Renombrar visitante de una sesión
    pub async fn update_visitor_name(
        pool: &PgPool,
        session_id: Uuid,
        name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE chat_sessions SET visitor_name = $2, updated_at = NOW() WHERE id = $1")
            .bind(session_id)
            .bind(name)
            .execute(pool)
            .await?;
        Ok(())
    }
}
