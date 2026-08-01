use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AuthResponse, LoginRequest, RegisterRequest};
use crate::repositories::auth_token_repo::{AuthTokenPurpose, AuthTokenRepository};
use crate::repositories::UserRepository;
use crate::services::SessionService;

/// Claims del JWT — `sub` es el `user_id`, `exp` la expiración Unix
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

pub struct AuthService;

impl AuthService {
    /// Registra una cuenta pendiente de verificación y devuelve el token crudo
    /// únicamente para enviarlo por el canal de correo transaccional.
    pub async fn register_verified(
        pool: &PgPool,
        req: &RegisterRequest,
    ) -> Result<(Uuid, String), AppError> {
        if UserRepository::find_by_email(pool, &req.email)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict("Email ya registrado".into()));
        }

        let password_hash = hash_password(&req.password)?;
        let raw_token = generate_action_token();
        let mut tx = pool.begin().await?;
        let user = UserRepository::create_unverified(&mut tx, &req.email, &password_hash).await?;
        AuthTokenRepository::create_tx(
            &mut tx,
            user.id,
            AuthTokenPurpose::EmailVerification,
            &hash_action_token(&raw_token),
            Utc::now() + Duration::hours(24),
        )
        .await?;
        tx.commit().await?;
        Ok((user.id, raw_token))
    }

    pub async fn verify_email(pool: &PgPool, raw_token: &str) -> Result<(), AppError> {
        let user_id = AuthTokenRepository::consume(
            pool,
            AuthTokenPurpose::EmailVerification,
            &hash_action_token(raw_token),
        )
        .await?
        .ok_or_else(|| AppError::BadRequest("Token inválido o expirado".into()))?;
        if !UserRepository::mark_email_verified(pool, user_id).await? {
            return Err(AppError::BadRequest("Cuenta no disponible".into()));
        }
        Ok(())
    }

    /// Responde siempre de forma genérica desde el handler para no enumerar
    /// correos; devuelve el token solo al servicio de correo cuando existe.
    pub async fn issue_password_reset(
        pool: &PgPool,
        email: &str,
    ) -> Result<Option<String>, AppError> {
        let Some(user) = UserRepository::find_by_email(pool, email).await? else {
            return Ok(None);
        };
        if !UserRepository::is_email_verified(pool, user.id).await? {
            return Ok(None);
        }
        let raw_token = generate_action_token();
        AuthTokenRepository::create(
            pool,
            user.id,
            AuthTokenPurpose::PasswordReset,
            &hash_action_token(&raw_token),
            Utc::now() + Duration::hours(1),
        )
        .await?;
        Ok(Some(raw_token))
    }

    pub async fn reset_password(
        pool: &PgPool,
        raw_token: &str,
        password: &str,
    ) -> Result<(), AppError> {
        let user_id = AuthTokenRepository::consume(
            pool,
            AuthTokenPurpose::PasswordReset,
            &hash_action_token(raw_token),
        )
        .await?
        .ok_or_else(|| AppError::BadRequest("Token inválido o expirado".into()))?;
        let password_hash = hash_password(password)?;
        if !UserRepository::update_password(pool, user_id, &password_hash).await? {
            return Err(AppError::BadRequest("Cuenta no disponible".into()));
        }
        SessionService::revoke_all_for_user(pool, user_id).await?;
        Ok(())
    }

    /// Registra un nuevo usuario: valida unicidad, hashea contraseña, genera JWT
    pub async fn register(
        pool: &PgPool,
        req: RegisterRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        if UserRepository::find_by_email(pool, &req.email)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict("Email ya registrado".into()));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))?
            .to_string();

        let user = UserRepository::create(pool, &req.email, &password_hash).await?;
        let token = Self::generate_token(user.id, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
        })
    }

    /// Inicia sesión: verifica credenciales y genera JWT
    pub async fn login(
        pool: &PgPool,
        req: LoginRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        let user = UserRepository::find_by_email(pool, &req.email)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| AppError::Internal(format!("Hash almacenado inválido: {e}")))?;

        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized)?;

        let token = Self::generate_token(user.id, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
        })
    }

    /// Genera un JWT con expiración de 24 horas
    pub fn generate_token(user_id: Uuid, secret: &str) -> Result<String, AppError> {
        let timestamp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .ok_or_else(|| AppError::Internal("Error calculando expiración del token".into()))?
            .timestamp();
        let exp = usize::try_from(timestamp)
            .map_err(|_| AppError::Internal("Timestamp fuera de rango".into()))?;

        let claims = Claims { sub: user_id, exp };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Error generando token: {e}")))
    }

    /// Verifica un JWT y retorna los claims
    pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
    }
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))
        .map(|hash| hash.to_string())
}

fn generate_action_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill(&mut bytes);
    hex::encode(bytes)
}

fn hash_action_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{generate_action_token, hash_action_token};

    #[test]
    fn action_token_is_opaque_and_hash_is_fixed_length() {
        let token = generate_action_token();
        let hash = hash_action_token(&token);
        assert_eq!(token.len(), 64);
        assert_eq!(hash.len(), 64);
        assert_ne!(token, hash);
    }

    #[test]
    fn action_token_hash_is_deterministic_for_lookup() {
        assert_eq!(hash_action_token("token"), hash_action_token("token"));
        assert_ne!(hash_action_token("token"), hash_action_token("other"));
    }
}
