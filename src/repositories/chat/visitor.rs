/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat usa runtime query_as
 * para soportar campos con #[sqlx(default)] (visitor_ip, visitor_user_agent). */
//! Perfiles de visitante anónimo: memoria, contexto IA y vínculo con usuario (T-3).

use sqlx::PgPool;
use uuid::Uuid;

use super::ChatRepository;
use crate::models::VisitorProfile;

impl ChatRepository {
    /* ============================================================
    VISITOR PROFILES (T-3 — Memoria usuario + contexto)
    ============================================================ */

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

    /* [267A-2] Captura conversacional atómica: email, normalización y consentimiento
     * deben persistirse juntos. Separarlos permitía guardar el email pero perder la
     * autorización necesaria para enviar el enlace de continuación. */
    pub async fn capture_visitor_email(
        pool: &PgPool,
        visitor_id: &str,
        email: &str,
        display_name: Option<&str>,
    ) -> Result<VisitorProfile, sqlx::Error> {
        sqlx::query_as::<_, VisitorProfile>(
            "UPDATE visitor_profiles SET \
               email = $2, \
               email_normalized = $2, \
               email_captured_at = NOW(), \
               continuation_consent_at = NOW(), \
               continuation_declined_at = NULL, \
               email_source = 'chatbot', \
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

    /* [267A-2] El nombre tiene su propia operación para que nunca sobrescriba el
     * email capturado. Este contrato evita depender de sentinelas como cadena vacía. */
    pub async fn update_visitor_display_name(
        pool: &PgPool,
        visitor_id: &str,
        display_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE visitor_profiles SET display_name = $2, last_seen_at = NOW() \
             WHERE visitor_id = $1",
        )
        .bind(visitor_id)
        .bind(display_name)
        .execute(pool)
        .await?;
        Ok(())
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
}
