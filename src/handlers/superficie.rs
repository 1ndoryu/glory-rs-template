/* [08AA-7] Superficie web pública del backend: fallback del SPA del
 * monorepo + respuestas para crawlers y agentes (`sitemap.xml`, `llms.txt`,
 * catálogo ARD). Vivía en `mod.rs` y lo empujó sobre el umbral sentinel;
 * `create_router` la consume como `superficie::*` sin cambiar rutas. */

use std::path::{Path, PathBuf};

use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;

use crate::errors::AppError;
use crate::AppState;

/* [239A-1] SPA del monorepo: sirve el fichero tal cual si existe y cae a
 * `index.html` en cualquier otra ruta (el front resuelve sus rutas). Las
 * rutas de API nunca caen aquí: devuelven 404 seco para no enmascarar
 * errores del backend con HTML. */
pub(super) async fn fallback_spa(
    axum::extract::State(est): axum::extract::State<AppState>,
    uri: Uri,
) -> impl axum::response::IntoResponse {
    use axum::response::IntoResponse as _;
    const RUTAS_API: [&str; 4] = ["/api", "/uploads", "/swagger-ui", "/api-docs"];
    let ruta = uri.path();
    if RUTAS_API
        .iter()
        .any(|p| ruta == *p || ruta.starts_with(&format!("{p}/")))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let base = est.static_dir.unwrap_or_default();
    /* [08AA-3 B3] `ruta` viene de la URI (input externo): un `..` escaparía
     * de `base` con el `join` pelado. Se canonicaliza el candidato y se
     * exige que siga dentro de la base canonicalizada; si no existe o
     * escapa, cae a `index.html` como cualquier otra ruta no fichero. */
    let base_canon: PathBuf = tokio::fs::canonicalize(&base)
        .await
        .unwrap_or_else(|_| PathBuf::from(&base));
    let candidato = Path::new(&base).join(ruta.trim_start_matches('/'));
    let dentro = tokio::fs::canonicalize(&candidato)
        .await
        .is_ok_and(|c| c.starts_with(&base_canon));
    /* `is_file` y no `exists`: `/` resuelve al propio directorio base y debe
     * caer a `index.html` en vez de intentar leer el directorio. */
    let archivo = if dentro && es_fichero(&candidato).await {
        candidato
    } else {
        Path::new(&base).join("index.html")
    };
    match tokio::fs::read(&archivo).await {
        Ok(bytes) => {
            let ext = archivo
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            /* [249A-1] Caché del SPA: los ficheros con hash de Vite
             * (`/assets/`) son inmutables un año; el HTML se revalida
             * siempre para que cada deploy tome efecto sin purgas. */
            let inmutable = ruta.starts_with("/assets/")
                || matches!(
                    ext,
                    "js" | "css" | "woff2" | "png" | "jpg" | "jpeg" | "webp" | "svg" | "ico"
                );
            let control = if inmutable {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            (
                [
                    (axum::http::header::CONTENT_TYPE, tipo_contenido(ext)),
                    (axum::http::header::CACHE_CONTROL, control),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/* [249A-1] Sitemap mínimo y honesto: el detalle es solo-modal (sin URLs con
 * slug indexables), así que declara `/` con su última modificación. Las URLs
 * `/inmueble/:slug` quedan para el bloque de ruteo futuro. */
pub(super) async fn sitemap(
    axum::extract::State(est): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    use crate::repositories::InmuebleRepository;
    let ultima = InmuebleRepository::ultima_modificacion_publica(&est.pool)
        .await
        .ok()
        .flatten()
        .map_or_else(
            || "2026-09-24".to_string(),
            |f| f.format("%Y-%m-%d").to_string(),
        );
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         <url><loc>https://mn-inmobiliaria.com/</loc><lastmod>{ultima}</lastmod></url>\n\
         </urlset>\n"
    );
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/xml"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        xml,
    )
        .into_response()
}

/* [249A-4] `llms.txt` dinámico desde publicados: mismo contenido que el
 * generador del prebuild (ya eliminado) más sección `## Enlaces` con
 * enlaces markdown, que exige la auditoría `llms-txt` de Lighthouse
 * (H1 + al menos un `[texto](url)` + 50 caracteres). */
/* Vacío legible para el `llms.txt`: el campo ausente se muestra como "—". */
fn linea(v: &str) -> &str {
    if v.is_empty() {
        "—"
    } else {
        v
    }
}

pub(super) async fn llms_txt(
    axum::extract::State(est): axum::extract::State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    use crate::models::FiltrosPublicos;
    use crate::services::InmuebleService;
    use std::fmt::Write as _;
    let filtros = FiltrosPublicos {
        tipo: None,
        operacion: None,
        precio_min: None,
        precio_max: None,
        page: 1,
        per_page: 100,
    };
    let lista = InmuebleService::list_public(&est.pool, filtros).await?;
    let mut texto = String::from(
        "# MN Inmobiliaria\n\n\
         > Compra, venta y alquiler de inmuebles en Puerto Ordaz, Venezuela.\n\
         > Publica tu inmueble o contacta por la web.\n\n\
         ## Enlaces\n\n\
         [Catálogo de inmuebles](https://mn-inmobiliaria.com/)\n\n\
         [Mapa del sitio](https://mn-inmobiliaria.com/sitemap.xml)\n",
    );
    for it in &lista.items {
        let mut specs = format!(
            "Tipo: {} · Operación: {} · Precio: {} · Ubicación: {}",
            linea(&it.tipo),
            linea(&it.operacion),
            it.precio,
            linea(&it.ubicacion),
        );
        if !it.residencia.is_empty() {
            let _ = write!(specs, " · Residencia: {}", it.residencia);
        }
        let _ = write!(
            specs,
            " · Habitaciones: {} · Baños: {}",
            it.habitaciones, it.banos
        );
        if it.puestos > 0 {
            let _ = write!(specs, " · Puestos: {}", it.puestos);
        }
        if it.metros > 0.0 {
            let _ = write!(specs, " · Construcción: {} m²", it.metros);
        }
        if it.metros_terreno > 0.0 {
            let _ = write!(specs, " · Terreno: {} m²", it.metros_terreno);
        }
        let _ = write!(
            texto,
            "\n## {}\n\n{}\n\n{}\n",
            it.titulo,
            specs,
            linea(it.descripcion.trim())
        );
    }
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        texto,
    )
        .into_response())
}

/* [249A-4] Catálogo ARD (`/.well-known/ai-catalog.json`, que Lighthouse
 * prueba aunque no se anuncie): cada publicado es una entrada con URN
 * `urn:air:mn-inmobiliaria:listings:<slug>`, exactamente un `data` (el
 * detalle es solo-modal, sin URLs por inmueble) y 2 consultas
 * representativas. Sin errores del validador oficial (warnings como
 * mucho): `specVersion` es `"1.0"` y el esquema no admite props extra. */
pub(super) async fn ai_catalog(
    axum::extract::State(est): axum::extract::State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    use crate::models::FiltrosPublicos;
    use crate::services::InmuebleService;
    let filtros = FiltrosPublicos {
        tipo: None,
        operacion: None,
        precio_min: None,
        precio_max: None,
        page: 1,
        per_page: 100,
    };
    let lista = InmuebleService::list_public(&est.pool, filtros).await?;
    let entradas: Vec<serde_json::Value> = lista
        .items
        .iter()
        .map(|it| {
            let descripcion: String = it.descripcion.chars().take(200).collect();
            serde_json::json!({
                "identifier": urn_inmueble(&it.slug, &it.id.to_string()),
                "displayName": it.titulo,
                "type": "application/json",
                "description": descripcion,
                "tags": [it.tipo, it.operacion],
                "data": {
                    "titulo": it.titulo,
                    "tipo": it.tipo,
                    "operacion": it.operacion,
                    "precio": it.precio,
                    "ubicacion": it.ubicacion,
                    "residencia": it.residencia,
                    "habitaciones": it.habitaciones,
                    "banos": it.banos,
                    "puestos": it.puestos,
                    "metros_construidos": it.metros,
                    "metros_terreno": it.metros_terreno,
                    "descripcion": it.descripcion,
                    "fotos": it.fotos.len(),
                },
                "representativeQueries": [
                    format!("{} en {} en {}", it.tipo, it.operacion, it.ubicacion),
                    it.titulo,
                ],
                "updatedAt": it.updated_at.to_rfc3339(),
            })
        })
        .collect();
    let catalogo = serde_json::json!({
        "specVersion": "1.0",
        "host": {
            "displayName": "MN Inmobiliaria",
            "logoUrl": "https://mn-inmobiliaria.com/img/logo-mn.svg",
        },
        "entries": entradas,
    });
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/json"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        axum::Json(catalogo),
    )
        .into_response())
}

