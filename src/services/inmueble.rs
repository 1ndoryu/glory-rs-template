use std::path::{Path, PathBuf};

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AddFotoRequest, CreateInmuebleRequest, FiltrosPublicos, FotoPublica, Inmueble, InmuebleRow,
    PaginatedInmuebles, UpdateInmuebleRequest, ESTADOS, EXTENSIONES_FOTO, FORMATOS_RECETA,
    MAX_FOTO_BYTES, OPERACIONES, ORIGENES_FOTO, TIPOS,
};
use crate::repositories::InmuebleRepository;

/* [159A-1] Lógica del catálogo: normalización de enums, slug único con
 * reintento ante carrera (UNIQUE 23505) y ensamblado fila+fotos sin N+1.
 * [159A-2] Subida a disco (`UPLOAD_DIR/<inmueble>/<uuid>.<ext>`) con validación
 * de magic-bytes + extensión; al borrar se limpia el archivo (best-effort). */

pub struct InmuebleService;

impl InmuebleService {
    /// Slug desde el título: minúsculas, ASCII, guiones; `inmueble` si queda vacío
    #[must_use]
    pub fn slugify(titulo: &str) -> String {
        let mut slug = String::new();
        let mut guion_pendiente = false;
        for ch in titulo.to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() {
                if guion_pendiente && !slug.is_empty() {
                    slug.push('-');
                }
                guion_pendiente = false;
                slug.push(ch);
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                guion_pendiente = true;
            }
            if slug.len() >= 60 {
                break;
            }
        }
        if slug.is_empty() {
            "inmueble".to_string()
        } else {
            slug
        }
    }

    fn normalizar(valor: &str, permitidos: &[&str], campo: &str) -> Result<String, AppError> {
        let normalizado = valor.trim().to_lowercase();
        let base = if normalizado.is_empty() {
            permitidos[0]
        } else {
            normalizado.as_str()
        };
        if permitidos.contains(&base) {
            Ok(base.to_string())
        } else {
            Err(AppError::Validation(format!(
                "Valor inválido para {campo}: {valor}"
            )))
        }
    }

    async fn con_fotos(pool: &PgPool, rows: Vec<InmuebleRow>) -> Result<Vec<Inmueble>, AppError> {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let mapa = InmuebleRepository::fotos_por_inmuebles(pool, &ids).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let fotos = mapa.get(&row.id).cloned().unwrap_or_default();
                Inmueble::from_row(row, fotos)
            })
            .collect())
    }

    fn paginacion(page: i64, per_page: i64) -> (i64, i64) {
        (page.max(1), per_page.clamp(1, 100))
    }

    pub async fn create(pool: &PgPool, req: CreateInmuebleRequest) -> Result<Inmueble, AppError> {
        let tipo = Self::normalizar(&req.tipo, TIPOS, "tipo")?;
        let operacion = Self::normalizar(&req.operacion, OPERACIONES, "operacion")?;
        let estado = Self::normalizar(&req.estado, ESTADOS, "estado")?;
        let base_slug = Self::slugify(&req.titulo);

        let mut intento = 0;
        loop {
            let slug = if intento == 0 {
                base_slug.clone()
            } else {
                format!("{base_slug}-{}", intento + 1)
            };
            let nuevo = crate::repositories::NuevoInmueble {
                titulo: &req.titulo,
                descripcion: &req.descripcion,
                ubicacion: &req.ubicacion,
                puestos: req.puestos,
                residencia: &req.residencia,
                precio: req.precio,
                tipo: &tipo,
                operacion: &operacion,
                habitaciones: req.habitaciones,
                banos: req.banos,
                metros: req.metros,
                metros_terreno: req.metros_terreno,
                estado: &estado,
                slug,
                copy_corta: req.copy.as_ref().map(|c| c.corta.as_str()),
                copy_larga: req.copy.as_ref().map(|c| c.larga.as_str()),
                copy_modelo: req.copy.as_ref().map(|c| c.modelo.as_str()),
                copy_actualizada_en: req.copy.as_ref().map(|c| c.actualizada_en),
            };
            match InmuebleRepository::create(pool, &nuevo).await {
                Ok(row) => return Ok(Inmueble::from_row(row, Vec::new())),
                Err(e) if InmuebleRepository::es_conflicto_slug(&e) && intento < 3 => {
                    intento += 1;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn get_admin(pool: &PgPool, id: Uuid) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn get_public(pool: &PgPool, slug: &str) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::find_public_by_slug(pool, slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn list_admin(
        pool: &PgPool,
        page: i64,
        per_page: i64,
    ) -> Result<PaginatedInmuebles, AppError> {
        let (page, per_page) = Self::paginacion(page, per_page);
        let (rows, total) = InmuebleRepository::list_admin(pool, page, per_page).await?;
        Ok(PaginatedInmuebles {
            items: Self::con_fotos(pool, rows).await?,
            total,
            page,
            per_page,
        })
    }

    pub async fn list_public(
        pool: &PgPool,
        mut filtros: FiltrosPublicos,
    ) -> Result<PaginatedInmuebles, AppError> {
        if let Some(tipo) = &filtros.tipo {
            filtros.tipo = Some(Self::normalizar(tipo, TIPOS, "tipo")?);
        }
        if let Some(operacion) = &filtros.operacion {
            filtros.operacion = Some(Self::normalizar(operacion, OPERACIONES, "operacion")?);
        }
        let (page, per_page) = Self::paginacion(filtros.page, filtros.per_page);
        filtros.page = page;
        filtros.per_page = per_page;
        let (rows, total) = InmuebleRepository::list_public(pool, &filtros).await?;
        Ok(PaginatedInmuebles {
            items: Self::con_fotos(pool, rows).await?,
            total,
            page,
            per_page,
        })
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        req: UpdateInmuebleRequest,
    ) -> Result<Inmueble, AppError> {
        let tipo = req
            .tipo
            .as_deref()
            .map(|v| Self::normalizar(v, TIPOS, "tipo"))
            .transpose()?;
        let operacion = req
            .operacion
            .as_deref()
            .map(|v| Self::normalizar(v, OPERACIONES, "operacion"))
            .transpose()?;
        let estado = req
            .estado
            .as_deref()
            .map(|v| Self::normalizar(v, ESTADOS, "estado"))
            .transpose()?;
        /* El formato de la receta no admite normalización con defecto (vacío
         * no es válido): allowlist directa. Los índices los cubre `range`
         * del validador en el modelo. [229A-2] */
        if let Some(receta) = &req.receta {
            if !FORMATOS_RECETA.contains(&receta.formato.as_str()) {
                return Err(AppError::Validation(format!(
                    "Valor inválido para formato: {}",
                    receta.formato
                )));
            }
        }

        let row = InmuebleRepository::update(
            pool,
            id,
            req.titulo.as_deref(),
            req.descripcion.as_deref(),
            req.ubicacion.as_deref(),
            req.puestos,
            req.residencia.as_deref(),
            req.precio,
            tipo.as_deref(),
            operacion.as_deref(),
            req.habitaciones,
            req.banos,
            req.metros,
            req.metros_terreno,
            estado.as_deref(),
            req.copy.as_ref().map(|c| c.corta.as_str()),
            req.copy.as_ref().map(|c| c.larga.as_str()),
            req.copy.as_ref().map(|c| c.modelo.as_str()),
            req.copy.as_ref().map(|c| c.actualizada_en),
            req.receta.clone().map(sqlx::types::Json),
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn set_publicado(
        pool: &PgPool,
        id: Uuid,
        publicado: bool,
    ) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::set_publicado(pool, id, publicado)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn delete(pool: &PgPool, upload_dir: &Path, id: Uuid) -> Result<(), AppError> {
        let fotos = InmuebleRepository::fotos_por_inmuebles(pool, &[id])
            .await?
            .remove(&id)
            .unwrap_or_default();
        if !InmuebleRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        /* Fila fuera (fotos en cascada); los archivos se limpian best-effort */
        for foto in fotos {
            Self::borrar_archivo(upload_dir, &foto.storage_key).await;
        }
        Ok(())
    }

    pub async fn add_foto(
        pool: &PgPool,
        inmueble_id: Uuid,
        req: AddFotoRequest,
    ) -> Result<FotoPublica, AppError> {
        if InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        let origen = req
            .origen
            .as_deref()
            .map(|v| Self::normalizar(v, ORIGENES_FOTO, "origen"))
            .transpose()?
            .unwrap_or_else(|| "original".to_string());
        Ok(FotoPublica::from(
            InmuebleRepository::add_foto(pool, inmueble_id, &req.storage_key, req.orden, &origen)
                .await?,
        ))
    }

    pub async fn delete_foto(
        pool: &PgPool,
        upload_dir: &Path,
        foto_id: Uuid,
    ) -> Result<(), AppError> {
        let foto = InmuebleRepository::find_foto(pool, foto_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Foto no encontrada".into()))?;
        InmuebleRepository::delete_foto(pool, foto_id).await?;
        /* [249A-1] Invalida la caché de fotos (`?v=<updated_at>`). */
        InmuebleRepository::tocar_inmueble(pool, foto.inmueble_id).await?;
        Self::borrar_archivo(upload_dir, &foto.storage_key).await;
        /* El thumb muere con su original (si no existe, no pasa nada):
         * formato actual más legado `thumb-` de 320 px ([249A-4]). */
        if let Some(clave_thumb) = Self::clave_miniatura(&foto.storage_key) {
            Self::borrar_archivo(upload_dir, &clave_thumb).await;
        }
        if let Some(clave_legada) = Self::clave_miniatura_legada(&foto.storage_key) {
            Self::borrar_archivo(upload_dir, &clave_legada).await;
        }
        Ok(())
    }

    /* [249A-1] Miniatura de tabla: JPEG del lado mayor, calidad 70. Puro
     * Rust (crate `image`, sin libs del sistema). Devuelve `None` si los
     * bytes no decodifican: la subida principal no debe caer por el thumb.
     * [249A-4] 320→160 px: la tabla pública muestra 64-80 px (DPR 2 cubierto
     * con 160) y el admin usa originales; PageSpeed pedía ~113 KiB menos en
     * las 8 portadas visibles. La clave cambia a `min160-` para regenerar. */
    pub(crate) fn miniatura(bytes: &[u8]) -> Option<Vec<u8>> {
        let img = image::load_from_memory(bytes).ok()?;
        let reducida = img.thumbnail(160, 160);
        let mut salida = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut salida, 70)
            .encode_image(&reducida)
            .ok()?;
        Some(salida)
    }

    /* Clave del thumb junto al original: `<inmueble>/min160-<uuid>.jpg`. */
    fn clave_miniatura(storage_key: &str) -> Option<String> {
        let (carpeta, archivo) = storage_key.split_once('/')?;
        let punto = archivo.rfind('.')?;
        if archivo.starts_with("min160-") || archivo.starts_with("thumb-") {
            return None;
        }
        Some(format!("{carpeta}/min160-{}.jpg", &archivo[..punto]))
    }

    /* Legado `thumb-<uuid>.jpg` de 320 px ([249A-1]): solo para borrar al
     * regenerar o eliminar la foto; ya no se genera ni se pide. */
    fn clave_miniatura_legada(storage_key: &str) -> Option<String> {
        let (carpeta, archivo) = storage_key.split_once('/')?;
        let punto = archivo.rfind('.')?;
        if archivo.starts_with("min160-") || archivo.starts_with("thumb-") {
            return None;
        }
        Some(format!("{carpeta}/thumb-{}.jpg", &archivo[..punto]))
    }

    /// Guarda bytes subidos en `UPLOAD_DIR/<inmueble>/<uuid>.<ext>` y registra la foto.
    /// Valida extensión + magic-bytes (jpeg/png/webp); el tope lo impone el extractor.
    pub async fn subir_foto(
        pool: &PgPool,
        upload_dir: &Path,
        inmueble_id: Uuid,
        filename: &str,
        origen: Option<&str>,
        orden: Option<i32>,
        bytes: &[u8],
    ) -> Result<FotoPublica, AppError> {
        if InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        let origen = origen
            .map(|v| Self::normalizar(v, ORIGENES_FOTO, "origen"))
            .transpose()?
            .unwrap_or_else(|| "original".to_string());

        let storage_key =
            Self::guardar_archivo(upload_dir, &inmueble_id.to_string(), filename, bytes).await?;

        match InmuebleRepository::add_foto(pool, inmueble_id, &storage_key, orden, &origen).await {
            Ok(foto) => {
                /* [249A-1] Invalida la caché de fotos (`?v=<updated_at>`). */
                InmuebleRepository::tocar_inmueble(pool, inmueble_id).await?;
                /* Thumb best-effort: si falla, la tabla usa el original. */
                if let Some(clave_thumb) = Self::clave_miniatura(&storage_key) {
                    if let Some(mini) = Self::miniatura(bytes) {
                        if let Err(e) = tokio::fs::write(upload_dir.join(&clave_thumb), mini).await
                        {
                            tracing::warn!("No se pudo guardar {clave_thumb}: {e}");
                        }
                    } else {
                        tracing::warn!("No se pudo generar miniatura de {storage_key}");
                    }
                }
                Ok(FotoPublica::from(foto))
            }
            Err(e) => {
                /* Sin fila no hay foto: se retira el archivo huérfano */
                Self::borrar_archivo(upload_dir, &storage_key).await;
                Err(e.into())
            }
        }
    }

    /* [169A-2] Núcleo reutilizable: valida y guarda bytes en
     * `UPLOAD_DIR/<carpeta>/<uuid>.<ext>`, devuelve la clave. Lo usan las
     * fotos de inmueble y las de solicitud (carpeta `solicitudes/<uuid>`). */
    pub async fn guardar_archivo(
        upload_dir: &Path,
        carpeta: &str,
        filename: &str,
        bytes: &[u8],
    ) -> Result<String, AppError> {
        if bytes.is_empty() {
            return Err(AppError::BadRequest("Archivo vacío".into()));
        }
        if bytes.len() > MAX_FOTO_BYTES {
            return Err(AppError::PayloadMuyGrande);
        }
        let extension = Self::extension_valida(filename)?;
        Self::magia_valida(bytes, extension)?;

        let dir = upload_dir.join(carpeta);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(AppError::from)?;
        let nombre = format!("{}.{}", Uuid::new_v4(), &extension[1..]);
        tokio::fs::write(dir.join(&nombre), bytes)
            .await
            .map_err(AppError::from)?;
        Ok(format!("{carpeta}/{nombre}"))
    }

    /* [299A-1 E12] Las notas de voz se archivan igual que las fotos pero
     * con su propia validación: `guardar_archivo` solo acepta imágenes y
     * todo audio caía al fallback `[media]` (la IA pedía el texto sin que el
     * staff pudiera oír nada). Misma carpeta y mismo tope de tamaño. */
    pub async fn guardar_audio(
        upload_dir: &Path,
        carpeta: &str,
        filename: &str,
        bytes: &[u8],
    ) -> Result<String, AppError> {
        if bytes.is_empty() {
            return Err(AppError::BadRequest("Archivo vacío".into()));
        }
        if bytes.len() > MAX_FOTO_BYTES {
            return Err(AppError::PayloadMuyGrande);
        }
        let extension = Self::extension_audio_valida(filename)?;
        Self::magia_audio_valida(bytes, extension)?;

        let dir = upload_dir.join(carpeta);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(AppError::from)?;
        let nombre = format!("{}.{}", Uuid::new_v4(), &extension[1..]);
        tokio::fs::write(dir.join(&nombre), bytes)
            .await
            .map_err(AppError::from)?;
        Ok(format!("{carpeta}/{nombre}"))
    }

    fn extension_audio_valida(filename: &str) -> Result<&'static str, AppError> {
        let minusculas = filename.to_lowercase();
        let punto = minusculas.rfind('.').ok_or_else(|| {
            AppError::BadRequest("El audio necesita extensión (.ogg, .mp3, .m4a)".into())
        })?;
        let extension = &minusculas[punto..];
        [".ogg", ".mp3", ".m4a"]
            .iter()
            .find(|e| ***e == *extension)
            .copied()
            .ok_or_else(|| {
                AppError::BadRequest("Extensión no permitida (solo .ogg, .mp3, .m4a)".into())
            })
    }

    /* Firmas mínimas: `OggS` (ogg), `ID3` o sync de trama `0xFF 0xE_` (mp3),
     * `....ftyp` (m4a/mp4). No es un parser completo: solo evita guardar
     * HTML de error o texto con extensión de audio. */
    fn magia_audio_valida(bytes: &[u8], extension: &str) -> Result<(), AppError> {
        let es_ogg = bytes.starts_with(b"OggS");
        let es_mp3 = bytes.starts_with(b"ID3")
            || (bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0);
        let es_m4a = bytes.len() >= 8 && bytes[4..8] == *b"ftyp";
        let valido = match extension {
            ".ogg" => es_ogg,
            ".mp3" => es_mp3,
            ".m4a" => es_m4a,
            _ => false,
        };
        if valido {
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "El contenido no coincide con un audio válido".into(),
            ))
        }
    }

    /// Borra un archivo del volumen; los fallos se registran pero no rompen la operación
    async fn borrar_archivo(upload_dir: &Path, storage_key: &str) {
        let ruta = upload_dir.join(storage_key);
        if let Err(e) = tokio::fs::remove_file(&ruta).await {
            tracing::warn!("No se pudo borrar {ruta:?}: {e}");
        }
    }

    fn extension_valida(filename: &str) -> Result<&'static str, AppError> {
        let minusculas = filename.to_lowercase();
        let punto = minusculas.rfind('.').ok_or_else(|| {
            AppError::BadRequest("El archivo necesita extensión (.jpg, .png, .webp)".into())
        })?;
        let extension = &minusculas[punto..];
        EXTENSIONES_FOTO
            .iter()
            .find(|e| **e == extension)
            .copied()
            .ok_or_else(|| {
                AppError::BadRequest("Extensión no permitida (solo .jpg, .png, .webp)".into())
            })
    }

    fn magia_valida(bytes: &[u8], extension: &str) -> Result<(), AppError> {
        let es_jpeg = bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF;
        let es_png = bytes.len() >= 8
            && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let es_webp = bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP";
        let valido = match extension {
            ".jpg" | ".jpeg" => es_jpeg,
            ".png" => es_png,
            ".webp" => es_webp,
            _ => false,
        };
        if valido {
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "El contenido no coincide con una imagen válida".into(),
            ))
        }
    }

    /// Ruta absoluta de una clave dentro del volumen (para servir archivos)
    #[must_use]
    pub fn ruta_archivo(upload_dir: &Path, storage_key: &str) -> PathBuf {
        upload_dir.join(storage_key)
    }
}

