/* [104A-5] Servicio de procesamiento de imágenes on-demand.
 * Inspirado en Jetpack Photon CDN: procesa imágenes al vuelo con cache en disco.
 * Soporta resize (por ancho), compresión (calidad), y conversión a WebP.
 * Cache en uploads/.cache/ con key basada en params para evitar re-procesamiento. */

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::DynamicImage;
use image::ImageReader;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::fs;
use tokio::sync::Semaphore;

use crate::errors::AppError;

const CACHE_DIR: &str = "uploads/.cache";
const IMAGE_PROCESSING_CONCURRENCY: usize = 2;

static IMAGE_PROCESSING_PERMITS: OnceLock<Semaphore> = OnceLock::new();

/* Formatos de salida soportados */
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Jpeg,
    Png,
    Webp,
    Original,
}

impl OutputFormat {
    #[must_use]
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s {
            Some("webp") => Self::Webp,
            Some("jpeg" | "jpg") => Self::Jpeg,
            Some("png") => Self::Png,
            _ => Self::Original,
        }
    }

    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Original => "bin",
        }
    }

    #[must_use]
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
            Self::Original => "application/octet-stream",
        }
    }
}

/* Parámetros de optimización */
#[derive(Debug, Clone)]
pub struct OptimizeParams {
    pub width: Option<u32>,
    pub quality: u8,
    pub format: OutputFormat,
}

impl Default for OptimizeParams {
    fn default() -> Self {
        Self {
            width: None,
            quality: 80,
            format: OutputFormat::Original,
        }
    }
}

/* Genera la ruta de cache basada en los parámetros de optimización.
 * Estructura: uploads/.cache/{w}_{q}_{fmt}/{path_original} */
#[must_use]
pub fn cache_path(original_path: &str, params: &OptimizeParams) -> PathBuf {
    let w = params
        .width
        .map_or_else(|| "orig".to_string(), |w| w.to_string());
    let q = params.quality.to_string();
    let fmt = match params.format {
        OutputFormat::Original => "orig",
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Png => "png",
        OutputFormat::Webp => "webp",
    };
    let cache_subdir = format!("{w}_{q}_{fmt}");

    /* Reemplazar la extensión del archivo por la del formato de salida si no es Original */
    let mut cached_name = PathBuf::from(original_path);
    if !matches!(params.format, OutputFormat::Original) {
        cached_name.set_extension(params.format.extension());
    }

    /* [259A-2] Funcion pura (sin IO): canonicalize imposible. Filtrado lexico
     * de componentes — se descartan `..`, raices y prefijos absolutos para que
     * el join nunca escape de CACHE_DIR aunque original_path traiga `..`. */
    let safe_original: PathBuf = cached_name
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .collect();

    PathBuf::from(CACHE_DIR)
        .join(cache_subdir)
        .join(safe_original)
}

/* Procesa una imagen: redimensiona y/o comprime según los parámetros.
 * Retorna (bytes procesados, content_type). */
pub fn process_image(
    original_bytes: &[u8],
    original_mime: &str,
    params: &OptimizeParams,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let img = ImageReader::new(Cursor::new(original_bytes))
        .with_guessed_format()
        .map_err(|e| AppError::Internal(format!("Error leyendo imagen: {e}")))?
        .decode()
        .map_err(|e| AppError::Internal(format!("Error decodificando imagen: {e}")))?;

    /* Redimensionar si se especificó ancho y la imagen es más ancha */
    let img = if let Some(target_width) = params.width {
        if img.width() > target_width {
            img.resize(target_width, u32::MAX, FilterType::Lanczos3)
        } else {
            img
        }
    } else {
        img
    };

    /* Determinar formato de salida: si es Original, usar el formato del archivo original */
    let effective_format = match params.format {
        OutputFormat::Original => match original_mime {
            "image/png" => OutputFormat::Png,
            "image/webp" => OutputFormat::Webp,
            _ => OutputFormat::Jpeg,
        },
        other => other,
    };

    let (bytes, content_type) = encode_image(&img, effective_format, params.quality)?;
    Ok((bytes, content_type))
}

