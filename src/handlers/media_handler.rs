use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::media::{CreateMediaRequest, Media, MediaQueryParams};
use crate::services::media_svc::{classify_media_type, MediaService};
use crate::AppState;

/* Tamano maximo de archivo: 10MB */
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Parametros del listado admin de media.
#[derive(Debug, serde::Deserialize)]
pub struct AdminMediaQueryParams {
    pub file_type: Option<String>,
    pub article_id: Option<Uuid>,
    pub asset_state: Option<String>,
}

/// Subir archivo (admin) — el backend decide el tipo por extensión.
pub async fn upload_media(
    State(state): State<AppState>,
    _auth: AdminUser,
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

                /* [297A-14 F4] El tipo lo decide el backend por extensión;
                 * extensiones no soportadas se rechazan en el boundary. */
                let ext = file_name.rsplit('.').next().unwrap_or("");
                let Some(classifier) = classify_media_type(ext) else {
                    return Err(AppError::BadRequest(format!(
                        "Tipo de archivo no soportado: .{ext}"
                    )));
                };
                file_type = classifier.to_string();

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

/// Listar archivos media (publico): solo envelope active + public + clean.
pub async fn list_media(
    State(state): State<AppState>,
    Query(params): Query<MediaQueryParams>,
) -> Result<Json<Vec<Media>>, AppError> {
    let media =
        MediaService::list_public(&state.pool, params.file_type.as_deref(), params.article_id)
            .await?;
    Ok(Json(media))
}

/// Listar archivos media (admin): envelope activo, todos los estados de asset.
pub async fn list_admin_media(
    State(state): State<AppState>,
    _auth: AdminUser,
    Query(params): Query<AdminMediaQueryParams>,
) -> Result<Json<Vec<Media>>, AppError> {
    let media = MediaService::list_admin(
        &state.pool,
        params.file_type.as_deref(),
        params.article_id,
        params.asset_state.as_deref(),
    )
    .await?;
    Ok(Json(media))
}

/// Listar media en la papelera (admin): envelope trashed.
pub async fn list_trashed_media(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<Vec<Media>>, AppError> {
    let media = MediaService::list_trashed(&state.pool).await?;
    Ok(Json(media))
}

/// Eliminar media (admin) — soft delete del envelope.
pub async fn delete_media(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    MediaService::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Restaurar media desde la papelera (admin).
pub async fn restore_media(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    MediaService::restore(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        /* Público: solo assets clean + public + active */
        .route("/media", get(list_media))
        /* Admin — contrato canónico /admin/media */
        .route("/admin/media", post(upload_media).get(list_admin_media))
        .route("/admin/media/trashed", get(list_trashed_media))
        .route("/admin/media/:id", delete(delete_media))
        .route("/admin/media/:id/restore", post(restore_media))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sqlx::postgres::PgPoolOptions;
    use tower::util::ServiceExt;
    use uuid::Uuid;

    use crate::config::AppConfig;
    use crate::handlers::create_router;
    use crate::AppState;

    async fn test_state() -> AppState {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL es obligatorio para las pruebas HTTP de media");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(&database_url)
            .await
            .expect("la base de datos de pruebas debe estar disponible");

        AppState {
            pool,
            upload_dir: "target/media-http-test-uploads".to_string(),
            resend_api_key: None,
            email_from: "test@example.invalid".to_string(),
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            site_url: "http://localhost:3000".to_string(),
            login_rate_limit: Arc::new(Mutex::new(
                HashMap::<String, (u8, std::time::Instant)>::new(),
            )),
        }
    }

    fn test_config(database_url: String) -> AppConfig {
        AppConfig {
            database_url,
            host: "127.0.0.1".to_string(),
            port: 3000,
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            upload_dir: "target/media-http-test-uploads".to_string(),
            resend_api_key: None,
            email_from: "test@example.invalid".to_string(),
            frontend_dist: "frontend/dist".to_string(),
        }
    }

    fn production_router(state: &AppState) -> axum::Router {
        create_router(
            state.pool.clone(),
            test_config(std::env::var("DATABASE_URL").expect("DATABASE_URL debe existir")),
        )
    }

    /// Crear un fixture de media + envelope con estado explícito.
    async fn insert_media_fixture(
        pool: &sqlx::PgPool,
        visibility: &str,
        lifecycle: &str,
        asset_state: &str,
    ) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO resources (id, kind, title, editorial, visibility, lifecycle) \
             VALUES ($1, 'media', 'fixture', 'ready', $2::visibility_state, $3::lifecycle_state)",
        )
        .bind(id)
        .bind(visibility)
        .bind(lifecycle)
        .execute(pool)
        .await
        .expect("envelope de media de prueba debe insertarse");

        sqlx::query(
            "INSERT INTO media (id, file_path, file_type, file_size, alt_text, asset_state) \
             VALUES ($1, '/uploads/fixture.png', 'image', 100, 'fixture', $2::asset_processing_state)",
        )
        .bind(id)
        .bind(asset_state)
        .execute(pool)
        .await
        .expect("fila de media de prueba debe insertarse");

        id
    }

    async fn cleanup_media_fixture(pool: &sqlx::PgPool, id: Uuid) {
        let _ = sqlx::query("DELETE FROM media WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM resources WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await;
    }

    #[tokio::test]
    async fn admin_media_routes_require_session_through_production_router() {
        let state = test_state().await;
        let router = production_router(&state);

        let delete_uri = format!("/api/admin/media/{}", Uuid::new_v4());
        let restore_uri = format!("/api/admin/media/{}/restore", Uuid::new_v4());
        let cases = [
            ("GET", "/api/admin/media"),
            ("GET", "/api/admin/media/trashed"),
            ("DELETE", delete_uri.as_str()),
            ("POST", restore_uri.as_str()),
        ];

        for (method, uri) in cases {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request de admin media válida"),
                )
                .await
                .expect("router debe responder");

            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{method} {uri} debe exigir sesión"
            );
        }
    }

    #[tokio::test]
    async fn public_media_list_only_exposes_clean_public_active() {
        let state = test_state().await;
        let router = production_router(&state);

        let clean_public = insert_media_fixture(&state.pool, "public", "active", "clean").await;
        let rejected = insert_media_fixture(&state.pool, "public", "active", "rejected").await;
        let private_asset = insert_media_fixture(&state.pool, "private", "active", "clean").await;
        let trashed = insert_media_fixture(&state.pool, "public", "trashed", "clean").await;

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/media")
                    .body(Body::empty())
                    .expect("request pública válida"),
            )
            .await
            .expect("router debe responder");

        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("cuerpo legible");
        let ids: Vec<String> = serde_json::from_slice::<Vec<serde_json::Value>>(&body_bytes)
            .expect("listado público debe ser JSON")
            .into_iter()
            .map(|item| item["id"].as_str().unwrap_or("").to_string())
            .collect();

        cleanup_media_fixture(&state.pool, clean_public).await;
        cleanup_media_fixture(&state.pool, rejected).await;
        cleanup_media_fixture(&state.pool, private_asset).await;
        cleanup_media_fixture(&state.pool, trashed).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            ids.contains(&clean_public.to_string()),
            "clean+public+active visible"
        );
        assert!(!ids.contains(&rejected.to_string()), "rejected oculto");
        assert!(!ids.contains(&private_asset.to_string()), "private oculto");
        assert!(!ids.contains(&trashed.to_string()), "trashed oculto");
    }
}
