//! GAME-01 — Tickets de identidad para transporte realtime.
//!
//! Este módulo no abre WebSocket ni conoce Axum. Emite y consume tickets de
//! propósito único para que el futuro upgrade pueda resolver la identidad desde
//! un UUID server-side, sin aceptar `user_id` enviado por el navegador.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::errors::AppError;

type HmacSha256 = Hmac<Sha256>;

const PROTOCOL_VERSION: &str = "g1";
const PURPOSE: &str = "game";
pub const GAME_TICKET_DEFAULT_TTL_SECS: i64 = 30;
pub const GAME_TICKET_MAX_TTL_SECS: i64 = 60;
pub const GAME_TICKET_MAX_TOKEN_BYTES: usize = 512;
pub const GAME_TICKET_MAX_REPLAY_ENTRIES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTicketClaims {
    /// UUID resuelto por el servidor; nunca procede del payload del cliente.
    pub subject: Uuid,
    pub expires_at: i64,
    pub nonce: Uuid,
}

/// Almacén single-use local para el primer despliegue single-instance.
///
/// Una futura topología multi-réplica deberá sustituir este adaptador por un
/// store compartido antes de habilitar tickets entre instancias.
#[derive(Clone, Default)]
pub struct GameTicketReplayStore {
    consumed: Arc<Mutex<HashMap<Uuid, i64>>>,
}

impl GameTicketReplayStore {
    /// Consume un ticket válido usando el reloj del sistema.
    pub fn consume(&self, token: &str, secret: &str) -> Result<GameTicketClaims, AppError> {
        consume_at(&self.consumed, token, secret, now_unix())
    }

    #[cfg(test)]
    fn consume_at_for_test(
        &self,
        token: &str,
        secret: &str,
        now: i64,
    ) -> Result<GameTicketClaims, AppError> {
        consume_at(&self.consumed, token, secret, now)
    }
}

/// Emite un ticket de juego con TTL acotado y nonce aleatorio.
pub fn issue(subject: Uuid, ttl_secs: i64, secret: &str) -> Result<String, AppError> {
    issue_at(subject, ttl_secs, secret, now_unix())
}

/// Variante determinista para tests y futuros adaptadores de reloj.
pub fn issue_at(subject: Uuid, ttl_secs: i64, secret: &str, now: i64) -> Result<String, AppError> {
    validate_secret(secret)?;
    if now < 0 {
        return Err(AppError::Internal("reloj de ticket inválido".into()));
    }

    let ttl = normalize_ttl(ttl_secs);
    let expires_at = now
        .checked_add(ttl)
        .ok_or_else(|| AppError::Internal("expiración de ticket fuera de rango".into()))?;
    let nonce = Uuid::new_v4();
    let payload = format!("{PROTOCOL_VERSION}.{PURPOSE}.{subject}.{expires_at}.{nonce}");
    let signature = sign(&payload, secret)?;
    Ok(format!("{payload}.{signature}"))
}

/// Verifica firma, propósito, subject UUID y expiración sin consumir el ticket.
pub fn verify(token: &str, secret: &str) -> Result<GameTicketClaims, AppError> {
    verify_at(token, secret, now_unix())
}

/// Variante determinista para tests y futuros adaptadores de reloj.
pub fn verify_at(token: &str, secret: &str, now: i64) -> Result<GameTicketClaims, AppError> {
    validate_secret(secret)?;
    if now < 0 {
        return Err(AppError::Internal("reloj de ticket inválido".into()));
    }

    if token.len() > GAME_TICKET_MAX_TOKEN_BYTES {
        return Err(AppError::Unauthorized);
    }

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 6
        || parts[0] != PROTOCOL_VERSION
        || parts[1] != PURPOSE
        || parts[2].is_empty()
        || parts[3].is_empty()
        || parts[4].is_empty()
        || parts[5].is_empty()
    {
        return Err(AppError::Unauthorized);
    }

    let subject = parts[2]
        .parse::<Uuid>()
        .map_err(|_| AppError::Unauthorized)?;
    let expires_at = parts[3]
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized)?;
    let nonce = parts[4]
        .parse::<Uuid>()
        .map_err(|_| AppError::Unauthorized)?;
    let payload = format!("{PROTOCOL_VERSION}.{PURPOSE}.{subject}.{expires_at}.{nonce}");
    let signature = hex::decode(parts[5]).map_err(|_| AppError::Unauthorized)?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::Internal("secreto HMAC inválido".into()))?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| AppError::Unauthorized)?;

    if expires_at <= now {
        return Err(AppError::Forbidden("ticket de juego expirado".into()));
    }

    Ok(GameTicketClaims {
        subject,
        expires_at,
        nonce,
    })
}

