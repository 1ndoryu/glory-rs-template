use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{FotoPublica, EXTENSIONES_FOTO};
use crate::services::InmuebleService;
use crate::AppState;

/* [159A-2] Subida de fotos como bytes crudos (sin dependencias nuevas) +
 * servicio estático público de `/uploads`. La clave interna `storage_key`
 * nunca sale del servidor: la API expone `url`. */

/// Parámetros de subida por query — el cuerpo son los bytes de la imagen
#[derive(Debug, Deserialize)]
pub struct UploadParams {
    /// Inmueble destino
    pub inmueble_id: Uuid,
    /// Nombre original (solo se usa su extensión)
    pub filename: String,
    /// `original` (defecto) o `mejorada`
    pub origen: Option<String>,
    /// Posición explícita (defecto: siguiente hueco)
    pub orden: Option<i32>,
}

/// Subir una foto a un inmueble (requiere JWT)
#[utoipa::path(
    post,
    path = "/api/admin/fotos/upload",
    params(
        ("inmueble_id" = Uuid, Query, description = "Inmueble destino"),
        ("filename" = String, Query, description = "Nombre original (vale su extensión)"),
        ("origen" = Option<String>, Query, description = "original o mejorada"),
        ("orden" = Option<i32>, Query, description = "Posición explícita"),
    ),
    request_body(content = Vec<u8>, description = "Bytes de la imagen (jpg/png/webp, máx 10 MiB)", content_type = "application/octet-stream"),
    responses(
        (status = 201, description = "Foto subida", body = FotoPublica),
        (status = 400, description = "Archivo inválido", body = crate::errors::ErrorResponse),
        (status = 404, description = "Inmueble no encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_foto(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<UploadParams>,
    bytes: Bytes,
) -> Result<(StatusCode, Json<FotoPublica>), AppError> {
    let foto = InmuebleService::subir_foto(
        &state.pool,
        &state.upload_dir,
        params.inmueble_id,
        &params.filename,
        params.origen.as_deref(),
        params.orden,
        bytes.as_ref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(foto)))
}

/// Servir un archivo subido — público, sin JWT (las fotos públicas cuelgan de aquí)
#[utoipa::path(
    get,
    path = "/uploads/{inmueble}/{archivo}",
    params(
        ("inmueble" = Uuid, Path, description = "ID del inmueble"),
        ("archivo" = String, Path, description = "Archivo <uuid>.<ext>"),
    ),
    responses(
        (status = 200, description = "Imagen", body = Vec<u8>, content_type = "image/jpeg"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse)
    )
)]
pub async fn servir_archivo(
    State(state): State<AppState>,
    Path((inmueble, archivo)): Path<(String, String)>,
    cabeceras: HeaderMap,
) -> Result<Response, AppError> {
    let clave = clave_valida(&format!("{inmueble}/{archivo}"))
        .ok_or_else(|| AppError::NotFound("No encontrado".into()))?;
    /* [249A-1] Backfill perezoso del thumb: si falta y existe el
     * original, se genera al vuelo, se guarda y se sirve. Asi las
     * fotos existentes ganan miniatura sin migraciones ni exec.
     * [249A-4] El original se deriva quitando `min160-` (o el legado
     * `thumb-`); al regenerar en nuevo formato se borra el legado. */
    let directa = tokio::fs::read(InmuebleService::ruta_archivo(&state.upload_dir, &clave)).await;
    let bytes = if let Ok(b) = directa {
        b
    } else {
        let original = clave
            .split_once('/')
            .and_then(|(c, a)| {
                a.strip_prefix("min160-")
                    .or_else(|| a.strip_prefix("thumb-"))
                    .map(|o| format!("{c}/{o}"))
            })
            .unwrap_or_default();
        let crudos = tokio::fs::read(InmuebleService::ruta_archivo(&state.upload_dir, &original))
            .await
            .map_err(|_| AppError::NotFound("No encontrado".into()))?;
        let mini = InmuebleService::miniatura(&crudos)
            .ok_or_else(|| AppError::NotFound("No encontrado".into()))?;
        tokio::fs::write(
            InmuebleService::ruta_archivo(&state.upload_dir, &clave),
            &mini,
        )
        .await
        .map_err(AppError::from)?;
        /* El legado de 320 px muere al regenerar (si no existe, aviso y
         * nada más: es limpieza, no parte de la respuesta). */
        if clave.contains("/min160-") {
            let legada = clave.replacen("min160-", "thumb-", 1);
            if let Err(e) =
                tokio::fs::remove_file(InmuebleService::ruta_archivo(&state.upload_dir, &legada))
                    .await
            {
                tracing::debug!("Thumb legado no borrado ({legada}): {e}");
            }
        }
        mini
    };
    let mime = match std::path::Path::new(&clave)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(e) if e.eq_ignore_ascii_case("png") => "image/png",
        Some(e) if e.eq_ignore_ascii_case("webp") => "image/webp",
        _ => "image/jpeg",
    };
    Ok(respuesta_archivo(bytes, mime, &cabeceras))
}