/* El esquema ARD solo admite `[a-zA-Z0-9._-]` tras el publisher y sin
 * guiones en los segmentos finales: el slug (`[a-z0-9-]`) se sanea a ese
 * alfabeto para que el `identifier` pase la validación estricta. */
fn urn_inmueble(slug: &str, id: &str) -> String {
    let mut nombre: String = slug
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if nombre.is_empty() {
        nombre = format!("inmueble_{}", id.chars().take(8).collect::<String>());
    }
    format!("urn:air:mn-inmobiliaria:listings:{nombre}")
}

/// `true` si la ruta es un fichero legible (no directorio).
async fn es_fichero(p: &Path) -> bool {
    tokio::fs::metadata(p).await.is_ok_and(|m| m.is_file())
}

/// `Content-Type` mínimo para los ficheros del SPA (el `index.html` del build
/// solo referencia `.js`, `.css`, imágenes y fuentes).
fn tipo_contenido(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod pruebas {
    use super::urn_inmueble;

    /* [249A-4] El `identifier` ARD debe pasar la regex estricta del tester
     * (`urn:air:<publisher>:<segmentos sin guiones>`): el slug real trae
     * guiones y se sanea a `_` para no fallar la auditoría. */
    #[test]
    fn urn_inmueble_sanea_slug_a_alfabeto_ard() {
        let urn = urn_inmueble("townhouse-2-niveles-en-arivana", "6e6706a7");
        assert_eq!(
            urn,
            "urn:air:mn-inmobiliaria:listings:townhouse_2_niveles_en_arivana"
        );
        assert!(urn
            .strip_prefix("urn:air:mn-inmobiliaria:")
            .is_some_and(|resto| resto.chars().all(|c| c.is_ascii_alphanumeric()
                || c == '.'
                || c == '_'
                || c == ':'
                || c == '-')));
        /* El esquema ARD admite guiones en el publisher, pero no en los
         * segmentos finales (`:[a-zA-Z0-9._-]+`): `listings` y el nombre
         * saneado van sin ellos. */
        assert!(!urn
            .strip_prefix("urn:air:mn-inmobiliaria:")
            .is_some_and(|resto| resto.contains('-')));
        assert!(urn_inmueble("", "6e6706a7").ends_with("inmueble_6e6706a7"));
    }
}
