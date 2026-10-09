use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/* [159A-1] Catálogo de inmuebles: fila plana (FromRow) + vista con fotos.
 * Sin obligatorios: crear admite payload vacío (cada columna con DEFAULT).
 * Enums como texto validado contra allowlists en el servicio.
 * [159A-2] Copy IA como columnas anulables + `FotoPublica` con URL lista. */

/// Valores permitidos para `tipo`
pub const TIPOS: &[&str] = &["apartamento", "casa", "local", "terreno", "townhouse"];
/// Valores permitidos para `operacion`
pub const OPERACIONES: &[&str] = &["venta", "alquiler"];
/// Valores permitidos para `estado`
pub const ESTADOS: &[&str] = &["disponible", "reservado", "vendido", "alquilado"];
/// Valores permitidos para `origen` de foto
pub const ORIGENES_FOTO: &[&str] = &["original", "mejorada"];
/// Valores permitidos para `formato` de receta publicitaria
pub const FORMATOS_RECETA: &[&str] = &["post-3-4", "post-4-5", "cuadrado-1-1"];

/// Receta de la imagen publicitaria: qué fotos (por ÍNDICE sobre las
/// visibles) + formato + título + precio. Vive en `inmuebles.receta` (JSONB)
/// para que lo configurado en admin llegue al frente público; `None` = usar
/// la automática (portada + 2ª/3ª, 3:4). [229A-2]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RecetaPublicidad {
    #[validate(range(min = 0, message = "fondo_idx no puede ser negativo"))]
    pub fondo_idx: i32,
    #[validate(range(min = 0, message = "circular_grande_idx no puede ser negativo"))]
    pub circular_grande_idx: i32,
    #[validate(range(min = 0, message = "circular_mediano_idx no puede ser negativo"))]
    pub circular_mediano_idx: i32,
    pub formato: String,
    #[serde(default = "default_con_precio")]
    pub con_precio: bool,
    #[validate(length(max = 120, message = "titulo1 no debe exceder 120 caracteres"))]
    #[serde(default)]
    pub titulo1: String,
    #[validate(length(max = 120, message = "titulo2 no debe exceder 120 caracteres"))]
    #[serde(default)]
    pub titulo2: String,
}

fn default_con_precio() -> bool {
    true
}
/// Extensiones de imagen aceptadas en subida (minúsculas, con punto)
pub const EXTENSIONES_FOTO: &[&str] = &[".jpg", ".jpeg", ".png", ".webp"];
/// Tope de subida por foto: 10 MiB
pub const MAX_FOTO_BYTES: usize = 10 * 1024 * 1024;

/// Copy generada por IA (nullable: `None` = pendiente de generar)
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CopyInmueble {
    #[validate(length(max = 500, message = "La copy corta no debe exceder 500 caracteres"))]
    pub corta: String,
    #[validate(length(
        max = 20000,
        message = "La copy larga no debe exceder 20000 caracteres"
    ))]
    pub larga: String,
    #[validate(length(max = 100, message = "El modelo no debe exceder 100 caracteres"))]
    pub modelo: String,
    pub actualizada_en: DateTime<Utc>,
}

/// Fila de `inmuebles` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct InmuebleRow {
    pub id: Uuid,
    pub titulo: String,
    pub descripcion: String,
    pub ubicacion: String,
    /// Puestos de estacionamiento (>= 0).
    pub puestos: i32,
    /// Nombre de la residencia/conjunto ('' = sin especificar).
    pub residencia: String,
    pub precio: f64,
    pub tipo: String,
    pub operacion: String,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: String,
    pub publicado: bool,
    pub slug: String,
    pub copy_corta: Option<String>,
    pub copy_larga: Option<String>,
    pub copy_modelo: Option<String>,
    pub copy_actualizada_en: Option<DateTime<Utc>>,
    /// Receta publicitaria elegida en admin (`None` = automática)
    pub receta: Option<sqlx::types::Json<RecetaPublicidad>>,
    /* [279A-3] Ficha /ask: respuestas de la dueña por clave (`piso`,
     * `punto_referencia`...). Objeto plano validado en `validar_extras`. */
    pub extras: sqlx::types::Json<serde_json::Value>,
    /* [279A-3] Precio mínimo privado: NUNCA se serializa a público ni a
     * tools de IA que citen cifras (solo insinuación). Solo viaja en
     * `FichaAskResponse` (rutas admin con JWT). */
    pub precio_minimo: Option<f64>,
    /* [09AA-21] Vínculo exacto con el aviso (`/marketplace/item/<id>`):
     * dígitos como TEXT (`None` = sin vincular). UNIQUE en BD. */
    pub marketplace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /* [09AA-24] Nombres alternativos (`alias_titulos TEXT[]`, vacío = sin
     * alias): el mismo inmueble publicado con otro nombre. Va al final para
     * no renumerar los `$n` del repositorio. */
    pub alias_titulos: Vec<String>,
}

