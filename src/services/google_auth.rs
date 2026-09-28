/* [155A-1] Google OAuth 2.0 — servicio para el flujo web.
 * Usa reqwest (ya en Cargo.toml) y runtime sqlx queries para evitar
 * invalidar el cache SQLX_OFFLINE al no tocar el struct User.
 * Tabla separada user_google_accounts para vincular google_id ↔ user_id. */

use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::AuthResponse;
use crate::repositories::UserRepository;
use crate::services::auth::{hash_password, AuthService};

pub struct GoogleAuthService;

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

impl GoogleAuthService {
    fn client_id() -> Result<String, AppError> {
        std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| AppError::Internal("GOOGLE_CLIENT_ID no configurado".into()))
    }

    fn client_secret() -> Result<String, AppError> {
        std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| AppError::Internal("GOOGLE_CLIENT_SECRET no configurado".into()))
    }

    fn redirect_uri() -> Result<String, AppError> {
        std::env::var("GOOGLE_REDIRECT_URI")
            .map_err(|_| AppError::Internal("GOOGLE_REDIRECT_URI no configurado".into()))
    }

    /// Genera la URL de redirección a Google OAuth.
    /// Llama a esto antes de redirigir al usuario.
    pub fn get_auth_url() -> Result<String, AppError> {
        let client_id = Self::client_id()?;
        let redirect_uri = Self::redirect_uri()?;

        /* Encodificar manualmente los parámetros que pueden contener chars especiales */
        let client_id_enc = urlencoding::encode(&client_id).into_owned();
        let redirect_enc = urlencoding::encode(&redirect_uri).into_owned();

        Ok(format!(
            "https://accounts.google.com/o/oauth2/v2/auth\
             ?client_id={client_id_enc}\
             &redirect_uri={redirect_enc}\
             &response_type=code\
             &scope=openid%20email%20profile\
             &access_type=online"
        ))
    }

    /// Intercambia un `code` de Google por datos de usuario.
    pub async fn get_user_from_code(http: &Client, code: &str) -> Result<GoogleUserInfo, AppError> {
        let client_id = Self::client_id()?;
        let client_secret = Self::client_secret()?;
        let redirect_uri = Self::redirect_uri()?;

        /* 1. Intercambiar code → access_token */
        let token_resp = http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("code", code),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
                ("redirect_uri", &redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Error al conectar con Google token: {e}")))?;

        if !token_resp.status().is_success() {
            let body = token_resp.text().await.unwrap_or_default();
            tracing::warn!("Google token error: {body}");
            return Err(AppError::Unauthorized);
        }

        let token: GoogleTokenResponse = token_resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Error parseando token Google: {e}")))?;

        /* 2. Obtener info del usuario con el access_token */
        let user_resp = http
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Error al obtener userinfo Google: {e}")))?;

        if !user_resp.status().is_success() {
            let body = user_resp.text().await.unwrap_or_default();
            tracing::warn!("Google userinfo error: {body}");
            return Err(AppError::Unauthorized);
        }

        user_resp
            .json::<GoogleUserInfo>()
            .await
            .map_err(|e| AppError::Internal(format!("Error parseando userinfo Google: {e}")))
    }

    /// Busca o crea un usuario a partir de la información de Google.
    /// Retorna `AuthResponse` igual que login/register estándar.
    pub async fn login_or_create(
        pool: &PgPool,
        http: &Client,
        code: &str,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        let google_user = Self::get_user_from_code(http, code).await?;

        /* 1. ¿google_id ya vinculado a un usuario? */
        let existing_user_id: Option<Uuid> = sqlx::query_scalar!(
            r#"SELECT user_id FROM user_google_accounts WHERE google_id = $1"#,
            &google_user.id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let user = if let Some(uid) = existing_user_id {
            /* Cuenta Google ya vinculada → cargar usuario */
            UserRepository::find_by_id(pool, uid)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::NotFound("Usuario no encontrado".into()))?
        } else {
            /* 2. Buscar usuario por email o crear uno nuevo */
            let user = if let Some(u) = UserRepository::find_by_email(pool, &google_user.email)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
            {
                u
            } else {
                /* Crear usuario con contraseña ficticia (Google es el medio de auth) */
                let placeholder_hash =
                    hash_password(&format!("google-{}-{}", google_user.id, Uuid::new_v4()))?;
                let u = UserRepository::create(pool, &google_user.email, &placeholder_hash, false)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;

                /* [279A-4] Macro compile-time: columna display_name nulable. */
                if let Some(name) = &google_user.name {
                    sqlx::query!(
                        "UPDATE users SET display_name = $2 WHERE id = $1 AND display_name IS NULL",
                        u.id,
                        name
                    )
                    .execute(pool)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                }
                u
            };

            /* [279A-4] Macro compile-time: ON CONFLICT DO NOTHING sin arbiter
             * no requiere inferencia de índice en PREPARE. Los binds $4/$5 se
             * infieren como &str no-nulo; name/picture de Google son opcionales
             * (None solo sin scope profile) y estas columnas son solo-escritura
             * (nadie las lee), asi que '' es equivalente observable a NULL. */
            sqlx::query!(
                r#"INSERT INTO user_google_accounts (user_id, google_id, email, name, picture)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT DO NOTHING"#,
                user.id,
                &google_user.id,
                &google_user.email,
                google_user.name.as_deref().unwrap_or(""),
                google_user.picture.as_deref().unwrap_or("")
            )
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            user
        };

        let effective = user.effective_role();
        let token = AuthService::generate_token(user.id, user.role, effective, None, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
            email: user.email.clone(),
            role: user.role,
            effective_role: effective,
            impersonating: false,
            needs_password: false,
        })
    }
}