/// Servir una foto de solicitud — pública, sin JWT (vista previa del modal
/// y revisión del admin cuelgan de aquí; la clave la genera el servidor)
#[utoipa::path(
    get,
    path = "/uploads/solicitudes/{sesion}/{archivo}",
    params(
        ("sesion" = Uuid, Path, description = "Carpeta de sesión de subida"),
        ("archivo" = String, Path, description = "Archivo <uuid>.<ext>"),
    ),
    responses(
        (status = 200, description = "Imagen", body = Vec<u8>, content_type = "image/jpeg"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse)
    )
)]
pub async fn servir_archivo_solicitud(
    State(state): State<AppState>,
    Path((sesion, archivo)): Path<(String, String)>,
    cabeceras: HeaderMap,
) -> Result<Response, AppError> {
    let clave = clave_solicitud_valida(&sesion, &archivo)
        .ok_or_else(|| AppError::NotFound("No encontrado".into()))?;
    let bytes = tokio::fs::read(InmuebleService::ruta_archivo(&state.upload_dir, &clave))
        .await
        .map_err(|_| AppError::NotFound("No encontrado".into()))?;
    let mime = match std::path::Path::new(&clave)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(e) if e.eq_ignore_ascii_case("png") => "image/png",
        Some(e) if e.eq_ignore_ascii_case("webp") => "image/webp",
        _ => "image/jpeg",
    };
    Ok(respuesta_archivo(bytes, mime, &cabeceras))
}

/// Servir una foto entrante de `WhatsApp` — pública, sin JWT (el panel staff
/// y la web cuelgan de aquí; la clave la genera el servidor al archivar).
/// Solo `whatsapp/<7..15 dígitos>/<uuid>.<ext permitida>`: el teléfono es
/// dígitos (salida de `normalizar_telefono`, sin `..` posible) y el archivo
/// uuid+extensión, igual que el resto de claves.
#[utoipa::path(
    get,
    path = "/uploads/whatsapp/{telefono}/{archivo}",
    params(
        ("telefono" = String, Path, description = "Teléfono normalizado (solo dígitos)"),
        ("archivo" = String, Path, description = "Archivo <uuid>.<ext>"),
    ),
    responses(
        (status = 200, description = "Imagen", body = Vec<u8>, content_type = "image/jpeg"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse)
    )
)]
pub async fn servir_archivo_whatsapp(
    State(state): State<AppState>,
    Path((telefono, archivo)): Path<(String, String)>,
    cabeceras: HeaderMap,
) -> Result<Response, AppError> {
    let clave = clave_whatsapp_valida(&telefono, &archivo)
        .ok_or_else(|| AppError::NotFound("No encontrado".into()))?;
    let bytes = tokio::fs::read(InmuebleService::ruta_archivo(&state.upload_dir, &clave))
        .await
        .map_err(|_| AppError::NotFound("No encontrado".into()))?;
    /* [299A-3] Las notas de voz se archivan como `.ogg/.mp3/.m4a`
     * (`descargar_y_guardar` en `whatsapp.rs`, E12): sin estas ramas el
     * `<audio>` del hilo pedía `image/jpeg` y el navegador no reproducía. */
    let mime = match std::path::Path::new(&clave)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(e) if e.eq_ignore_ascii_case("png") => "image/png",
        Some(e) if e.eq_ignore_ascii_case("webp") => "image/webp",
        Some(e) if e.eq_ignore_ascii_case("ogg") => "audio/ogg",
        Some(e) if e.eq_ignore_ascii_case("mp3") => "audio/mpeg",
        Some(e) if e.eq_ignore_ascii_case("m4a") => "audio/mp4",
        _ => "image/jpeg",
    };
    Ok(respuesta_archivo(bytes, mime, &cabeceras))
}

