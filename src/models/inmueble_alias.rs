//! [09AA-24] Nombres alternativos del aviso (`alias_titulos`): un mismo
//! inmueble publicado con otro nombre (Caroní Plaza = Río Aro Plaza).
//! Módulo propio para no empujar `inmueble.rs` sobre el tope de 300 líneas
//! (regla `limite-lineas` de Sentinel).

/// Topes de alias: máx 10 nombres de máx 200 caracteres.
pub const MAX_ALIAS: usize = 10;
pub const MAX_ALIAS_LEN: usize = 200;

/// Normaliza la lista de alias: recorta, quita vacíos y duplicados (sin
/// importar caja) y valida topes. Pura (sin BD): testeable sin `DATABASE_URL`.
pub fn normalizar_alias_titulos(entradas: Vec<String>) -> Result<Vec<String>, String> {
    let mut vistos = std::collections::HashSet::new();
    let mut salida = Vec::new();
    for crudo in entradas {
        let alias = crudo.trim().to_string();
        if alias.is_empty() || !vistos.insert(alias.to_lowercase()) {
            continue;
        }
        if alias.len() > MAX_ALIAS_LEN {
            return Err(format!(
                "alias demasiado largo (máx {MAX_ALIAS_LEN}): {alias}"
            ));
        }
        salida.push(alias);
    }
    if salida.len() > MAX_ALIAS {
        return Err(format!("demasiados alias (máx {MAX_ALIAS})"));
    }
    Ok(salida)
}

/* Alias: recorte, vacíos fuera, duplicados (sin caja) fuera, topes
 * con 422 explícito (nunca truncado silencioso: la dueña perdería nombres). */
#[cfg(test)]
mod pruebas_alias {
    use super::*;

    #[test]
    fn alias_limpia_vacios_y_duplicados() {
        let aide = |v: &[&str]| {
            normalizar_alias_titulos(v.iter().map(|s| (*s).to_string()).collect()).unwrap()
        };
        assert_eq!(aide(&[]), Vec::<String>::new());
        assert_eq!(
            aide(&[
                "  Residencias Río Aro Plaza  ",
                "",
                "residencias río aro plaza"
            ]),
            vec!["Residencias Río Aro Plaza".to_string()],
            "recorta, quita vacíos y duplicados sin caja"
        );
    }

    #[test]
    fn alias_rechaza_topes() {
        let largo = "x".repeat(MAX_ALIAS_LEN + 1);
        assert!(
            normalizar_alias_titulos(vec![largo]).is_err(),
            "alias más largo que el tope es 422"
        );
        let muchos: Vec<String> = (0..=MAX_ALIAS).map(|i| format!("Alias {i}")).collect();
        assert!(
            normalizar_alias_titulos(muchos).is_err(),
            "más alias que el tope es 422"
        );
    }
}