#[cfg(test)]
mod tests {
    use super::InmuebleService;

    #[test]
    fn slugify_basicos() {
        assert_eq!(InmuebleService::slugify("Piso en Centro"), "piso-en-centro");
        assert_eq!(
            InmuebleService::slugify("  Casa -- Grande__  "),
            "casa-grande"
        );
        assert_eq!(InmuebleService::slugify(""), "inmueble");
        assert_eq!(InmuebleService::slugify("---"), "inmueble");
    }

    #[test]
    fn extension_y_magia() {
        assert!(InmuebleService::extension_valida("foto.JPG").is_ok());
        assert!(InmuebleService::extension_valida("foto.webp").is_ok());
        assert!(InmuebleService::extension_valida("sin-extension").is_err());
        assert!(InmuebleService::extension_valida("doc.pdf").is_err());
        let jpeg = [0xFF, 0xD8, 0xFF, 0x00];
        assert!(InmuebleService::magia_valida(&jpeg, ".jpg").is_ok());
        assert!(InmuebleService::magia_valida(&jpeg, ".png").is_err());
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(InmuebleService::magia_valida(&png, ".png").is_ok());
        assert!(InmuebleService::magia_valida(b"RIFFxxxxWEBP", ".webp").is_ok());
        assert!(InmuebleService::magia_valida(&[], ".jpg").is_err());
    }

