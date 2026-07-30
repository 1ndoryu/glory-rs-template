use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::media::{CreateMediaRequest, Media, MediaQueryParams};
use crate::services::media_svc::MediaService;
use crate::AppState;

/* Tamano maximo de archivo: 10MB */
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Subir archivo (admin)
pub async fn upload_media(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Media>), AppError> {
    let mut file_path = String::new();
    let mut file_type = String::new();
    let mut file_size: i64 = 0;
    let mut alt_text = String::new();
    let mut article_id: Option<Uuid> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Error leyendo multipart: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let file_name = field.file_name().unwrap_or("upload").to_string();

                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Error leyendo archivo: {e}")))?;

                /* Limitar tamano del archivo */
                if data.len() > MAX_FILE_SIZE {
                    return Err(AppError::BadRequest(
                        "Archivo excede el limite de 10MB".into(),
                    ));
                }

                file_size = i64::try_from(data.len())
                    .map_err(|_| AppError::BadRequest("Archivo demasiado grande".into()))?;

                /* Determinar tipo por extension */
                let ext = file_name.rsplit('.').next().unwrap_or("");
                file_type = match ext.to_lowercase().as_str() {
                    "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" => "image",
                    "mp3" | "wav" | "ogg" | "flac" => "audio",
                    "mp4" | "webm" | "mov" => "video",
                    _ => "file",
                }
                .to_string();

                /* Sanitizar nombre — prevenir path traversal */
                let sanitized: String = file_name
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
                    .collect::<String>()
                    .trim_start_matches('.')
                    .to_string();

                let safe_name = if sanitized.is_empty() {
                    "upload".to_string()
                } else {
                    sanitized
                };
                let safe_name = format!("{}-{}", chrono::Utc::now().timestamp(), safe_name);

                /* Guardar archivo usando upload_dir del state */
                std::fs::create_dir_all(&state.upload_dir)
                    .map_err(|e| AppError::Internal(format!("Error creando directorio: {e}")))?;

                let dest = format!("{}/{safe_name}", state.upload_dir);
                std::fs::write(&dest, &data)
                    .map_err(|e| AppError::Internal(format!("Error guardando archivo: {e}")))?;

                file_path = format!("/uploads/{safe_name}");
            }
            "alt_text" => {
                alt_text = field.text().await.unwrap_or_default();
            }
            "article_id" => {
                let text = field.text().await.unwrap_or_default();
                if let Ok(id) = Uuid::parse_str(&text) {
                    article_id = Some(id);
                }
            }
            _ => {}
        }
    }

    if file_path.is_empty() {
        return Err(AppError::BadRequest("No se proporciono archivo".into()));
    }

    let media = MediaService::create(
        &state.pool,
        CreateMediaRequest {
            article_id,
            file_path,
            file_type,
            file_size,
            alt_text,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(media)))
}

/// Listar archivos media (publico)
pub async fn list_media(
    State(state): State<AppState>,
    Query(params): Query<MediaQueryParams>,
) -> Result<Json<Vec<Media>>, AppError> {
    let media =
        MediaService::list(&state.pool, params.file_type.as_deref(), params.article_id).await?;
    Ok(Json(media))
}

/// Eliminar archivo media (admin)
pub async fn delete_media(
    State(state): State<AppState>,
    _auth: AuthUser,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<StatusCode, AppError> {
    MediaService::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/media", post(upload_media).get(list_media))
        .route("/media/{id}", axum::routing::delete(delete_media))
}
