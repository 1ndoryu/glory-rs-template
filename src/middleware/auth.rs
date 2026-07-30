use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Method;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::user::UserRole;
use crate::repositories::UserRepository;
use crate::services::{AuthService, SessionService};
use crate::AppState;

/// Nombre de la cookie de sesión
const SESSION_COOKIE: &str = "session_id";
/// Nombre de la cookie CSRF
const CSRF_COOKIE: &str = "csrf_token";

/// Extractor que valida sesión (cookie o JWT fallback) y extrae el `user_id`.
/// [297A-8] Lee cookie `session_id` primero, fallback a JWT Bearer.
pub struct AuthUser {
    pub user_id: Uuid,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user_id = resolve_user_id(parts, state).await?;

        /* [297A-8] CSRF check para mutaciones vía cookie de sesión */
        if is_mutation(&parts.method) && has_session_cookie(parts) {
            verify_csrf(parts)?;
        }

        Ok(Self { user_id })
    }
}

/// Extractor que valida sesión Y verifica que el usuario sea admin.
/// [297A-8] Lee cookie `session_id` primero, fallback a JWT Bearer.
pub struct AdminUser {
    pub user_id: Uuid,
}

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user_id = resolve_user_id(parts, state).await?;

        /* [297A-8] CSRF check para mutaciones vía cookie de sesión */
        if is_mutation(&parts.method) && has_session_cookie(parts) {
            verify_csrf(parts)?;
        }

        /* Verificar que el usuario existe, está activo y es admin */
        let user = UserRepository::find_by_id(&state.pool, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Error verificando usuario: {e}")))?
            .ok_or(AppError::Unauthorized)?;

        if user.role != UserRole::Admin {
            return Err(AppError::Forbidden("Se requiere rol administrador".into()));
        }

        Ok(Self { user_id })
    }
}

/// Resuelve el `user_id` intentando cookie de sesión primero, luego JWT Bearer.
async fn resolve_user_id(parts: &Parts, state: &AppState) -> Result<Uuid, AppError> {
    /* Intento 1: cookie de sesión opaca */
    if let Some(raw_token) = extract_cookie(parts, SESSION_COOKIE) {
        if let Some(session) = SessionService::validate(&state.pool, raw_token)
            .await
            .map_err(|e| AppError::Internal(format!("Error validando sesión: {e}")))?
        {
            return Ok(session.user_id);
        }
    }

    /* Intento 2 (fallback): JWT Bearer header — compatibilidad durante transición */
    if let Ok(token) = extract_bearer_token(parts) {
        if let Ok(claims) = AuthService::verify_token(token, &state.jwt_secret) {
            return Ok(claims.sub);
        }
    }

    Err(AppError::Unauthorized)
}

/// Verifica el token CSRF: compara cookie `csrf_token` con header `X-CSRF-Token`
fn verify_csrf(parts: &Parts) -> Result<(), AppError> {
    let csrf_cookie = extract_cookie(parts, CSRF_COOKIE)
        .ok_or_else(|| AppError::Forbidden("CSRF token missing from cookie".into()))?;

    let csrf_header = parts
        .headers
        .get("X-CSRF-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Forbidden("CSRF token missing from header".into()))?;

    if csrf_cookie != csrf_header {
        return Err(AppError::Forbidden("CSRF token mismatch".into()));
    }

    Ok(())
}

/// Extrae una cookie por nombre del header Cookie
fn extract_cookie<'a>(parts: &'a Parts, name: &str) -> Option<&'a str> {
    let cookie_header = parts.headers.get("cookie")?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(name).and_then(|s| s.strip_prefix('=')) {
            return Some(value);
        }
    }
    None
}

/// Determina si el método HTTP es una mutación
fn is_mutation(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// Verifica si existe cookie de sesión
fn has_session_cookie(parts: &Parts) -> bool {
    extract_cookie(parts, SESSION_COOKIE).is_some()
}

/// Extrae el token Bearer del header Authorization (fallback JWT)
fn extract_bearer_token(parts: &Parts) -> Result<&str, AppError> {
    let auth_header = parts
        .headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)
}
