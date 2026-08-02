//! GAME-01 — Tickets de identidad para transporte realtime.
//!
//! Este módulo no abre WebSocket ni conoce Axum. El token transporta solo un
//! handle aleatorio firmado; el UUID del subject permanece en `GameTicketStore`
//! y nunca cruza el boundary del navegador.

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
pub const GAME_TICKET_MAX_PENDING_ENTRIES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTicketClaims {
    /// UUID resuelto por el servidor desde el handle; nunca procede del token.
    pub subject: Uuid,
    pub expires_at: i64,
    pub nonce: Uuid,
}

#[derive(Debug, Clone, Copy)]
struct PendingTicket {
    subject: Uuid,
    expires_at: i64,
}

/// Almacén single-use local para el primer despliegue single-instance.
///
/// Una futura topología multi-réplica deberá sustituir este adaptador por un
/// store compartido antes de habilitar tickets entre instancias.
#[derive(Clone, Default)]
pub struct GameTicketStore {
    pending: Arc<Mutex<HashMap<Uuid, PendingTicket>>>,
}

impl GameTicketStore {
    /// Emite un ticket opaco: el UUID solo queda en este store server-side.
    pub fn issue(&self, subject: Uuid, ttl_secs: i64, secret: &str) -> Result<String, AppError> {
        issue_at(self, subject, ttl_secs, secret, now_unix())
    }

    /// Verifica y consume el ticket de forma atómica para impedir replay.
    pub fn consume(&self, token: &str, secret: &str) -> Result<GameTicketClaims, AppError> {
        consume_at(self, token, secret, now_unix())
    }

    #[cfg(test)]
    fn consume_at_for_test(
        &self,
        token: &str,
        secret: &str,
        now: i64,
    ) -> Result<GameTicketClaims, AppError> {
        consume_at(self, token, secret, now)
    }
}

/// Variante determinista para tests y futuros adaptadores de reloj.
pub fn issue_at(
    store: &GameTicketStore,
    subject: Uuid,
    ttl_secs: i64,
    secret: &str,
    now: i64,
) -> Result<String, AppError> {
    validate_secret(secret)?;
    if now < 0 {
        return Err(AppError::Internal("reloj de ticket inválido".into()));
    }

    let ttl = normalize_ttl(ttl_secs);
    let expires_at = now
        .checked_add(ttl)
        .ok_or_else(|| AppError::Internal("expiración de ticket fuera de rango".into()))?;
    let nonce = Uuid::new_v4();
    let mut pending = store
        .pending
        .lock()
        .map_err(|_| AppError::Internal("almacén de tickets no disponible".into()))?;
    pending.retain(|_, ticket| ticket.expires_at > now);
    if pending.len() >= GAME_TICKET_MAX_PENDING_ENTRIES {
        return Err(AppError::Internal("almacén de tickets lleno".into()));
    }
    pending.insert(
        nonce,
        PendingTicket {
            subject,
            expires_at,
        },
    );
    drop(pending);

    let payload = format!("{PROTOCOL_VERSION}.{PURPOSE}.{nonce}.{expires_at}");
    let signature = sign(&payload, secret)?;
    Ok(format!("{payload}.{signature}"))
}

