/* [259A-2] Guardia anti-traversal para joins de ESCRITURA.
 * `canonicalize` no aplica al archivo objetivo (aun no existe), asi que la
 * proteccion es en dos capas reales:
 * 1. Rechazo lexico: ningun segmento puede ser vacio, `.`, `..` ni contener
 *    separadores (`/` o `\`). Con segmentos asi, el join nunca escapa de base.
 * 2. Contencion del ancestro: se sube desde el objetivo hasta el primer
 *    ancestro existente, se canonicaliza y se exige dentro de la base
 *    canonicalizada. Detecta symlinks intermedios que apunten fuera.
 * Para LECTURA (el archivo existe) usar canonicalize()+starts_with() directo,
 * como ya hacen deliverables.rs e image_proxy.rs. */

use std::path::{Path, PathBuf};

use crate::errors::AppError;

pub fn join_write_path(base: &Path, segments: &[&str]) -> Result<PathBuf, AppError> {
    for seg in segments {
        if seg.is_empty() || *seg == "." || *seg == ".." || seg.contains(['/', '\\']) {
            return Err(AppError::BadRequest("Nombre de archivo inválido".into()));
        }
    }

    let mut path = base.to_path_buf();
    for seg in segments {
        path.push(seg);
    }

    let canonical_base = base
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Directorio base no accesible: {}", e)))?;

    /* Subir hasta el primer ancestro existente (el objetivo y sus padres
     * intermedios pueden no existir aun) y exigirlo dentro de la base. */
    let mut probe: Option<&Path> = Some(&path);
    while let Some(p) = probe {
        if p.exists() {
            let canonical = p
                .canonicalize()
                .map_err(|e| AppError::Internal(format!("No se pudo resolver ruta: {}", e)))?;
            if !canonical.starts_with(&canonical_base) {
                return Err(AppError::BadRequest(
                    "Ruta fuera de directorio permitido".into(),
                ));
            }
            return Ok(path);
        }
        probe = p.parent();
    }

    /* Sin ancestro existente bajo base (disco fresco): los segmentos ya son
     * separator-free, el join es lexicamente interior a base. */
    Ok(path)
}