    /* [299A-3] Notas de voz: extensiones + firmas mínimas por formato. */
    #[test]
    fn extension_y_magia_audio() {
        assert!(InmuebleService::extension_audio_valida("nota.OGG").is_ok());
        assert!(InmuebleService::extension_audio_valida("nota.mp3").is_ok());
        assert!(InmuebleService::extension_audio_valida("nota.m4a").is_ok());
        assert!(InmuebleService::extension_audio_valida("nota.wav").is_err());
        assert!(InmuebleService::extension_audio_valida("nota.jpg").is_err());
        assert!(InmuebleService::magia_audio_valida(b"OggSxyz", ".ogg").is_ok());
        assert!(InmuebleService::magia_audio_valida(b"ID3xyz", ".mp3").is_ok());
        assert!(InmuebleService::magia_audio_valida(&[0xFF, 0xFB, 0x90], ".mp3").is_ok());
        assert!(InmuebleService::magia_audio_valida(b"xxxxftypm4a ", ".m4a").is_ok());
        assert!(InmuebleService::magia_audio_valida(b"OggSxyz", ".mp3").is_err());
        assert!(InmuebleService::magia_audio_valida(&[], ".ogg").is_err());
    }
}

/* [229A-2] La receta publicitaria persiste en `inmuebles.receta` (JSONB):
 * crear la deja NULL, `update` la fija y la relectura la trae; un formato
 * fuera del allowlist se rechaza. Humo contra la BD real de rama
 * (`DATABASE_URL`); sin ella se omite como el resto de humos. */
