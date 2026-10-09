/* [09AA-21] Vínculo exacto inmueble ↔ aviso Marketplace en su dominio.
 * Extraído de `inmueble.rs` (gate >500 efectivas): normalizador puro,
 * preparación para create/update (normaliza + 422 si otra ficha lo reclama),
 * préstamo tri-estado para `ActualizacionInmueble` y mapeo del conflicto
 * UNIQUE a 422. `inmueble.rs` conserva solo las llamadas + un delegador
 * `normalizar_marketplace_id` para no mover `handlers/marketplace.rs`.
 * Sin `unwrap()` sobre input externo: cualquier error se propaga con `?` o se
 * convierte en `AppError::Validation`. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::repositories::InmuebleRepository;

/* [09AA-21] Normaliza el vínculo exacto con el aviso (`/marketplace/item/<id>`
 * o dígitos puros) al ID canónico. Espeja `normalizarMarketplaceId` del front
 * (`frontend/src/domain/inmueble.ts`): URL → dígitos de la ruta, dígitos
 * 5–32 tal cual, vacío/`None` → `None` (sin vincular). Cualquier otra forma
 * es 422. Pura (sin BD): testeable sin `DATABASE_URL`. */
pub fn normalizar_marketplace_id(valor: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(crudo) = valor else { return Ok(None) };
    let recortado = crudo.trim();
    if recortado.is_empty() {
        return Ok(None);
    }
    if let Some(pos) = recortado.find("marketplace/item/") {
        let resto = &recortado[pos + "marketplace/item/".len()..];
        let digitos: String = resto.chars().take_while(char::is_ascii_digit).collect();
        if (5..=32).contains(&digitos.len()) {
            return Ok(Some(digitos));
        }
        return Err(AppError::Validation(format!(
            "Valor inválido para marketplace_id: {crudo}"
        )));
    }
    let todo_digitos = !recortado.is_empty() && recortado.chars().all(|c| c.is_ascii_digit());
    if todo_digitos && (5..=32).contains(&recortado.len()) {
        return Ok(Some(recortado.to_string()));
    }
    Err(AppError::Validation(format!(
        "Valor inválido para marketplace_id: {crudo}"
    )))
}

/// 422 canónico cuando el aviso ya lo reclama otra ficha.
#[must_use]
pub fn error_duplicado() -> AppError {
    AppError::Validation("marketplace_id ya vinculado a otro inmueble".into())
}

/// El error de BD es el UNIQUE de `marketplace_id` (la red de carrera).
#[must_use]
pub fn es_conflicto(e: &sqlx::Error) -> bool {
    InmuebleRepository::es_conflicto_marketplace(e)
}

/* Vínculo normalizado para `create` + 422 previo si otra ficha ya lo reclama
 * (el UNIQUE queda como red de carrera). */
pub async fn preparar_para_crear(
    pool: &PgPool,
    valor: Option<&str>,
) -> Result<Option<String>, AppError> {
    let normalizado = normalizar_marketplace_id(valor)?;
    if let Some(ref aviso) = normalizado {
        if InmuebleRepository::find_by_marketplace_id(pool, aviso)
            .await?
            .is_some()
        {
            return Err(error_duplicado());
        }
    }
    Ok(normalizado)
}

/* Tri-estado del PUT (ausente = no tocar, `null` = desvincular, texto = fijar
 * normalizado) + 422 si el aviso lo reclama otra ficha (`id` distinto).
 * El UNIQUE queda como red de carrera. */
#[allow(clippy::option_option)] // Tri-estado del PUT: ausente/null/texto. Justificado.
pub async fn preparar_para_update(
    pool: &PgPool,
    id: Uuid,
    valor: Option<&Option<String>>,
) -> Result<Option<Option<String>>, AppError> {
    let resuelto: Option<Option<String>> = match valor {
        None => None,
        Some(None) => Some(None),
        Some(Some(crudo)) => Some(normalizar_marketplace_id(Some(crudo.as_str()))?),
    };
    if let Some(Some(ref aviso)) = resuelto {
        if let Some(otra) = InmuebleRepository::find_by_marketplace_id(pool, aviso).await? {
            if otra.id != id {
                return Err(error_duplicado());
            }
        }
    }
    Ok(resuelto)
}

/* Presta el vínculo ya normalizado para `ActualizacionInmueble`:
 * `None` = no tocar, `Some(None)` = SET NULL, `Some(Some(v))` = fijar. */
#[must_use]
#[allow(clippy::option_option)] // Tri-estado del PUT: ausente/null/texto. Justificado.
pub fn prestar_para_update(valor: Option<&Option<String>>) -> Option<Option<&str>> {
    match valor {
        None => None,
        Some(None) => Some(None),
        Some(Some(v)) => Some(Some(v.as_str())),
    }
}