fn consume_at(
    consumed: &Mutex<HashMap<Uuid, i64>>,
    token: &str,
    secret: &str,
    now: i64,
) -> Result<GameTicketClaims, AppError> {
    let claims = verify_at(token, secret, now)?;
    let mut consumed = consumed
        .lock()
        .map_err(|_| AppError::Internal("almacén de replay no disponible".into()))?;
    consumed.retain(|_, expires_at| *expires_at > now);
    if consumed.contains_key(&claims.nonce) {
        return Err(AppError::Unauthorized);
    }
    if consumed.len() >= GAME_TICKET_MAX_REPLAY_ENTRIES {
        return Err(AppError::Internal(
            "almacén de replay de tickets lleno".into(),
        ));
    }
    consumed.insert(claims.nonce, claims.expires_at);
    Ok(claims)
}

fn validate_secret(secret: &str) -> Result<(), AppError> {
    if secret.trim().is_empty() {
        return Err(AppError::Internal(
            "secreto de tickets no configurado".into(),
        ));
    }
    Ok(())
}

fn normalize_ttl(ttl_secs: i64) -> i64 {
    if ttl_secs <= 0 {
        GAME_TICKET_DEFAULT_TTL_SECS
    } else {
        ttl_secs.min(GAME_TICKET_MAX_TTL_SECS)
    }
}

fn sign(payload: &str, secret: &str) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::Internal("secreto HMAC inválido".into()))?;
    mac.update(payload.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::{
        issue_at, verify_at, GameTicketReplayStore, GAME_TICKET_DEFAULT_TTL_SECS,
        GAME_TICKET_MAX_TTL_SECS,
    };
    use uuid::Uuid;

    const NOW: i64 = 1_700_000_000;
    const SECRET: &str = "test-game-ticket-secret";

    #[test]
    fn issues_ticket_with_uuid_subject_and_bounded_ttl() {
        let subject = Uuid::new_v4();
        let token = issue_at(subject, GAME_TICKET_MAX_TTL_SECS * 10, SECRET, NOW).expect("ticket");
        let claims = verify_at(&token, SECRET, NOW).expect("claims");
        assert_eq!(claims.subject, subject);
        assert_eq!(claims.expires_at, NOW + GAME_TICKET_MAX_TTL_SECS);
        assert!(token.starts_with("g1.game."));
    }

    #[test]
    fn non_positive_ttl_uses_safe_default() {
        let token = issue_at(Uuid::new_v4(), 0, SECRET, NOW).expect("ticket");
        let claims = verify_at(&token, SECRET, NOW).expect("claims");
        assert_eq!(claims.expires_at, NOW + GAME_TICKET_DEFAULT_TTL_SECS);
    }

    #[test]
    fn rejects_tampering_wrong_secret_wrong_purpose_and_malformed_tokens() {
        let token = issue_at(Uuid::new_v4(), 30, SECRET, NOW).expect("ticket");
        let mut tampered = token.clone();
        tampered.push('x');
        assert!(verify_at(&tampered, SECRET, NOW).is_err());
        assert!(verify_at(&token, "wrong-secret", NOW).is_err());
        assert!(verify_at(&token.replacen("g1.game.", "g1.account.", 1), SECRET, NOW).is_err());
        assert!(verify_at("g1.game.not-a-uuid.1.2.sig", SECRET, NOW).is_err());
    }

    #[test]
    fn rejects_expired_and_invalid_clock_tickets() {
        let token = issue_at(Uuid::new_v4(), 30, SECRET, NOW).expect("ticket");
        assert!(verify_at(&token, SECRET, NOW + 30).is_err());
        assert!(issue_at(Uuid::new_v4(), 30, SECRET, -1).is_err());
        assert!(verify_at(&token, SECRET, -1).is_err());
    }

    #[test]
    fn consumes_ticket_once_and_prunes_expired_replay_entries() {
        let store = GameTicketReplayStore::default();
        let token = issue_at(Uuid::new_v4(), 30, SECRET, NOW).expect("ticket");
        assert!(store.consume_at_for_test(&token, SECRET, NOW).is_ok());
        assert!(store.consume_at_for_test(&token, SECRET, NOW + 1).is_err());
        assert_eq!(store.consumed.lock().expect("store").len(), 1);

        let fresh_token = issue_at(Uuid::new_v4(), 30, SECRET, NOW + 31).expect("ticket");
        assert!(store
            .consume_at_for_test(&fresh_token, SECRET, NOW + 31)
            .is_ok());
        assert_eq!(store.consumed.lock().expect("store").len(), 1);
    }

    #[test]
    fn rejects_oversized_tokens_before_parsing() {
        let oversized = "x".repeat(super::GAME_TICKET_MAX_TOKEN_BYTES + 1);
        assert!(verify_at(&oversized, SECRET, NOW).is_err());
    }

    #[test]
    fn rejects_empty_secret_before_parsing() {
        assert!(issue_at(Uuid::new_v4(), 30, " ", NOW).is_err());
        assert!(verify_at("malformed", " ", NOW).is_err());
    }
}
