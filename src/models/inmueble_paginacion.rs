//! [09AA-24-split] Paginación pública de inmuebles: módulo propio para no
//! empujar `inmueble.rs` sobre el tope de 300 líneas (regla `limite-lineas`
//! de Sentinel). Se re-exporta desde `models/mod.rs`, las rutas no cambian.

use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use super::inmueble::Inmueble;

/// Filtros públicos + paginación
#[derive(Debug, Deserialize, IntoParams)]
pub struct FiltrosPublicos {
    /// Filtrar por tipo (apartamento, casa, local, terreno, townhouse)
    pub tipo: Option<String>,
    /// Filtrar por operación (venta, alquiler)
    pub operacion: Option<String>,
    /// Precio mínimo
    pub precio_min: Option<f64>,
    /// Precio máximo
    pub precio_max: Option<f64>,
    /// Página (empezando en 1)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Resultados por página
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// Response paginada de inmuebles
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaginatedInmuebles {
    pub items: Vec<Inmueble>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
