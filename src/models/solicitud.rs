use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::inmueble::EXTENSIONES_FOTO;

/* [169A-2] Solicitudes "Publicar mi inmueble": propuestas de clientes que
 * quedan en `pendiente` hasta revisión del admin.
 * Las fotos se suben antes (endpoint público) y aquí solo viajan claves
 * `solicitudes/<uuid>/<uuid>.<ext>` validadas contra el formato que el
 * servicio genera (nunca paths libres: sin `..`, sin otras carpetas). */

/* [169A-4] Trazabilidad anti-spam: `ip_origen` + `user_agent` (cabeceras
 * reales o anonimizadas tras el proxy) para auditar ráfagas y origen. */

/* [169A-6] Teléfono opcional: el formulario público no lo exige; el
 * contacto mínimo es nombre + email. `Option<String>` con validación
 * de dígitos solo cuando viene informado. */

/* [169A-7] Alta blindada: `origen_contacto` con allowlist cerrada y
 * `precio_estimado` opcional (None = sin estimar). La longitud del
 * `user_agent` se trunca a 500 caracteres. */

/* [169A-8] Contacto mínimo real: el email pasa a opcional y el teléfono
 * a obligatorio para que el admin siempre tenga una vía de contacto. */

/* [169A-9] El admin necesita los mismos filtros de la web pública
 * (q/tipo/operación/estado) más paginación y orden por fecha. */

/* [169A-11] Métricas del panel: `GET /api/admin/solicitudes/stats`
 * agregado por estado en una sola consulta. */

/// Estados del ciclo de revisión (CHECK en BD)
pub const ESTADOS_SOLICITUD: &[&str] = &["pendiente", "revisada", "aceptada", "descartada"];
/// Tope de fotos por solicitud (defensa + UX del modal)
pub const MAX_FOTOS_SOLICITUD: usize = 10;
/// Orígenes de contacto aceptados (allowlist cerrada, extensible)
pub const ORIGENES_CONTACTO: &[&str] =
    &["web", "telefono", "email", "presencial", "whatsapp", "otro"];

/// Fila de `solicitudes` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct SolicitudRow {
    pub id: Uuid,
    pub nombre: String,
    pub telefono: String,
    pub email: String,
    pub descripcion: String,
    pub ubicacion: String,
    /// Puestos de estacionamiento (>= 0).
    pub puestos: i32,
    /// Nombre de la residencia/conjunto ('' = sin especificar).
    pub residencia: String,
    pub precio_estimado: Option<f64>,
    pub operacion: String,
    pub estado: String,
    pub fotos: Vec<String>,
    pub ip_origen: Option<String>,
    pub user_agent: Option<String>,
    pub origen_contacto: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Foto de solicitud tal como la expone la API (clave + URL servible)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FotoSolicitud {
    pub storage_key: String,
    pub url: String,
}

impl FotoSolicitud {
    #[must_use]
    pub fn nueva(storage_key: String) -> Self {
        let url = format!("/uploads/{storage_key}");
        Self { storage_key, url }
    }
}