/// Inmueble con sus fotos — lo que expone la API
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Inmueble {
    pub id: Uuid,
    pub titulo: String,
    pub descripcion: String,
    pub ubicacion: String,
    /// Puestos de estacionamiento (>= 0).
    pub puestos: i32,
    /// Nombre de la residencia/conjunto ('' = sin especificar).
    pub residencia: String,
    pub precio: f64,
    pub tipo: String,
    pub operacion: String,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: String,
    pub publicado: bool,
    pub slug: String,
    pub copy: Option<CopyInmueble>,
    /// Receta publicitaria elegida en admin (`None` = automática)
    pub receta: Option<RecetaPublicidad>,
    /* [279A-3] Respuestas /ask (piso, referencia...): enriquecen la ficha
     * pública y el contexto de la IA. Lo privado (`precio_minimo`) jamás
     * entra aquí: la frontera es por construcción (no existe el campo). */
    pub extras: serde_json::Value,
    /* [09AA-21] Vínculo exacto con el aviso (dígitos, `None` = sin vincular).
     * Viaja en admin y en público (es el ID del aviso, ya público en FB). */
    pub marketplace_id: Option<String>,
    /* [09AA-24] Nombres alternativos: el admin los edita y el emparejado los
     * puntúa; en público solo informan (son nombres ya publicados). */
    pub alias_titulos: Vec<String>,
    pub fotos: Vec<FotoPublica>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Inmueble {
    /// Ensambla la vista a partir de la fila y sus fotos ya ordenadas
    #[must_use]
    pub fn from_row(row: InmuebleRow, fotos: Vec<Foto>) -> Self {
        let copy = match (
            row.copy_corta,
            row.copy_larga,
            row.copy_modelo,
            row.copy_actualizada_en,
        ) {
            (Some(corta), Some(larga), Some(modelo), Some(actualizada_en)) => Some(CopyInmueble {
                corta,
                larga,
                modelo,
                actualizada_en,
            }),
            _ => None,
        };
        Self {
            id: row.id,
            titulo: row.titulo,
            descripcion: row.descripcion,
            ubicacion: row.ubicacion,
            puestos: row.puestos,
            residencia: row.residencia,
            precio: row.precio,
            tipo: row.tipo,
            operacion: row.operacion,
            habitaciones: row.habitaciones,
            banos: row.banos,
            metros: row.metros,
            metros_terreno: row.metros_terreno,
            estado: row.estado,
            publicado: row.publicado,
            slug: row.slug,
            copy,
            receta: row.receta.map(|j| j.0),
            extras: row.extras.0,
            marketplace_id: row.marketplace_id,
            alias_titulos: row.alias_titulos,
            fotos: fotos.into_iter().map(FotoPublica::from).collect(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Foto almacenada en `fotos` (`storage_key` = clave en el volumen de uploads)
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Foto {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub storage_key: String,
    pub orden: i32,
    pub origen: String,
    pub created_at: DateTime<Utc>,
}

/// Foto tal como la expone la API: con URL pública en vez de clave interna
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FotoPublica {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub url: String,
    pub orden: i32,
    pub origen: String,
    pub created_at: DateTime<Utc>,
}

impl From<Foto> for FotoPublica {
    fn from(f: Foto) -> Self {
        Self {
            id: f.id,
            inmueble_id: f.inmueble_id,
            url: format!("/uploads/{}", f.storage_key),
            orden: f.orden,
            origen: f.origen,
            created_at: f.created_at,
        }
    }
}

fn default_tipo() -> String {
    "apartamento".to_string()
}

fn default_operacion() -> String {
    "venta".to_string()
}

fn default_estado() -> String {
    "disponible".to_string()
}

/// Alta de inmueble — sin campos obligatorios
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInmuebleRequest {
    #[serde(default)]
    #[validate(length(max = 500, message = "El título no debe exceder 500 caracteres"))]
    pub titulo: String,
    #[serde(default)]
    #[validate(length(
        max = 20000,
        message = "La descripción no debe exceder 20000 caracteres"
    ))]
    pub descripcion: String,
    #[serde(default)]
    #[validate(length(max = 500, message = "La ubicación no debe exceder 500 caracteres"))]
    pub ubicacion: String,
    #[serde(default)]
    #[validate(range(min = 0, message = "Puestos no puede ser negativo"))]
    pub puestos: i32,
    #[serde(default)]
    #[validate(length(max = 500, message = "La residencia no debe exceder 500 caracteres"))]
    pub residencia: String,
    #[serde(default)]
    #[validate(range(min = 0.0, message = "El precio no puede ser negativo"))]
    pub precio: f64,
    #[serde(default = "default_tipo")]
    pub tipo: String,
    #[serde(default = "default_operacion")]
    pub operacion: String,
    #[serde(default)]
    #[validate(range(min = 0, message = "Habitaciones no puede ser negativo"))]
    pub habitaciones: i32,
    #[serde(default)]
    #[validate(range(min = 0, message = "Baños no puede ser negativo"))]
    pub banos: i32,
    #[serde(default)]
    #[validate(range(min = 0.0, message = "Los metros no pueden ser negativos"))]
    pub metros: f64,
    #[serde(default)]
    #[validate(range(min = 0.0, message = "Los metros de terreno no pueden ser negativos"))]
    pub metros_terreno: f64,
    #[serde(default = "default_estado")]
    pub estado: String,
    /* [09AA-21] Vínculo exacto (`/marketplace/item/<id>` o dígitos).
     * `None`/ausente = sin vincular; el servicio normaliza y valida
     * (dígitos 5–32, URL → dígitos). */
    #[serde(default)]
    pub marketplace_id: Option<String>,
    /* [09AA-24] Nombres alternativos (lista completa; ausente = vacía).
     * El servicio recorta, quita vacíos/duplicados y valida (máx 10,
     * c/u ≤200 caracteres). */
    #[serde(default)]
    pub alias_titulos: Vec<String>,
    /// Copy IA (`None` = sin copy); al crear, `None` deja las columnas NULL
    #[serde(default)]
    #[validate(nested)]
    pub copy: Option<CopyInmueble>,
}

/// Actualización parcial de inmueble
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInmuebleRequest {
    #[validate(length(max = 500))]
    pub titulo: Option<String>,
    #[validate(length(max = 20000))]
    pub descripcion: Option<String>,
    #[validate(length(max = 500))]
    pub ubicacion: Option<String>,
    #[validate(range(min = 0))]
    pub puestos: Option<i32>,
    #[validate(length(max = 500))]
    pub residencia: Option<String>,
    #[validate(range(min = 0.0))]
    pub precio: Option<f64>,
    pub tipo: Option<String>,
    pub operacion: Option<String>,
    #[validate(range(min = 0))]
    pub habitaciones: Option<i32>,
    #[validate(range(min = 0))]
    pub banos: Option<i32>,
    #[validate(range(min = 0.0))]
    pub metros: Option<f64>,
    #[validate(range(min = 0.0))]
    pub metros_terreno: Option<f64>,
    pub estado: Option<String>,
    /* [09AA-21] Vínculo exacto tri-estado: ausente = dejar como está,
     * `null` = desvincular (NULL), texto = fijar (URL o dígitos, el
     * servicio normaliza a dígitos 5–32). `Option<Option<..>>` distingue
     * ausente (`None`) de `null` (`Some(None)`). */
    #[serde(default)]
    pub marketplace_id: Option<Option<String>>,
    /* [09AA-24] Alias: ausente = no tocar, lista (incluso vacía) = reemplazar
     * entera (como el resto del PUT, que es reemplazo entero). */
    #[serde(default)]
    pub alias_titulos: Option<Vec<String>>,
    /// Copy IA (`Some` la fija, `None` la deja como está; no se puede borrar por PUT)
    #[validate(nested)]
    pub copy: Option<CopyInmueble>,
    /// Receta publicitaria (`Some` la fija, `None` la deja como está)
    #[validate(nested)]
    pub receta: Option<RecetaPublicidad>,
}

/* [08AA-3] B5 parametros-excesivos: el repositorio recibía 21 argumentos
 * (sentinel `parametros-excesivos-rs`, máx 8). Agrupa los 18 campos ya
 * normalizados listos para bindear: `&str` prestados, resto propios. El
 * servicio la arma desde `UpdateInmuebleRequest` + normalizados. */
#[derive(Debug)]
pub struct ActualizacionInmueble<'a> {
    pub titulo: Option<&'a str>,
    pub descripcion: Option<&'a str>,
    pub ubicacion: Option<&'a str>,
    pub puestos: Option<i32>,
    pub residencia: Option<&'a str>,
    pub precio: Option<f64>,
    pub tipo: Option<&'a str>,
    pub operacion: Option<&'a str>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub metros: Option<f64>,
    pub metros_terreno: Option<f64>,
    pub estado: Option<&'a str>,
    pub copy_corta: Option<&'a str>,
    pub copy_larga: Option<&'a str>,
    pub copy_modelo: Option<&'a str>,
    pub copy_actualizada_en: Option<DateTime<Utc>>,
    pub receta: Option<sqlx::types::Json<RecetaPublicidad>>,
    /* [09AA-21] Tri-estado listo para bindear: `None` = no tocar,
     * `Some(None)` = SET NULL (desvincular), `Some(Some(v))` = fijar. */
    pub marketplace_id: Option<Option<&'a str>>,
    /* [09AA-24] Alias ya normalizados: `None` = no tocar, `Some` = fijar la
     * lista entera (`COALESCE`, el vacío limpia). */
    pub alias_titulos: Option<Vec<String>>,
}

/// Cambio de visibilidad pública — el backend decide qué se publica
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublicacionRequest {
    pub publicado: bool,
}

/// Cambio de estado (`disponible|reservado|vendido|alquilado`) — [08AA-33]:
/// `vendido`/`alquilado` despublican en la misma query (una propiedad
/// cerrada no debe seguir en la web pública); el resto solo cambia el
/// estado sin tocar la visibilidad.
#[derive(Debug, Deserialize, ToSchema)]
pub struct EstadoRequest {
    pub estado: String,
}

/// Alta de foto en un inmueble
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AddFotoRequest {
    #[validate(length(min = 1, max = 500, message = "storage_key requerido"))]
    pub storage_key: String,
    pub orden: Option<i32>,
    pub origen: Option<String>,
}

/* [279A-3] Ficha /ask: lo que la dueña responde por inmueble. `extras` es
 * objeto plano `{clave: string|number|bool|null}`; las claves las define el
 * esquema del front por tipo (`piso`, `punto_referencia`...). `precio_minimo`
 * solo existe en estas rutas admin: ningún `Inmueble` público lo incluye. */

/// Guardar ficha /ask (admin)
#[derive(Debug, Deserialize, ToSchema)]
pub struct FichaAskRequest {
    pub extras: serde_json::Value,
    pub precio_minimo: Option<f64>,
}

/// Ficha /ask leída (admin): incluye lo privado
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FichaAskResponse {
    pub inmueble_id: Uuid,
    pub extras: serde_json::Value,
    pub precio_minimo: Option<f64>,
}

/// Valida `extras`: objeto, tope de claves y valores simples acotados.
/// Pura (sin BD): testeable sin `DATABASE_URL`.
pub fn validar_extras(v: &serde_json::Value) -> Result<(), String> {
    const MAX_CLAVES: usize = 64;
    const MAX_CLAVE: usize = 64;
    const MAX_TEXTO: usize = 500;
    let obj = v
        .as_object()
        .ok_or_else(|| "extras debe ser un objeto".to_string())?;
    if obj.len() > MAX_CLAVES {
        return Err(format!("extras admite hasta {MAX_CLAVES} campos"));
    }
    for (k, val) in obj {
        if k.len() > MAX_CLAVE || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("clave de ficha inválida: {k}"));
        }
        let ok = match val {
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
                true
            }
            serde_json::Value::String(s) => s.len() <= MAX_TEXTO,
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => false,
        };
        if !ok {
            return Err(format!("valor de ficha inválido para: {k}"));
        }
    }
    Ok(())
}

/// Alta de usuario por un owner (register público = solo bootstrap)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateUserRequest {
    #[validate(email(message = "Formato de email inválido"))]
    pub email: String,
    #[validate(length(min = 8, message = "La contraseña debe tener al menos 8 caracteres"))]
    pub password: String,
}
