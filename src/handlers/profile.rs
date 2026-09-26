/* [044A-43] Endpoints de perfil de usuario: obtener perfil y subir avatar.
Upload multipart con validación MIME (image), max 2MB, guardado en uploads/avatars/.
[074A-23] PATCH /api/profile para actualizar display_name y campos extendidos. */
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::Json;
use axum::Router;
use chrono::Utc;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{ChangePasswordRequest, UpdateProfileRequest, UserResponse};
use crate::repositories::UserRepository;
use crate::services::{AuthService, EmailService};
use crate::AppState;

const MAX_AVATAR_SIZE: usize = 2 * 1024 * 1024;
const ALLOWED_MIME_PREFIXES: &[&str] = &["image/jpeg", "image/png", "image/webp", "image/gif"];

/* GET /api/profile — obtiene el perfil del usuario autenticado */
#[utoipa::path(
    get,
    path = "/api/profile",
    responses(
        (status = 200, description = "Perfil del usuario", body = UserResponse),
        (status = 401, description = "No autenticado"),
    ),
    security(("bearer_auth" = [])),
    tag = "profile"
)]
pub async fn get_profile(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UserResponse>, AppError> {
    let user = UserRepository::find_by_id(&state.pool, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".into()))?;
    Ok(Json(user.into()))
}

/* POST /api/profile/avatar — sube imagen de avatar (multipart) */
#[utoipa::path(
    post,
    path = "/api/profile/avatar",
    responses(
        (status = 200, description = "Avatar actualizado", body = AvatarResponse),
        (status = 400, description = "Archivo inválido"),
        (status = 401, description = "No autenticado"),
    ),
    security(("bearer_auth" = [])),
    tag = "profile"
)]
pub async fn upload_avatar(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<AvatarResponse>, AppError> {
    let mut file_data: Option<(Vec<u8>, String)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Error leyendo multipart: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "avatar" {
            continue;
        }

        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        if !ALLOWED_MIME_PREFIXES
            .iter()
            .any(|m| content_type.starts_with(m))
        {
            return Err(AppError::BadRequest(
                "Tipo de archivo no permitido. Usa JPG, PNG, WebP o GIF.".into(),
            ));
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Error leyendo archivo: {e}")))?;

        if data.len() > MAX_AVATAR_SIZE {
            return Err(AppError::BadRequest(
                "El archivo excede el límite de 2MB.".into(),
            ));
        }

        let ext = match content_type.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "jpg",
        };

        file_data = Some((data.to_vec(), ext.to_string()));
    }

    let (data, ext) = file_data
        .ok_or_else(|| AppError::BadRequest("No se encontró el campo 'avatar'.".into()))?;

    /* [054A-3] Guardar con nombre versionado para invalidar cache del navegador.
     * Antes de escribir, eliminamos avatares anteriores del mismo usuario para no acumular basura. */
    let dir = std::path::Path::new("uploads/avatars");
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| AppError::Internal(format!("Error creando directorio: {e}")))?;

    let avatar_prefix = auth.user_id.to_string();
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| AppError::Internal(format!("Error listando avatares previos: {e}")))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| AppError::Internal(format!("Error leyendo avatar previo: {e}")))?
    {
        let file_name = entry.file_name();
        if file_name.to_string_lossy().starts_with(&avatar_prefix) {
            if let Err(error) = tokio::fs::remove_file(entry.path()).await {
                tracing::warn!(
                    user_id = %auth.user_id,
                    %error,
                    "No se pudo eliminar una version anterior del avatar"
                );
            }
        }
    }

    let filename = format!("{}-{}.{ext}", auth.user_id, Utc::now().timestamp_millis());
    /* [259A-2] Join via helper anti-traversal (rechazo lexico + contencion). */
    let filepath = crate::util::join_write_path(dir, &[&filename])?;
    tokio::fs::write(&filepath, &data)
        .await
        .map_err(|e| AppError::Internal(format!("Error guardando archivo: {e}")))?;

    let avatar_url = format!("/uploads/avatars/{filename}");
    UserRepository::update_avatar(&state.pool, auth.user_id, &avatar_url).await?;

    Ok(Json(AvatarResponse { avatar_url }))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AvatarResponse {
    pub avatar_url: String,
}

/* Rutas de perfil — montadas bajo /api en mod.rs */
pub fn routes() -> Router<AppState> {
    use axum::routing::{get, post, put};
    Router::new()
        .route("/profile", get(get_profile).patch(update_profile))
        .route("/profile/avatar", post(upload_avatar))
        .route("/profile/password", put(change_password))
}

/* [074A-23] PATCH /api/profile — actualiza display_name y campos extendidos */
#[utoipa::path(
    patch,
    path = "/api/profile",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Perfil actualizado", body = UserResponse),
        (status = 400, description = "Datos inválidos"),
        (status = 401, description = "No autenticado"),
    ),
    security(("bearer_auth" = [])),
    tag = "profile"
)]
pub async fn update_profile(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let current_user = UserRepository::find_by_id(&state.pool, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".into()))?;

    let mut email_changed = false;

    if let Some(email) = req
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let email = email.to_ascii_lowercase();
        if !current_user.email.eq_ignore_ascii_case(&email) {
            if let Some(existing) = UserRepository::find_by_email(&state.pool, &email).await? {
                if existing.id != auth.user_id {
                    return Err(AppError::Conflict("Ese email ya está registrado".into()));
                }
            }
            UserRepository::update_email(&state.pool, auth.user_id, &email)
                .await
                .map_err(|e| {
                    if let sqlx::Error::Database(db_error) = &e {
                        if db_error.constraint() == Some("users_email_key") {
                            return AppError::Conflict("Ese email ya está registrado".into());
                        }
                    }
                    AppError::Internal(format!("Error actualizando email: {e}"))
                })?;
            email_changed = true;
        }
    }

    UserRepository::update_profile(
        &state.pool,
        auth.user_id,
        req.display_name.as_deref(),
        req.bio.as_deref(),
        req.linkedin.as_deref(),
        req.twitter.as_deref(),
        req.website.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal(format!("Error actualizando perfil: {e}")))?;

    let user = UserRepository::find_by_id(&state.pool, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".into()))?;

    if email_changed {
        if let Some(config) = &state.email_config {
            EmailService::send_profile_email_changed_new_address(
                config,
                &state.pool,
                &user.email,
                user.display_name.as_deref(),
                &current_user.email,
            )
            .await;

            EmailService::send_profile_email_changed_old_address(
                config,
                &state.pool,
                &current_user.email,
                current_user.display_name.as_deref(),
                &user.email,
            )
            .await;
        }
    }

    Ok(Json(user.into()))
}

/* [205A-2] PUT /api/profile/password — cambia contraseña desde configuración de perfil.
 * Requiere contraseña actual; las cuentas quick_register siguen usando /api/auth/set-password. */
#[utoipa::path(
    put,
    path = "/api/profile/password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Contraseña actualizada"),
        (status = 400, description = "Datos inválidos", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autenticado", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "profile"
)]
pub async fn change_password(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = AuthService::change_password(&state.pool, auth.user_id, req).await?;

    if let Some(config) = &state.email_config {
        EmailService::send_profile_password_changed(
            config,
            &state.pool,
            &user.email,
            user.display_name.as_deref(),
        )
        .await;
    }

    Ok(StatusCode::OK)
}