/// Solicitud con sus fotos — lo que expone la API
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Solicitud {
    pub id: Uuid,
    pub nombre: String,
    pub telefono: String,
    pub email: String,
    pub descripcion: String,
    pub ubicacion: String,
    pub puestos: i32,
    pub residencia: String,
    pub precio_estimado: Option<f64>,
    pub operacion: String,
    pub estado: String,
    pub fotos: Vec<FotoSolicitud>,
    pub ip_origen: Option<String>,
    pub user_agent: Option<String>,
    pub origen_contacto: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Solicitud {
    /// Ensambla la vista a partir de la fila (claves → clave + URL)
    #[must_use]
    pub fn from_row(row: SolicitudRow) -> Self {
        Self {
            id: row.id,
            nombre: row.nombre,
            telefono: row.telefono,
            email: row.email,
            descripcion: row.descripcion,
            ubicacion: row.ubicacion,
            puestos: row.puestos,
            residencia: row.residencia,
            precio_estimado: row.precio_estimado,
            operacion: row.operacion,
            estado: row.estado,
            fotos: row.fotos.into_iter().map(FotoSolicitud::nueva).collect(),
            ip_origen: row.ip_origen,
            user_agent: row.user_agent,
            origen_contacto: row.origen_contacto,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Respuesta de la subida pública de foto (clave para adjuntar al alta)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FotoSolicitudSubida {
    pub storage_key: String,
    pub url: String,
}

fn validar_telefono(telefono: &str) -> Result<(), ValidationError> {
    let digitos: String = telefono.chars().filter(char::is_ascii_digit).collect();
    if (6..=15).contains(&digitos.len()) {
        Ok(())
    } else {
        let mut error = ValidationError::new("telefono");
        error.message = Some("El teléfono debe tener entre 6 y 15 dígitos".into());
        Err(error)
    }
}

fn validar_operacion(operacion: &str) -> Result<(), ValidationError> {
    if super::inmueble::OPERACIONES.contains(&operacion) {
        Ok(())
    } else {
        let mut error = ValidationError::new("operacion");
        error.message = Some("La operación debe ser venta o alquiler".into());
        Err(error)
    }
}

/// Clave generada por el servicio: `solicitudes/<uuid>/<uuid>.<ext>`
fn clave_foto_valida(clave: &str) -> bool {
    if clave.contains("..") || !clave.starts_with("solicitudes/") {
        return false;
    }
    let partes: Vec<&str> = clave.split('/').collect();
    if partes.len() != 3 {
        return false;
    }
    if partes[1].parse::<Uuid>().is_err() {
        return false;
    }
    let archivo = partes[2].to_lowercase();
    let Some(punto) = archivo.rfind('.') else {
        return false;
    };
    EXTENSIONES_FOTO.contains(&&archivo[punto..])
}

/* El derive `custom` de validator 0.18 pasa `&Vec` tal cual: la firma con
 * slice dispara E0308, así que se permite `ptr_arg` solo aquí. */
#[allow(clippy::ptr_arg)]
fn validar_claves_fotos(fotos: &Vec<String>) -> Result<(), ValidationError> {
    if fotos.len() > MAX_FOTOS_SOLICITUD {
        let mut error = ValidationError::new("fotos");
        error.message = Some("Máximo 10 fotos por solicitud".into());
        return Err(error);
    }
    if fotos.iter().all(|c| clave_foto_valida(c)) {
        Ok(())
    } else {
        let mut error = ValidationError::new("fotos");
        error.message = Some("Hay claves de foto no generadas por el servidor".into());
        Err(error)
    }
}

fn validar_estado(estado: &str) -> Result<(), ValidationError> {
    if ESTADOS_SOLICITUD.contains(&estado) {
        Ok(())
    } else {
        let mut error = ValidationError::new("estado");
        error.message = Some("Estado de revisión no válido".into());
        Err(error)
    }
}

/// Alta pública de solicitud — el contacto mínimo es nombre + teléfono
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateSolicitudRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "El nombre es obligatorio (máximo 200 caracteres)"
    ))]
    pub nombre: String,
    #[validate(custom(function = "validar_telefono"))]
    pub telefono: String,
    #[serde(default)]
    #[validate(email(message = "El email no tiene un formato válido"))]
    pub email: Option<String>,
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
    #[validate(range(min = 0.0, message = "El precio estimado no puede ser negativo"))]
    pub precio_estimado: Option<f64>,
    #[serde(default = "operacion_por_defecto")]
    #[validate(custom(function = "validar_operacion"))]
    pub operacion: String,
    #[serde(default)]
    #[validate(custom(function = "validar_claves_fotos"))]
    pub fotos: Vec<String>,
    #[serde(default)]
    #[validate(length(
        max = 50,
        message = "El origen de contacto no debe exceder 50 caracteres"
    ))]
    pub origen_contacto: Option<String>,
}

fn operacion_por_defecto() -> String {
    "venta".to_string()
}

/// Cambio de estado por el admin (revisión)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateEstadoSolicitud {
    #[validate(custom(function = "validar_estado"))]
    pub estado: String,
}

/// Página de solicitudes para el panel admin
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedSolicitudes {
    pub items: Vec<Solicitud>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Query del listado admin: filtro opcional por estado + paginación
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct SolicitudesAdminParams {
    /// Filtra por estado de revisión (`pendiente`, `revisada`, …)
    pub estado: Option<String>,
    #[serde(default = "pagina_por_defecto")]
    pub page: i64,
    #[serde(default = "por_pagina_por_defecto")]
    pub per_page: i64,
}

fn pagina_por_defecto() -> i64 {
    1
}

fn por_pagina_por_defecto() -> i64 {
    20
}

/// Filtros del listado admin (misma búsqueda de la web pública + estado)
#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
pub struct FiltrosSolicitudes {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub tipo: Option<String>,
    #[serde(default)]
    pub operacion: Option<String>,
    #[serde(default)]
    pub estado: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
    /// `recientes` (defecto) o `antiguas`
    #[serde(default)]
    pub orden: Option<String>,
}

/// Una fila del agregado por estado del panel
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConteoEstado {
    pub estado: String,
    pub total: i64,
}

/// Agregado por estado + total para las tarjetas del panel
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StatsSolicitudes {
    pub por_estado: Vec<ConteoEstado>,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{clave_foto_valida, validar_operacion, validar_telefono};

    #[test]
    fn telefono_acepta_formato_habitual() {
        assert!(validar_telefono("+34 600 123 456").is_ok());
        assert!(validar_telefono("600123").is_ok());
        assert!(validar_telefono("12345").is_err());
        assert!(validar_telefono("abcdef").is_err());
    }

    #[test]
    fn operacion_solo_venta_o_alquiler() {
        assert!(validar_operacion("venta").is_ok());
        assert!(validar_operacion("alquiler").is_ok());
        assert!(validar_operacion("traspaso").is_err());
    }

    #[test]
    fn claves_foto_solo_las_del_servidor() {
        let uuid = Uuid::new_v4();
        let buena = format!("solicitudes/{uuid}/{uuid}.jpg");
        assert!(clave_foto_valida(&buena));
        assert!(!clave_foto_valida("inmuebles/otro/foto.jpg"));
        assert!(!clave_foto_valida("solicitudes/../fuga.jpg"));
        assert!(!clave_foto_valida("solicitudes/no-uuid/foto.jpg"));
        assert!(!clave_foto_valida(&format!("solicitudes/{uuid}/foto.pdf")));
    }
}