/* [09AA-21] Vínculo exacto: normalizador puro (dígitos/URL/vacío/inválido) +
 * humo contra la BD real de rama (`DATABASE_URL`): crear con ID, duplicado
 * 422, PUT que fija y que desvincula con `null`, y `find_by_marketplace_id`.
 * Sin BD se omite como el resto de humos. */
#[cfg(test)]
mod pruebas_marketplace_id {
    use super::*;
    use crate::models::{CreateInmuebleRequest, UpdateInmuebleRequest};
    use crate::services::InmuebleService;

    #[test]
    fn normalizar_acepta_digitos_url_y_vacio() {
        assert_eq!(normalizar_marketplace_id(None).unwrap(), None);
        assert_eq!(normalizar_marketplace_id(Some("")).unwrap(), None);
        assert_eq!(normalizar_marketplace_id(Some("   ")).unwrap(), None);
        assert_eq!(
            normalizar_marketplace_id(Some("1234567890")).unwrap(),
            Some("1234567890".to_string())
        );
        assert_eq!(
            normalizar_marketplace_id(Some("https://www.facebook.com/marketplace/item/987654321/"))
                .unwrap(),
            Some("987654321".to_string())
        );
    }

    #[test]
    fn normalizar_rechaza_formas_invalidas() {
        for malo in [
            "abc",
            "1234",
            "12ab34",
            "marketplace/item/",
            "marketplace/item/12ab",
        ] {
            assert!(
                normalizar_marketplace_id(Some(malo)).is_err(),
                "{malo} debe ser 422"
            );
        }
    }

    fn pool_si_hay() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    fn crear_humo(titulo: &str, marketplace_id: Option<&str>) -> CreateInmuebleRequest {
        CreateInmuebleRequest {
            titulo: titulo.to_string(),
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
            marketplace_id: marketplace_id.map(str::to_string),
            alias_titulos: Vec::new(),
            copy: None,
        }
    }

    /* Tri-estado deliberado del PUT (ausente = no tocar, null = desvincular,
     * texto = fijar): el anidado es el contrato, no un descuido. */
    #[allow(clippy::option_option)]
    fn solo_marketplace(marketplace_id: Option<Option<&str>>) -> UpdateInmuebleRequest {
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
            marketplace_id: marketplace_id.map(|interior| interior.map(str::to_string)),
            alias_titulos: None,
            copy: None,
            receta: None,
        }
    }

    #[tokio::test]
    async fn vinculo_exacta_duplicado_fijar_y_desvincular() {
        let Some(pool) = pool_si_hay() else { return };
        let aviso = "123456789012345";
        let creado = InmuebleService::create(&pool, crear_humo("Humo mp-id 09AA-21", Some(aviso)))
            .await
            .unwrap();
        assert_eq!(creado.marketplace_id.as_deref(), Some(aviso));

        let hallado = InmuebleRepository::find_by_marketplace_id(&pool, aviso)
            .await
            .unwrap()
            .expect("el aviso recién vinculado se halla por ID");
        assert_eq!(hallado.id, creado.id);

        let inexistente = InmuebleRepository::find_by_marketplace_id(&pool, "999999999999999")
            .await
            .unwrap();
        assert!(inexistente.is_none(), "ID inexistente no empareja");

        let duplicado = InmuebleService::create(
            &pool,
            crear_humo("Humo mp-id duplicado 09AA-21", Some(aviso)),
        )
        .await;
        assert!(
            matches!(duplicado, Err(AppError::Validation(_))),
            "dos fichas no reclaman el mismo aviso"
        );

        let otro = InmuebleService::create(&pool, crear_humo("Humo mp-id libre 09AA-21", None))
            .await
            .unwrap();
        assert_eq!(otro.marketplace_id, None, "sin ID queda sin vincular");

        let fijado =
            InmuebleService::update(&pool, otro.id, solo_marketplace(Some(Some(aviso)))).await;
        assert!(
            matches!(fijado, Err(AppError::Validation(_))),
            "fijar un aviso ya reclamado es 422"
        );

        let desvinculado = InmuebleService::update(&pool, creado.id, solo_marketplace(Some(None)))
            .await
            .unwrap();
        assert_eq!(desvinculado.marketplace_id, None, "`null` desvincula");
        let ya_no = InmuebleRepository::find_by_marketplace_id(&pool, aviso)
            .await
            .unwrap();
        assert!(ya_no.is_none(), "tras desvincular el aviso queda libre");

        InmuebleService::delete(&pool, std::path::Path::new("."), creado.id)
            .await
            .unwrap();
        InmuebleService::delete(&pool, std::path::Path::new("."), otro.id)
            .await
            .unwrap();
    }
}