/* [299A-3] Audio que archiva el webhook (`descargar_y_guardar` en
 * `whatsapp.rs`: `audio/ogg`→`.ogg`, `audio/mpeg`→`.mp3`, `audio/mp4`→`.m4a`).
 * Antes solo valían fotos y el `<audio>` del hilo devolvía 404. */
const EXTENSIONES_AUDIO_WHATSAPP: &[&str] = &[".ogg", ".mp3", ".m4a"];

fn clave_whatsapp_valida(telefono: &str, archivo: &str) -> Option<String> {
    if !(7..=15).contains(&telefono.len())
        || !telefono.bytes().all(|b| b.is_ascii_digit())
        || archivo.contains('/')
        || archivo.contains('\\')
    {
        return None;
    }
    let punto = archivo.rfind('.')?;
    Uuid::parse_str(&archivo[..punto]).ok()?;
    let extension = archivo[punto..].to_lowercase();
    if EXTENSIONES_FOTO.iter().any(|e| *e == extension)
        || EXTENSIONES_AUDIO_WHATSAPP.iter().any(|e| *e == extension)
    {
        Some(format!("whatsapp/{telefono}/{}", archivo.to_lowercase()))
    } else {
        None
    }
}
/* [249A-1] Fotos con caché larga + ETag: las URLs llevan `?v=<updated_at>`
 * y el backend toca el inmueble al subir/borrar, así `immutable` es seguro.
 * `If-None-Match` coincidente devuelve 304 sin reenviar bytes. */
fn respuesta_archivo(bytes: Vec<u8>, mime: &'static str, cabeceras: &HeaderMap) -> Response {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut suma = DefaultHasher::new();
    bytes.hash(&mut suma);
    let etag = format!("\"{:x}-{}\"", suma.finish(), bytes.len());
    if cabeceras
        .get(header::IF_NONE_MATCH)
        .is_some_and(|v| v.as_bytes() == etag.as_bytes())
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    let mut mapa = HeaderMap::new();
    mapa.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    mapa.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    if let Ok(valor) = HeaderValue::from_str(&etag) {
        mapa.insert(header::ETAG, valor);
    }
    (mapa, bytes).into_response()
}

/// Solo `solicitudes/<uuid>/<uuid>.<ext permitida>`: el prefijo es fijo,
/// nada de `..`, subrutas ni extensiones raras
fn clave_solicitud_valida(sesion: &str, archivo: &str) -> Option<String> {
    if archivo.contains('/') || archivo.contains('\\') {
        return None;
    }
    Uuid::parse_str(sesion).ok()?;
    let punto = archivo.rfind('.')?;
    Uuid::parse_str(&archivo[..punto]).ok()?;
    let extension = archivo[punto..].to_lowercase();
    if EXTENSIONES_FOTO.iter().any(|e| *e == extension) {
        Some(format!("solicitudes/{sesion}/{}", archivo.to_lowercase()))
    } else {
        None
    }
}
/// Solo `<uuid>/<uuid>.<ext permitida>` (nada de `..`, subrutas ni extensiones raras)
/// más su miniatura `min160-<uuid>.jpg` ([249A-4]; el legado `thumb-` de
/// [249A-1] se sigue sirviendo mientras exista en disco).
fn clave_valida(ruta: &str) -> Option<String> {
    let (inmueble, archivo) = ruta.split_once('/')?;
    if archivo.contains('/') || archivo.contains('\\') {
        return None;
    }
    Uuid::parse_str(inmueble).ok()?;
    let punto = archivo.rfind('.')?;
    let base = archivo
        .strip_prefix("min160-")
        .or_else(|| archivo.strip_prefix("thumb-"))
        .unwrap_or(archivo);
    let base_sin_ext = base.strip_suffix(&archivo[punto..]).unwrap_or(base);
    Uuid::parse_str(base_sin_ext).ok()?;
    let extension = archivo[punto..].to_lowercase();
    if EXTENSIONES_FOTO.iter().any(|e| *e == extension) {
        Some(format!("{inmueble}/{}", archivo.to_lowercase()))
    } else {
        None
    }
}