/* Codifica la imagen en el formato especificado con la calidad dada */
fn encode_image(
    img: &DynamicImage,
    format: OutputFormat,
    quality: u8,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let mut buf = Vec::new();

    match format {
        OutputFormat::Jpeg => {
            let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
            img.write_with_encoder(encoder)
                .map_err(|e| AppError::Internal(format!("Error codificando JPEG: {e}")))?;
            Ok((buf, "image/jpeg"))
        }
        OutputFormat::Webp => {
            /* [175A-2] Codificación WebP lossy real via libwebp vendorizado.
             * image-webp solo soporta lossless (archivos 5-10x más grandes para fotos).
             * Encoder::from_rgba no depende de la versión de `image`, solo bytes RGBA crudos.
             * Esto corrige el Content-Type y satisface el diagnóstico de Lighthouse
             * "Mejorar la entrega de imágenes" que antes reclamaba 2361 KiB de savings. */
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let encoder = webp::Encoder::from_rgba(rgba.as_raw(), w, h);
            let webp_bytes = encoder.encode(f32::from(quality));
            Ok((webp_bytes.to_vec(), "image/webp"))
        }
        OutputFormat::Png => {
            img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
                .map_err(|e| AppError::Internal(format!("Error codificando PNG: {e}")))?;
            Ok((buf, "image/png"))
        }
        OutputFormat::Original => {
            /* Fallback: codificar como JPEG */
            let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
            img.write_with_encoder(encoder)
                .map_err(|e| AppError::Internal(format!("Error codificando imagen: {e}")))?;
            Ok((buf, "image/jpeg"))
        }
    }
}

/* Lee una imagen del disco, la procesa según los parámetros, cachea el resultado y lo retorna.
 * Si ya existe en cache, retorna directamente desde cache. */
pub async fn get_optimized_image(
    original_path: &Path,
    params: &OptimizeParams,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let path_str = original_path
        .to_str()
        .ok_or_else(|| AppError::BadRequest("Ruta inválida".into()))?;

    let cached = cache_path(path_str, params);

    /* Intentar servir desde cache */
    if cached.exists() {
        let bytes = fs::read(&cached)
            .await
            .map_err(|e| AppError::Internal(format!("Error leyendo cache: {e}")))?;

        let content_type = match params.format {
            OutputFormat::Original => {
                mime_from_extension(cached.extension().and_then(|e| e.to_str()))
            }
            other => other.content_type(),
        };

        return Ok((bytes, content_type));
    }

    /* Leer original */
    let original_bytes = fs::read(original_path)
        .await
        .map_err(|_| AppError::NotFound("Imagen no encontrada".into()))?;

    /* Detectar MIME del original por extensión */
    let original_mime = mime_from_extension(original_path.extension().and_then(|e| e.to_str()));

    /* [135A-1] Decode/resize/encode es CPU-bound. Mantenerlo fuera de los
     * workers async evita que una pagina con muchos srcset deje el HTTP vivo
     * pero sin responder a healthchecks. */
    let permit = IMAGE_PROCESSING_PERMITS
        .get_or_init(|| Semaphore::new(IMAGE_PROCESSING_CONCURRENCY))
        .acquire()
        .await
        .map_err(|e| AppError::Internal(format!("Image processing semaphore closed: {e}")))?;
    let params_for_processing = params.clone();
    let (processed_bytes, content_type) = tokio::task::spawn_blocking(move || {
        process_image(&original_bytes, original_mime, &params_for_processing)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Image processing task failed: {e}")))??;
    drop(permit);

    /* Guardar en cache (best-effort, no bloquear si falla) */
    if let Some(parent) = cached.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    let _ = fs::write(&cached, &processed_bytes).await;

    Ok((processed_bytes, content_type))
}

#[must_use]
pub fn mime_from_extension(ext: Option<&str>) -> &'static str {
    match ext {
        Some("webp") => "image/webp",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        _ => "image/jpeg",
    }
}
