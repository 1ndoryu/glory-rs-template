use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    AuthResponse, GoogleAuthUrlResponse, GoogleLoginRequest, LoginRequest, QuickRegisterRequest,
    RegisterRequest, SetPasswordRequest,
};
use crate::repositories::UserRepository;
use crate::services::{AuditService, AuthService, GoogleAuthService};
use crate::AppState;

/// Registrar nuevo usuario
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Usuario registrado", body = AuthResponse),
        (status = 409, description = "Email ya registrado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = AuthService::register(&state.pool, req, &state.jwt_secret).await?;

    /* [011A-1] Email a admins notificando nuevo registro (non-fatal) */
    if let Some(ref email_cfg) = state.email_config {
        if let Ok(admin_emails) = UserRepository::admin_emails(&state.pool).await {
            if !admin_emails.is_empty() {
                let cfg = email_cfg.clone();
                let pool = state.pool.clone();
                let user_email = response.email.clone();
                let user_name = response.email.clone();
                let user_id = response.user_id;
                let site_url = std::env::var("SITE_URL")
                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                tokio::spawn(async move {
                    crate::services::EmailService::send_new_user_registered_admin(
                        &cfg,
                        &pool,
                        &admin_emails,
                        &user_email,
                        &user_name,
                        user_id,
                        &site_url,
                    )
                    .await;
                });
            }
        }
    }

    Ok((StatusCode::CREATED, Json(response)))
}

/// Iniciar sesión
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login exitoso", body = AuthResponse),
        (status = 401, description = "Credenciales inválidas", body = crate::errors::ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let email = req.email.clone();
    match AuthService::login(&state.pool, req, &state.jwt_secret).await {
        Ok(response) => {
            /* [064A-73] Audit: login exitoso */
            AuditService::log(
                &state.pool,
                "login_success",
                Some(response.user_id),
                None,
                serde_json::json!({"email": email}),
            )
            .await;
            Ok(Json(response))
        }
        Err(e) => {
            /* [064A-73] Audit: login fallido */
            AuditService::log(
                &state.pool,
                "login_failed",
                None,
                None,
                serde_json::json!({"email": email}),
            )
            .await;
            Err(e)
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/quick-register", post(quick_register))
        .route("/auth/login", post(login))
        .route("/auth/set-password", put(set_password))
        .route("/auth/google/url", get(google_auth_url))
        .route("/auth/google/login", post(google_login))
}

/* [064A-3] Registro rapido: solo email, sin password.
 * Pensado para el flujo de compra — crea cuenta con password aleatorio.
 * Si el email ya existe retorna 409 para que el frontend pida password. */
#[utoipa::path(
    post,
    path = "/api/auth/quick-register",
    request_body = QuickRegisterRequest,
    responses(
        (status = 201, description = "Usuario registrado (sin password)", body = AuthResponse),
        (status = 409, description = "Email ya registrado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    )
)]
pub async fn quick_register(
    State(state): State<AppState>,
    Json(req): Json<QuickRegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = AuthService::quick_register(&state.pool, req, &state.jwt_secret).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/* [154A-5] Establece contraseña para usuarios de quick_register (password_set = false).
 * Requiere autenticación. Si el usuario ya tiene contraseña, retorna 400. */
#[utoipa::path(
    put,
    path = "/api/auth/set-password",
    request_body = SetPasswordRequest,
    responses(
        (status = 200, description = "Contraseña establecida"),
        (status = 400, description = "Ya tiene contraseña", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autenticado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_password(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<SetPasswordRequest>,
) -> Result<StatusCode, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    AuthService::set_password(&state.pool, auth.user_id, req).await?;
    Ok(StatusCode::OK)
}

/* [155A-1] Google OAuth 2.0 — flujo web.
 * GET  /api/auth/google/url   → URL de redirección a Google.
 * POST /api/auth/google/login → intercambia ?code= por JWT.
 * El frontend redirige al usuario a `url`, Google vuelve con ?code= a la raíz del SPA,
 * y el SPA detecta ese param y llama al POST. */

#[utoipa::path(
    get,
    path = "/api/auth/google/url",
    responses(
        (status = 200, description = "URL de autenticación con Google", body = GoogleAuthUrlResponse),
        (status = 500, description = "Google no configurado", body = crate::errors::ErrorResponse)
    )
)]
pub async fn google_auth_url() -> Result<Json<GoogleAuthUrlResponse>, AppError> {
    let url = GoogleAuthService::get_auth_url()?;
    Ok(Json(GoogleAuthUrlResponse { url }))
}

#[utoipa::path(
    post,
    path = "/api/auth/google/login",
    request_body = GoogleLoginRequest,
    responses(
        (status = 200, description = "Login con Google exitoso", body = AuthResponse),
        (status = 401, description = "Código inválido o expirado", body = crate::errors::ErrorResponse),
        (status = 500, description = "Error interno", body = crate::errors::ErrorResponse)
    )
)]
pub async fn google_login(
    State(state): State<AppState>,
    Json(req): Json<GoogleLoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let response = GoogleAuthService::login_or_create(
        &state.pool,
        &state.http_client,
        &req.code,
        &state.jwt_secret,
    )
    .await?;
    Ok(Json(response))
}
