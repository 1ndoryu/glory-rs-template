use axum::extract::{ConnectInfo, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Instant;
use validator::Validate;

use argon2::PasswordVerifier;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::user::UserResponse;
use crate::models::{
    ConfirmPasswordResetRequest, LoginRequest, PasswordResetRequest, RegisterRequest,
    RegistrationResponse, VerifyEmailRequest,
};
use crate::repositories::UserRepository;
use crate::services::{AuthService, SessionService};
use crate::AppState;

/// [297A-8] Rate limit: máximo 5 intentos de login por IP por minuto
const MAX_LOGIN_ATTEMPTS: u8 = 5;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Almacén de rate limit por IP (en memoria)
pub type LoginRateLimit = Mutex<HashMap<String, (u8, Instant)>>;

/// Verifica rate limit para una IP. Retorna Ok(()) si permitido, Err si bloqueado.
fn check_rate_limit(rate_limit: &LoginRateLimit, ip: &str) -> Result<(), AppError> {
    let mut map = rate_limit
        .lock()
        .map_err(|e| AppError::Internal(format!("Error verificando rate limit: {e}")))?;

    let now = Instant::now();

    // Limpiar entradas expiradas
    map.retain(|_, (_, instant)| now.duration_since(*instant).as_secs() < RATE_LIMIT_WINDOW_SECS);

    if let Some((count, first)) = map.get_mut(ip) {
        if now.duration_since(*first).as_secs() >= RATE_LIMIT_WINDOW_SECS {
            /* Ventana expirada — resetear */
            map.insert(ip.to_string(), (1, now));
            Ok(())
        } else if *count >= MAX_LOGIN_ATTEMPTS {
            Err(AppError::Forbidden(
                "Demasiados intentos de login. Intenta de nuevo en un minuto.".into(),
            ))
        } else {
            *count += 1;
            Ok(())
        }
    } else {
        map.insert(ip.to_string(), (1, now));
        Ok(())
    }
}

/// Registrar nuevo usuario
/// [297A-7] Registro público deshabilitado por defecto.
/// Solo se permite cuando `registration_enabled = 'true'` en `site_settings`.
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 202, description = "Verificación requerida", body = RegistrationResponse),
        (status = 403, description = "Registro deshabilitado", body = crate::errors::ErrorResponse),
        (status = 409, description = "Email ya registrado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegistrationResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    /* Verificar feature flag de registro */
    let settings = crate::repositories::settings_repo::SettingsRepository::get_all(&state.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error verificando registro: {e}")))?;
    let registration_enabled = settings
        .get("registration_enabled")
        .is_some_and(|v| v == "true");

    if !registration_enabled {
        return Err(AppError::Forbidden(
            "El registro público está deshabilitado".into(),
        ));
    }

    let email = req.email.clone();
    let (_user_id, token) = AuthService::register_verified(&state.pool, &req).await?;
    if let Some(api_key) = state.resend_api_key.as_deref() {
        let link = format!("{}/verify-email?token={token}", state.site_url);
        crate::services::email::EmailService::send_account_link(
            api_key,
            &state.email_from,
            &email,
            "verifica tu cuenta",
            "verifica tu correo",
            &link,
        )
        .await?;
    } else {
        tracing::warn!("Registro creado sin proveedor de correo configurado");
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(RegistrationResponse {
            message: "Revisa tu correo para verificar la cuenta".into(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/verify-email",
    request_body = VerifyEmailRequest,
    responses((status = 200, body = RegistrationResponse), (status = 400, body = crate::errors::ErrorResponse))
)]
pub async fn verify_email(
    State(state): State<AppState>,
    Json(req): Json<VerifyEmailRequest>,
) -> Result<Json<RegistrationResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    AuthService::verify_email(&state.pool, &req.token).await?;
    Ok(Json(RegistrationResponse {
        message: "Cuenta verificada. Ya puedes iniciar sesión".into(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/password-reset",
    request_body = PasswordResetRequest,
    responses((status = 202, body = RegistrationResponse))
)]
pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequest>,
) -> Result<(StatusCode, Json<RegistrationResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if let Some(token) = AuthService::issue_password_reset(&state.pool, &req.email).await? {
        if let Some(api_key) = state.resend_api_key.as_deref() {
            let link = format!("{}/reset-password?token={token}", state.site_url);
            crate::services::email::EmailService::send_account_link(
                api_key,
                &state.email_from,
                &req.email,
                "restablece tu contraseña",
                "restablecer contraseña",
                &link,
            )
            .await?;
        }
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(RegistrationResponse {
            message: "Si la cuenta existe, recibirás instrucciones por correo".into(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/password-reset/confirm",
    request_body = ConfirmPasswordResetRequest,
    responses((status = 204), (status = 400, body = crate::errors::ErrorResponse))
)]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ConfirmPasswordResetRequest>,
) -> Result<StatusCode, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    AuthService::reset_password(&state.pool, &req.token, &req.password).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Iniciar sesión — [297A-8] crea sesión opaca en cookie `HttpOnly`
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login exitoso"),
        (status = 401, description = "Credenciales inválidas", body = crate::errors::ErrorResponse),
        (status = 403, description = "Rate limit", body = crate::errors::ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<(HeaderMap, StatusCode), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    /* [297A-8] Rate limit por IP */
    let ip = addr.ip().to_string();
    check_rate_limit(&state.login_rate_limit, &ip)?;

    /* Verificar credenciales */
    let Some(user) = UserRepository::find_by_email(&state.pool, &req.email)
        .await
        .map_err(|e| AppError::Internal(format!("Error buscando usuario: {e}")))?
    else {
        crate::repositories::auth_audit_repo::AuthAuditRepository::record(
            &state.pool,
            None,
            "login_failed",
            &ip,
            false,
        )
        .await?;
        return Err(AppError::Unauthorized);
    };

    let parsed_hash = argon2::PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::Internal(format!("Hash almacenado inválido: {e}")))?;

    if argon2::Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        crate::repositories::auth_audit_repo::AuthAuditRepository::record(
            &state.pool,
            Some(user.id),
            "login_failed",
            &ip,
            false,
        )
        .await?;
        return Err(AppError::Unauthorized);
    }

    if !UserRepository::is_email_verified(&state.pool, user.id).await? {
        return Err(AppError::Forbidden(
            "Debes verificar tu correo antes de iniciar sesión".into(),
        ));
    }

    /* [297A-8] Crear sesión opaca */
    let session_result = SessionService::create(&state.pool, user.id, Some(&ip), None).await?;
    crate::repositories::auth_audit_repo::AuthAuditRepository::record(
        &state.pool,
        Some(user.id),
        "login_succeeded",
        &ip,
        true,
    )
    .await?;

    /* Construir cookies */
    let mut headers = HeaderMap::new();

    // Cookie de sesión: HttpOnly, Secure en producción, SameSite=Lax
    let session_cookie = format!(
        "session_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session_result.raw_token,
        7 * 24 * 60 * 60, // 7 días
    );
    // En producción añadir Secure
    let session_cookie = if state.site_url.starts_with("https") {
        format!("{session_cookie}; Secure")
    } else {
        session_cookie
    };
    headers.append(
        SET_COOKIE,
        session_cookie
            .parse()
            .map_err(|e| AppError::Internal(format!("Error construyendo cookie de sesión: {e}")))?,
    );

    // Cookie CSRF: NO HttpOnly (el frontend necesita leerla), SameSite=Lax
    let csrf_cookie = format!(
        "csrf_token={}; Path=/; SameSite=Lax; Max-Age={}",
        session_result.csrf_token,
        7 * 24 * 60 * 60,
    );
    let csrf_cookie = if state.site_url.starts_with("https") {
        format!("{csrf_cookie}; Secure")
    } else {
        csrf_cookie
    };
    headers.append(
        SET_COOKIE,
        csrf_cookie
            .parse()
            .map_err(|e| AppError::Internal(format!("Error construyendo cookie CSRF: {e}")))?,
    );

    Ok((headers, StatusCode::NO_CONTENT))
}

/// Obtener usuario actual — [297A-8] lee sesión de cookie
pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    let user = UserRepository::find_by_id(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error buscando usuario: {e}")))?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(UserResponse::from(user)))
}

/// Cerrar sesión — [297A-8] revoca sesión y limpia cookies
pub async fn logout(
    State(state): State<AppState>,
    _auth: AuthUser,
    headers: HeaderMap,
) -> Result<(HeaderMap, StatusCode), AppError> {
    /* Extraer token de sesión para revocarlo */
    let cookie_header = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(token) =
            pair.strip_prefix("session_id=")
                .and_then(|s| if s.is_empty() { None } else { Some(s) })
        {
            let _ = SessionService::revoke_by_token(&state.pool, token).await;
            break;
        }
    }

    /* Limpiar cookies */
    let mut response_headers = HeaderMap::new();
    response_headers.append(
        SET_COOKIE,
        "session_id=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"
            .parse()
            .expect("cookie statique"),
    );
    response_headers.append(
        SET_COOKIE,
        "csrf_token=; Path=/; SameSite=Lax; Max-Age=0"
            .parse()
            .expect("cookie statique"),
    );

    Ok((response_headers, StatusCode::NO_CONTENT))
}

/// Listar sesiones activas del usuario — [297A-8]
pub async fn list_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<crate::services::session::Session>>, AppError> {
    let sessions = SessionService::list_for_user(&state.pool, auth.user_id).await?;
    Ok(Json(sessions))
}

/// Revocar una sesión específica — [297A-8]
pub async fn revoke_session(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Path(session_id): axum::extract::Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    /* Verificar que la sesión pertenece al usuario */
    let sessions = SessionService::list_for_user(&state.pool, auth.user_id).await?;
    if !sessions.iter().any(|s| s.id == session_id) {
        return Err(AppError::NotFound("Sesión no encontrada".into()));
    }

    SessionService::revoke_by_id(&state.pool, session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/verify-email", post(verify_email))
        .route("/auth/password-reset", post(request_password_reset))
        .route("/auth/password-reset/confirm", post(reset_password))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/logout", post(logout))
        .route("/auth/sessions", get(list_sessions))
        .route("/auth/sessions/:id", delete(revoke_session))
}