#[cfg(test)]
mod pruebas_receta {
    use super::*;
    use crate::models::{CreateInmuebleRequest, RecetaPublicidad, UpdateInmuebleRequest};

    fn pool_si_hay() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    fn receta_valida() -> RecetaPublicidad {
        RecetaPublicidad {
            fondo_idx: 0,
            circular_grande_idx: 1,
            circular_mediano_idx: 2,
            formato: "post-4-5".to_string(),
            con_precio: false,
            titulo1: "Casa".to_string(),
            titulo2: "en Prueba".to_string(),
        }
    }

    fn crear_humo() -> CreateInmuebleRequest {
        CreateInmuebleRequest {
            titulo: "Humo receta 229A-2".to_string(),
            descripcion: String::new(),
            ubicacion: String::new(),
            puestos: 0,
            residencia: String::new(),
            precio: 0.0,
            tipo: "apartamento".to_string(),
            operacion: "venta".to_string(),
            habitaciones: 0,
            banos: 0,
            metros: 0.0,
            metros_terreno: 0.0,
            estado: "disponible".to_string(),
            copy: None,
        }
    }

    fn solo_receta(receta: Option<RecetaPublicidad>) -> UpdateInmuebleRequest {
        UpdateInmuebleRequest {
            titulo: None,
            descripcion: None,
            ubicacion: None,
            puestos: None,
            residencia: None,
            precio: None,
            tipo: None,
            operacion: None,
            habitaciones: None,
            banos: None,
            metros: None,
            metros_terreno: None,
            estado: None,
            copy: None,
            receta,
        }
    }

    #[tokio::test]
    async fn receta_persiste_y_formato_invalido_rechaza() {
        let Some(pool) = pool_si_hay() else { return };
        let creado = InmuebleService::create(&pool, crear_humo()).await.unwrap();
        assert!(creado.receta.is_none());

        let con_receta =
            InmuebleService::update(&pool, creado.id, solo_receta(Some(receta_valida())))
                .await
                .unwrap();
        let guardada = con_receta.receta.expect("receta guardada");
        assert_eq!(guardada.formato, "post-4-5");
        assert!(!guardada.con_precio);

        let releido = InmuebleService::get_admin(&pool, creado.id).await.unwrap();
        assert_eq!(releido.receta.map(|r| r.titulo1), Some("Casa".to_string()));

        let mut mala = receta_valida();
        mala.formato = "banner-9-16".to_string();
        let err = InmuebleService::update(&pool, creado.id, solo_receta(Some(mala)))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        InmuebleService::delete(&pool, Path::new("."), creado.id)
            .await
            .unwrap();
    }
}
