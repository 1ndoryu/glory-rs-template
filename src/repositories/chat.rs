/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat usa runtime query_as
 * para soportar campos con #[sqlx(default)] (visitor_ip, visitor_user_agent). */
/* [044A-38 Fase 5] Repositorio de chat: CRUD sesiones y mensajes.
 * [064A-72] ChatSession queries migradas a runtime query_as para soportar
 * campos con #[sqlx(default)] (visitor_ip, visitor_user_agent).
 * Queries con prepared statements. Soporta sesiones anónimas y autenticadas. */

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{ChatAttachment, ChatMessage, ChatSession, ChatSessionNote, VisitorProfile};

pub struct ChatRepository;

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
               visitor_country, is_escalated \
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

    /// Todas las sesiones con historial (panel staff).
    pub async fn list_sessions(pool: &PgPool) -> Result<Vec<ChatSession>, sqlx::Error> {
        /* [074A-30] Filtrar sesiones sin mensajes — no tiene sentido mostrarlas.
         * [237A-5] Las cerradas forman el archivo auditable y no se ocultan. */
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

    /// Toggle IA en una sesión. También sincroniza ai_mode.
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
        sqlx::query_as::<_, ChatSession>(
            "UPDATE chat_sessions SET ai_enabled = $2, status = $3, ai_mode = $4, \
             updated_at = NOW() WHERE id = $1 \
             RETURNING id, visitor_id, visitor_name, user_id, order_id, status, \
               assigned_staff_id, ai_enabled, created_at, updated_at, \
               visitor_ip, visitor_user_agent, last_viewed_at, visitor_last_connected_at, \
               visitor_country, is_escalated",
        )
        .bind(session_id)
        .bind(enabled)
        .bind(new_status)
        .bind(new_mode)
        .fetch_one(pool)
        .await
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
            "UPDATE chat_sessions SET ai_mode = $2, updated_at = NOW() WHERE id = $1",
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

    /* [084A-40][237A-5] Borrar perfil del visitante anónimo al resetear para limpiar
     * context_summary, preferences, sesiones acumuladas, etc.) */
    pub async fn delete_visitor_profile(
        pool: &PgPool,
        visitor_id: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM visitor_profiles WHERE visitor_id = $1")
            .bind(visitor_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

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

    /* ============================================================
    VISITOR PROFILES (T-3 — Memoria usuario + contexto)
    ============================================================ */

    /* [T-3] Buscar perfil por visitor_id (localStorage UUID del visitante) */
    pub async fn find_visitor_profile(
        pool: &PgPool,
        visitor_id: &str,
    ) -> Result<Option<VisitorProfile>, sqlx::Error> {
        sqlx::query_as::<_, VisitorProfile>(
            "SELECT id, visitor_id, email, user_id, display_name, context_summary, \
               preferences, first_seen_at, last_seen_at, total_sessions, \
               ip_addresses, device_fingerprints \
             FROM visitor_profiles WHERE visitor_id = $1",
        )
        .bind(visitor_id)
        .fetch_optional(pool)
        .await
    }

    /* [T-3] Upsert: crea o actualiza perfil al conectar WS.
     * Usa ON CONFLICT para atomicidad (evita race conditions).
     * Agrega IP y fingerprint solo si no existen ya en el array. */
    pub async fn upsert_visitor_profile(
        pool: &PgPool,
        visitor_id: &str,
        ip: Option<&str>,
        device_fingerprint: Option<&str>,
    ) -> Result<VisitorProfile, sqlx::Error> {
        sqlx::query_as::<_, VisitorProfile>(
            "INSERT INTO visitor_profiles (visitor_id, ip_addresses, device_fingerprints) \
             VALUES ($1, \
               CASE WHEN $2::text IS NOT NULL THEN ARRAY[$2::text] ELSE '{}'::text[] END, \
               CASE WHEN $3::text IS NOT NULL THEN ARRAY[$3::text] ELSE '{}'::text[] END) \
             ON CONFLICT (visitor_id) DO UPDATE SET \
               last_seen_at = NOW(), \
               total_sessions = visitor_profiles.total_sessions + 1, \
               ip_addresses = CASE WHEN $2::text IS NOT NULL AND NOT ($2::text = ANY(visitor_profiles.ip_addresses)) \
                 THEN array_append(visitor_profiles.ip_addresses, $2::text) \
                 ELSE visitor_profiles.ip_addresses END, \
               device_fingerprints = CASE WHEN $3::text IS NOT NULL AND NOT ($3::text = ANY(visitor_profiles.device_fingerprints)) \
                 THEN array_append(visitor_profiles.device_fingerprints, $3::text) \
                 ELSE visitor_profiles.device_fingerprints END \
             RETURNING id, visitor_id, email, user_id, display_name, context_summary, \
               preferences, first_seen_at, last_seen_at, total_sessions, \
               ip_addresses, device_fingerprints",
        )
        .bind(visitor_id)
        .bind(ip)
        .bind(device_fingerprint)
        .fetch_one(pool)
        .await
    }

    /* [T-3] Capturar email del visitante (tool call capture_email).
     * También actualiza display_name si se proporciona. */
    pub async fn update_visitor_email(
        pool: &PgPool,
        visitor_id: &str,
        email: &str,
        display_name: Option<&str>,
    ) -> Result<VisitorProfile, sqlx::Error> {
        sqlx::query_as::<_, VisitorProfile>(
            "UPDATE visitor_profiles SET \
               email = $2, \
               display_name = COALESCE($3, display_name), \
               last_seen_at = NOW() \
             WHERE visitor_id = $1 \
             RETURNING id, visitor_id, email, user_id, display_name, context_summary, \
               preferences, first_seen_at, last_seen_at, total_sessions, \
               ip_addresses, device_fingerprints",
        )
        .bind(visitor_id)
        .bind(email)
        .bind(display_name)
        .fetch_one(pool)
        .await
    }

    /* [T-3] Actualizar resumen de contexto (generado por IA al cerrar sesión) */
    pub async fn update_context_summary(
        pool: &PgPool,
        visitor_id: &str,
        summary: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE visitor_profiles SET context_summary = $2, last_seen_at = NOW() \
             WHERE visitor_id = $1",
        )
        .bind(visitor_id)
        .bind(summary)
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [T-3] Actualizar preferencias extraídas de la conversación (JSON merge) */
    pub async fn update_visitor_preferences(
        pool: &PgPool,
        visitor_id: &str,
        preferences: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE visitor_profiles SET \
               preferences = COALESCE(preferences, '{}'::jsonb) || $2::jsonb, \
               last_seen_at = NOW() \
             WHERE visitor_id = $1",
        )
        .bind(visitor_id)
        .bind(preferences)
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [T-3] Vincular visitante con usuario registrado */
    pub async fn link_visitor_to_user(
        pool: &PgPool,
        visitor_id: &str,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE visitor_profiles SET user_id = $2, last_seen_at = NOW() \
             WHERE visitor_id = $1",
        )
        .bind(visitor_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

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
