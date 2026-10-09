//! [09AA-24] Normalización de `alias_titulos` en el servicio, en paralelo a
//! `inmueble_vinculo` (F7): el `create`/`update` solo llaman a estos
//! preparadores para no engordar `services/inmueble.rs` sobre el tope de
//! 500 líneas (regla `limite-lineas` de Sentinel).

use crate::errors::AppError;
use crate::models::normalizar_alias_titulos;

/// Alta: normaliza la lista entrante (recorte + dedup + topes, 422 si se pasan).
pub fn preparar_para_crear(alias: Vec<String>) -> Result<Vec<String>, AppError> {
    normalizar_alias_titulos(alias).map_err(AppError::Validation)
}

/// Edición tri-estado: ausente = no tocar; lista = normalizar y reemplazar entera.
pub fn preparar_para_update(alias: Option<Vec<String>>) -> Result<Option<Vec<String>>, AppError> {
    alias
        .map(normalizar_alias_titulos)
        .transpose()
        .map_err(AppError::Validation)
}