/// Variante determinista para tests y futuros adaptadores de reloj.
pub fn consume_at(
    store: &GameTicketStore,
    token: &str,
    secret: &str,
    now: i64,
) -> Result<GameTicketClaims, AppError> {
    validate_secret(secret)?;
    if now < 0 {
        return Err(AppError::Internal("reloj de ticket inválido".into()));
    }
    if token.len() > GAME_TICKET_MAX_TOKEN_BYTES {
        return Err(AppError::Unauthorized);
    }

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 5
        || parts[0] != PROTOCOL_VERSION
        || parts[1] != PURPOSE
        || parts[2].is_empty()
        || parts[3].is_empty()
        || parts[4].is_empty()
    {
        return Err(AppError::Unauthorized);
    }

    let nonce = parts[2]
        .parse::<Uuid>()
        .map_err(|_| AppError::Unauthorized)?;
    let expires_at = parts[3]
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized)?;
    let payload = format!("{PROTOCOL_VERSION}.{PURPOSE}.{nonce}.{expires_at}");
    let signature = hex::decode(parts[4]).map_err(|_| AppError::Unauthorized)?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::Internal("secreto HMAC inválido".into()))?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| AppError::Unauthorized)?;

    let mut pending = store
        .pending
        .lock()
        .map_err(|_| AppError::Internal("almacén de tickets no disponible".into()))?;
    pending.retain(|_, ticket| ticket.expires_at > now);
    let Some(ticket) = pending.remove(&nonce) else {
        return Err(AppError::Unauthorized);
    };
    if ticket.expires_at != expires_at {
        return Err(AppError::Unauthorized);
    }

    Ok(GameTicketClaims {
        subject: ticket.subject,
        expires_at: ticket.expires_at,
        nonce,
    })
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
        consume_at, issue_at, GameTicketStore, GAME_TICKET_DEFAULT_TTL_SECS,
        GAME_TICKET_MAX_TTL_SECS,
    };
    use uuid::Uuid;

    const NOW: i64 = 1_700_000_000;
    const SECRET: &str = "test-game-ticket-secret";

    #[test]
    fn issues_opaque_ticket_and_resolves_uuid_only_server_side() {
        let store = GameTicketStore::default();
        let subject = Uuid::new_v4();
        let token =
            issue_at(&store, subject, GAME_TICKET_MAX_TTL_SECS * 10, SECRET, NOW).expect("ticket");
        assert!(!token.contains(&subject.to_string()));
        let claims = consume_at(&store, &token, SECRET, NOW).expect("claims");
        assert_eq!(claims.subject, subject);
        assert_eq!(claims.expires_at, NOW + GAME_TICKET_MAX_TTL_SECS);
        assert!(token.starts_with("g1.game."));
    }

    #[test]
    fn non_positive_ttl_uses_safe_default() {
        let store = GameTicketStore::default();
        let token = issue_at(&store, Uuid::new_v4(), 0, SECRET, NOW).expect("ticket");
        let claims = consume_at(&store, &token, SECRET, NOW).expect("claims");
        assert_eq!(claims.expires_at, NOW + GAME_TICKET_DEFAULT_TTL_SECS);
    }

    #[test]
    fn rejects_tampering_wrong_secret_wrong_purpose_and_malformed_tokens() {
        let store = GameTicketStore::default();
        let token = issue_at(&store, Uuid::new_v4(), 30, SECRET, NOW).expect("ticket");
        let mut tampered = token.clone();
        tampered.push('x');
        assert!(consume_at(&store, &tampered, SECRET, NOW).is_err());
        assert!(consume_at(&store, &token, "wrong-secret", NOW).is_err());
        assert!(consume_at(
            &store,
            &token.replacen("g1.game.", "g1.account.", 1),
            SECRET,
            NOW
        )
        .is_err());
        assert!(consume_at(&store, "g1.game.not-a-uuid.1.2.sig", SECRET, NOW).is_err());
    }

    #[test]
    fn rejects_expired_and_invalid_clock_tickets() {
        let store = GameTicketStore::default();
        let token = issue_at(&store, Uuid::new_v4(), 30, SECRET, NOW).expect("ticket");
        assert!(consume_at(&store, &token, SECRET, NOW + 30).is_err());
        assert!(issue_at(&store, Uuid::new_v4(), 30, SECRET, -1).is_err());
        assert!(consume_at(&store, &token, SECRET, -1).is_err());
    }

    #[test]
    fn consumes_ticket_once_and_prunes_expired_entries() {
        let store = GameTicketStore::default();
        let token = issue_at(&store, Uuid::new_v4(), 30, SECRET, NOW).expect("ticket");
        assert!(store.consume_at_for_test(&token, SECRET, NOW).is_ok());
        assert!(store.consume_at_for_test(&token, SECRET, NOW + 1).is_err());
        assert_eq!(store.pending.lock().expect("store").len(), 0);

        let expired = issue_at(&store, Uuid::new_v4(), 1, SECRET, NOW).expect("ticket");
        let fresh = issue_at(&store, Uuid::new_v4(), 30, SECRET, NOW + 2).expect("ticket");
        assert!(store
            .consume_at_for_test(&expired, SECRET, NOW + 2)
            .is_err());
        assert!(store.consume_at_for_test(&fresh, SECRET, NOW + 2).is_ok());
        assert_eq!(store.pending.lock().expect("store").len(), 0);
    }

    #[test]
    fn rejects_oversized_tokens_before_parsing() {
        let store = GameTicketStore::default();
        let oversized = "x".repeat(super::GAME_TICKET_MAX_TOKEN_BYTES + 1);
        assert!(consume_at(&store, &oversized, SECRET, NOW).is_err());
    }

    #[test]
    fn rejects_empty_secret_before_parsing() {
        let store = GameTicketStore::default();
        assert!(issue_at(&store, Uuid::new_v4(), 30, " ", NOW).is_err());
        assert!(consume_at(&store, "malformed", " ", NOW).is_err());
    }
}
