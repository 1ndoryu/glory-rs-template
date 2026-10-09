//! [09AA-24-split] Slug del catálogo: módulo propio para no empujar
//! `services/inmueble.rs` sobre el tope de 500 líneas (regla `god-object-rs`
//! de Sentinel). Función pura movida tal cual desde `InmuebleService`.

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

/* [09AA-24-split] Tests del slug en su dominio (precedente 09AA-21-split:
 * los del vínculo viven en `inmueble_vinculo::pruebas_marketplace_id`). */
#[cfg(test)]
mod pruebas_slug {
    use super::*;

    #[test]
    fn slugify_basicos() {
        assert_eq!(slugify("Piso en Centro"), "piso-en-centro");
        assert_eq!(slugify("  Casa -- Grande__  "), "casa-grande");
        assert_eq!(slugify(""), "inmueble");
        assert_eq!(slugify("---"), "inmueble");
    }
}