pub fn routes() -> Router<AppState> {
    /* Sin el límite por defecto de 2 MB: el servicio impone 10 MiB con 413 */
    Router::new().route(
        "/fotos/upload",
        post(upload_foto).layer(DefaultBodyLimit::disable()),
    )
}

#[cfg(test)]
mod tests {
    use super::{clave_solicitud_valida, clave_valida, clave_whatsapp_valida};

    #[test]
    fn clave_valida_acepta_y_rechaza() {
        let id = "aa03ccec-c424-40e1-86ed-5f862d1bfa8f";
        assert_eq!(
            clave_valida(&format!("{id}/{id}.jpg")),
            Some(format!("{id}/{id}.jpg"))
        );
        assert!(clave_valida(&format!("{id}/{id}.WEBP")).is_some());
        /* [249A-4] Formato actual `min160-` y legado `thumb-`. */
        assert_eq!(
            clave_valida(&format!("{id}/min160-{id}.jpg")),
            Some(format!("{id}/min160-{id}.jpg"))
        );
        assert!(clave_valida(&format!("{id}/thumb-{id}.jpg")).is_some());
        assert!(clave_valida(&format!("{id}/mini-{id}.jpg")).is_none());
        assert!(clave_valida("../secreto.jpg").is_none());
        assert!(clave_valida(&format!("{id}/{id}.exe")).is_none());
        assert!(clave_valida(&format!("{id}/a/b.jpg")).is_none());
        assert!(clave_valida("no-es-uuid/archivo.jpg").is_none());
    }

    #[test]
    fn clave_solicitud_solo_prefijo_y_uuids() {
        let id = "aa03ccec-c424-40e1-86ed-5f862d1bfa8f";
        assert_eq!(
            clave_solicitud_valida(id, &format!("{id}.png")),
            Some(format!("solicitudes/{id}/{id}.png"))
        );
        assert!(clave_solicitud_valida(id, &format!("{id}.WEBP")).is_some());
        assert!(clave_solicitud_valida(id, "../fuga.jpg").is_none());
        assert!(clave_solicitud_valida(id, "no-uuid.jpg").is_none());
        assert!(clave_solicitud_valida(id, &format!("{id}.pdf")).is_none());
        assert!(clave_solicitud_valida("no-es-uuid", &format!("{id}.jpg")).is_none());
    }

    /* [279A-2] Fotos entrantes de WhatsApp: solo dígitos 7..15 + uuid+ext. */
    #[test]
    fn clave_whatsapp_solo_telefono_y_uuid() {
        let id = "aa03ccec-c424-40e1-86ed-5f862d1bfa8f";
        assert_eq!(
            clave_whatsapp_valida("584120825234", &format!("{id}.jpg")),
            Some(format!("whatsapp/584120825234/{id}.jpg"))
        );
        assert!(clave_whatsapp_valida("584120825234", &format!("{id}.WEBP")).is_some());
        /* [299A-3] Notas de voz (E12): `.ogg/.mp3/.m4a` valen, otros no. */
        assert!(clave_whatsapp_valida("584120825234", &format!("{id}.ogg")).is_some());
        assert!(clave_whatsapp_valida("584120825234", &format!("{id}.mp3")).is_some());
        assert!(clave_whatsapp_valida("584120825234", &format!("{id}.m4a")).is_some());
        assert!(clave_whatsapp_valida("584120825234", &format!("{id}.wav")).is_none());
        assert!(clave_whatsapp_valida("584120825234", "../fuga.jpg").is_none());
        assert!(clave_whatsapp_valida("584120825234", "no-uuid.jpg").is_none());
        assert!(clave_whatsapp_valida("584120825234", &format!("{id}.pdf")).is_none());
        assert!(clave_whatsapp_valida("+584120825234", &format!("{id}.jpg")).is_none());
        assert!(clave_whatsapp_valida("123456", &format!("{id}.jpg")).is_none());
        assert!(clave_whatsapp_valida("1234567890123456", &format!("{id}.jpg")).is_none());
        assert!(clave_whatsapp_valida("584120825234", &format!("{id}/x.jpg")).is_none());
    }
}
