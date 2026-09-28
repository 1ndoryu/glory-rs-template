/* [237A-7j] Repositorio de tokens de continuación de chat.
 * Permite generar, validar y canjear tokens para recuperar conversaciones por email.
 * El token en claro NUNCA se persiste — solo su hash SHA-256. */

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/* Resultado de validar un token de continuación. */
pub struct ContinuationTokenInfo {
    pub session_id: Uuid,
    pub visitor_id: String,
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

/// Genera un token criptográfico aleatorio, persiste su hash y retorna el token en claro.
/// El caller debe enviar el token en claro por email — nunca loguearlo ni persistirlo.
pub async fn generate_token(
    pool: &PgPool,
    session_id: Uuid,
    visitor_id: &str,
    email: &str,
    disconnect_epoch: i64,
) -> Result<String, sqlx::Error> {
    /* Generar 32 bytes aleatorios y codificar como hex (64 caracteres) */
    let raw_bytes: [u8; 32] = rand::random();
    let token_hex = hex::encode(raw_bytes);
    let token_hash = hash_token(&token_hex);

    sqlx::query!(
        r#"INSERT INTO chat_continuation_tokens
             (session_id, visitor_id, token_hash, email, disconnect_epoch)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (session_id, disconnect_epoch) DO UPDATE SET
             visitor_id = EXCLUDED.visitor_id,
             token_hash = EXCLUDED.token_hash,
             email = EXCLUDED.email,
             expires_at = NOW() + INTERVAL '7 days',
             used_at = NULL,
             revoked_at = NULL,
             created_at = NOW()"#,
        session_id,
        visitor_id,
        &token_hash,
        email,
        disconnect_epoch
    )
    .execute(pool)
    .await?;

    Ok(token_hex)
}

/* [267A-3] Marca presencia y cancela de forma durable cualquier seguimiento
 * que todavía no haya sido entregado para esta sesión. */
pub async fn mark_connected(pool: &PgPool, session_id: Uuid) -> Result<DateTime<Utc>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    /* COALESCE en RETURNING: la columna es nulable en el esquema pero el
     * UPDATE la acaba de fijar a NOW() en la misma sentencia, así que el
     * valor es no-nulo por construcción y el macro lo verifica como tal. */
    let connected_at: DateTime<Utc> = sqlx::query_scalar!(
        r#"UPDATE chat_sessions
         SET visitor_last_connected_at = NOW(), visitor_disconnected_at = NULL
         WHERE id = $1 RETURNING COALESCE(visitor_last_connected_at, NOW()) AS "connected_at!""#,
        session_id
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!(
        r#"UPDATE chat_alert_outbox SET status = 'cancelled', locked_at = NULL,
             updated_at = NOW(), last_error = 'visitor_reconnected'
         WHERE event_type = 'chat.continuation' AND reference_id = $1
           AND status IN ('pending','processing')"#,
        session_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        r#"UPDATE chat_continuation_tokens SET revoked_at = NOW()
         WHERE session_id = $1 AND used_at IS NULL AND revoked_at IS NULL"#,
        session_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(connected_at)
}

/* [267A-3] La última desconexión crea el trabajo en la outbox en la misma TX.
 * Si falta email, consentimiento o historial, solo persiste presencia. */
pub async fn schedule_after_disconnect(pool: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    /* query_as! no acepta tuplas: query! con alias + override `!`
     * (COALESCE con NOW() nunca es nulo; epoch es BIGINT NOT NULL). */
    let row = sqlx::query!(
        r#"UPDATE chat_sessions SET visitor_disconnected_at = NOW(),
             visitor_disconnect_epoch = visitor_disconnect_epoch + 1
         WHERE id = $1 AND status <> 'closed'
         RETURNING visitor_disconnect_epoch, COALESCE(visitor_disconnected_at, NOW()) AS "connected_at!""#,
        session_id
    )
    .fetch_one(&mut *tx)
    .await?;

    /* CAST($2 AS bigint): CONCAT/jsonb_build_object dejan el parámetro como
     * unknown en DESCRIBE; el cast fija su tipo para el macro sin cambiar
     * el valor (número JSON igual que antes).
     * CAST($3 AS timestamptz): `$3 + INTERVAL` con $3 unknown resolvía $3
     * como interval; el cast restaura timestamptz+interval = timestamptz
     * (en runtime sqlx ya enviaba $3 tipado como timestamptz). */
    sqlx::query!(
        r#"INSERT INTO chat_alert_outbox
            (idempotency_key, event_type, channel, recipient, reference_type,
             reference_id, payload, available_at)
         SELECT CONCAT('chat-continuation:', s.id, ':', CAST($2 AS bigint)),
                'chat.continuation', 'email', p.email_normalized,
                'chat_session', s.id,
                jsonb_build_object(
                    'session_id', s.id,
                    'visitor_id', s.visitor_id,
                    'visitor_name', COALESCE(p.display_name, s.visitor_name, 'Visitante'),
                    'disconnect_epoch', CAST($2 AS bigint)
                ),
                CAST($3 AS timestamptz) + INTERVAL '2 minutes'
         FROM chat_sessions s
         JOIN visitor_profiles p ON p.visitor_id = s.visitor_id
         WHERE s.id = $1
           AND NULLIF(p.email_normalized, '') IS NOT NULL
           AND p.continuation_consent_at IS NOT NULL
           AND (p.continuation_declined_at IS NULL
                OR p.continuation_declined_at < p.continuation_consent_at)
           AND EXISTS (SELECT 1 FROM chat_messages m WHERE m.session_id = s.id)
         ON CONFLICT (idempotency_key) DO NOTHING"#,
        session_id,
        row.visitor_disconnect_epoch,
        row.connected_at
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

pub async fn is_disconnect_cycle_current(
    pool: &PgPool,
    session_id: Uuid,
    disconnect_epoch: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS(
            SELECT 1 FROM chat_sessions s
            JOIN visitor_profiles p ON p.visitor_id = s.visitor_id
            WHERE s.id = $1 AND s.visitor_disconnect_epoch = $2
              AND s.visitor_disconnected_at IS NOT NULL AND s.status <> 'closed'
              AND p.continuation_consent_at IS NOT NULL
              AND (p.continuation_declined_at IS NULL
                   OR p.continuation_declined_at < p.continuation_consent_at)
        ) AS "exists!""#,
        session_id,
        disconnect_epoch
    )
    .fetch_one(pool)
    .await
}

/// Valida un token: busca su hash, verifica que no esté usado/revocado/expirado.
/// Retorna la info de sesión si es válido, None si no coincide o está inválido.
pub async fn validate_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<ContinuationTokenInfo>, sqlx::Error> {
    let token_hash = hash_token(token);

    let row = sqlx::query_as!(
        TokenRow,
        r#"SELECT session_id, visitor_id, email, expires_at
         FROM chat_continuation_tokens
         WHERE token_hash = $1
           AND used_at IS NULL
           AND revoked_at IS NULL
           AND expires_at > NOW()"#,
        &token_hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ContinuationTokenInfo {
        session_id: r.session_id,
        visitor_id: r.visitor_id,
        email: r.email,
        expires_at: r.expires_at,
    }))
}

/// Canjea un token: lo marca como usado y retorna la info de sesión.
/// Retorna None si el token no es válido (ya usado, revocado, expirado o inexistente).
pub async fn redeem_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<ContinuationTokenInfo>, sqlx::Error> {
    let token_hash = hash_token(token);

    let row = sqlx::query_as!(
        TokenRow,
        r#"UPDATE chat_continuation_tokens
         SET used_at = NOW()
         WHERE token_hash = $1
           AND used_at IS NULL
           AND revoked_at IS NULL
           AND expires_at > NOW()
         RETURNING session_id, visitor_id, email, expires_at"#,
        &token_hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ContinuationTokenInfo {
        session_id: r.session_id,
        visitor_id: r.visitor_id,
        email: r.email,
        expires_at: r.expires_at,
    }))
}

/// Revoca todos los tokens activos de una sesión (al cerrar conversación, etc.).
pub async fn revoke_for_session(pool: &PgPool, session_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        r#"UPDATE chat_continuation_tokens
         SET revoked_at = NOW()
         WHERE session_id = $1
           AND used_at IS NULL
           AND revoked_at IS NULL"#,
        session_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Verifica si ya existe un token activo (no usado, no revocado, no expirado) para esta sesión.
/// Esto evita enviar múltiples emails de continuación para la misma desconexión.
pub async fn has_active_token(pool: &PgPool, session_id: Uuid) -> Result<bool, sqlx::Error> {
    let exists: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(
            SELECT 1 FROM chat_continuation_tokens
            WHERE session_id = $1
              AND used_at IS NULL
              AND revoked_at IS NULL
              AND expires_at > NOW()
        ) AS "exists!""#,
        session_id
    )
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

/* ===== Internos ===== */

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    session_id: Uuid,
    visitor_id: String,
    email: String,
    expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_is_deterministic() {
        let token = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let h1 = hash_token(token);
        let h2 = hash_token(token);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); /* SHA-256 hex = 64 chars */
    }

    #[test]
    fn different_tokens_produce_different_hashes() {
        let h1 = hash_token("aaaa");
        let h2 = hash_token("bbbb");
        assert_ne!(h1, h2);
    }
}
