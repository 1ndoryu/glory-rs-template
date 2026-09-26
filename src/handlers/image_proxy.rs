/* [104A-5] Proxy de optimización de imágenes on-demand.
 * Ruta: GET /api/img/{*path}?w={ancho}&q={calidad}&fmt={formato}
 * Procesa imágenes locales de uploads/ y assets/ al vuelo con cache en disco.
 * Headers de cache agresivos (1 año) porque la URL incluye los params. */

use axum::extract::State;
use axum::extract::{Path, Query};
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use std::path::PathBuf;

use crate::errors::AppError;
use crate::services::image_processing::{self, OptimizeParams, OutputFormat};
use crate::AppState;

/* Límites de seguridad para evitar abuso de recursos */
const MAX_WIDTH: u32 = 2400;
const MIN_WIDTH: u32 = 16;
const MIN_QUALITY: u8 = 10;
const MAX_QUALITY: u8 = 100;

/* Anchos permitidos (whitelist) para evitar cache flooding.
 * Solo se permiten estos valores exactos o ninguno (original). */
const ALLOWED_WIDTHS: &[u32] = &[150, 300, 480, 640, 800, 1024, 1200, 1600, 2400];

#[derive(Debug, Deserialize)]
pub struct ImageQueryParams {
    /* Ancho objetivo en píxeles. Debe ser uno de los valores permitidos. */
    pub w: Option<u32>,
    /* Calidad 10-100 (default 80) */
    pub q: Option<u8>,
    /* Formato: webp, jpeg, png (default: mismo que original) */
    pub fmt: Option<String>,
}

/// Proxy de optimización de imágenes
#[utoipa::path(
    get,
    path = "/api/img/{path}",
    params(
        ("path" = String, Path, description = "Ruta relativa de la imagen en uploads/"),
        ("w" = Option<u32>, Query, description = "Ancho objetivo (150,300,480,640,800,1024,1200,1600,2400)"),
        ("q" = Option<u8>, Query, description = "Calidad 10-100 (default 80)"),
        ("fmt" = Option<String>, Query, description = "Formato: webp, jpeg, png"),
    ),
    responses(
        (status = 200, description = "Imagen optimizada"),
        (status = 400, description = "Parámetros inválidos"),
        (status = 404, description = "Imagen no encontrada"),
    ),
    tag = "images"
)]
pub async fn image_proxy(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(params): Query<ImageQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    /* Validar que la ruta no intente path traversal */
    if path.contains("..") || path.starts_with('/') || path.starts_with('\\') {
        return Err(AppError::BadRequest("Ruta inválida".into()));
    }

    /* Validar y normalizar ancho */
    let width = if let Some(w) = params.w {
        if !(MIN_WIDTH..=MAX_WIDTH).contains(&w) {
            return Err(AppError::BadRequest(format!(
                "Ancho debe estar entre {MIN_WIDTH} y {MAX_WIDTH}"
            )));
        }
        /* Snapear al ancho permitido más cercano */
        let snapped = ALLOWED_WIDTHS
            .iter()
            .min_by_key(|&&allowed| (i64::from(allowed) - i64::from(w)).unsigned_abs())
            .copied()
            .unwrap_or(w);
        Some(snapped)
    } else {
        None
    };

    /* Validar calidad */
    let quality = params.q.map_or(80, |q| q.clamp(MIN_QUALITY, MAX_QUALITY));

    /* Validar formato */
    let format = OutputFormat::from_str_opt(params.fmt.as_deref());

    let optimize_params = OptimizeParams {
        width,
        quality,
        format,
    };

    /* Resolver la raíz local permitida según el namespace solicitado */
    let (source_root, original_path) = resolve_source_path(&state, &path)?;

    /* Verificar que el archivo existe y está dentro de la raíz permitida */
    let canonical = original_path.canonicalize().map_err(|e| {
        tracing::warn!(
            path = %path,
            resolved = %original_path.display(),
            error = %e,
            "Imagen no encontrada en disco"
        );
        AppError::NotFound("Imagen no encontrada".into())
    })?;

    let source_root = source_root
        .canonicalize()
        .map_err(|_| AppError::Internal("Directorio de imágenes no encontrado".into()))?;

    if !canonical.starts_with(&source_root) {
        return Err(AppError::BadRequest(
            "Ruta fuera de directorio permitido".into(),
        ));
    }

    /* Verificar que es un formato de imagen soportado */
    let ext = original_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if !["jpg", "jpeg", "png", "webp", "gif"].contains(&ext) {
        return Err(AppError::BadRequest(
            "Formato de archivo no soportado para optimización".into(),
        ));
    }

    /* Si no hay transformación, servir el original con cache headers */
    if width.is_none() && quality == 80 && matches!(format, OutputFormat::Original) {
        let bytes = tokio::fs::read(&original_path)
            .await
            .map_err(|_| AppError::NotFound("Imagen no encontrada".into()))?;

        let content_type = image_processing::mime_from_extension(Some(ext));
        return Ok((
            [
                (header::CONTENT_TYPE, content_type.to_string()),
                (
                    header::CACHE_CONTROL,
                    "public, max-age=31536000, immutable".to_string(),
                ),
            ],
            bytes,
        ));
    }

    /* Procesar con cache */
    let (bytes, content_type) =
        image_processing::get_optimized_image(&original_path, &optimize_params).await?;

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".to_string(),
            ),
        ],
        bytes,
    ))
}

/* [259A-2] Rechazo sintactico de traversal ANTES del join: `path` viene de
 * la URL (ruta /api/img/ con wildcard, input externo). El check
 * canonicalize+starts_with del llamador cubre symlinks; esto cubre `..` y
 * separadores Windows. */
fn resolve_source_path(state: &AppState, path: &str) -> Result<(PathBuf, PathBuf), AppError> {
    if path.split('/').any(|seg| seg == ".." || seg.contains('\\')) {
        return Err(AppError::BadRequest(
            "Ruta fuera de directorio permitido".into(),
        ));
    }
    if path.starts_with("assets/") {
        let root = state
            .static_dir
            .as_deref()
            .map_or_else(|| PathBuf::from("frontend/public"), PathBuf::from);
        return Ok((root.clone(), root.join(path)));
    }

    let root = PathBuf::from("uploads");
    Ok((root.clone(), root.join(path)))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/img/*path", get(image_proxy))
}
