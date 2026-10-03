/* [01AA-4-f3m] Resolucion de SEO meta estatico/dinamico (extraido de prerender.rs).
 * resolve_seo_meta es pub(crate): lo usa middleware.rs. Waivers
 * sentinel-disable sqlx-query-as-sin-macro preservados verbatim. */

use sqlx::PgPool;
use std::time::Instant;

use crate::repositories::SeoSettingsRepository;

use super::helpers::{SeoMeta, json_escape};
use super::inject::{dynamic_blog_json_ld, dynamic_service_json_ld, static_json_ld};
use super::state::{CachedSeoEntry, SeoCache};

/* [277A-13] Rutas estáticas conocidas por el prerender. */
const KNOWN_STATIC: &[&str] = &[
    "/",
    "/servicios",
    "/proyectos",
    "/nosotros",
    "/soluciones/hosting",
    "/soluciones/hosting-wordpress",
    "/soluciones/vps",
    "/blog",
    "/contacto",
    "/politica-privacidad",
];

/* Construye SeoMeta: estáticas con cache+DB fallback, dinámicas consultan BD. */
pub(crate) async fn resolve_seo_meta(
    path: &str,
    pool: &PgPool,
    app_url: &str,
    cache: &SeoCache,
) -> Option<SeoMeta> {
    let canonical = format!("{app_url}{path}");

    if KNOWN_STATIC.contains(&path) {
        return resolve_static_with_cache(path, pool, &canonical, app_url, cache).await;
    }

    resolve_dynamic_meta(path, pool, &canonical, app_url).await
}

/* [277A-13] Resuelve SEO meta para páginas estáticas con cache.
 * 1. Check cache → si hit y no expirado, retornar.
 * 2. Consultar tabla seo_settings.
 * 3. Si hay datos en DB, usarlos. Si no, fallback hardcoded.
 * 4. Guardar en cache. */
async fn resolve_static_with_cache(
    path: &str,
    pool: &PgPool,
    canonical: &str,
    app_url: &str,
    cache: &SeoCache,
) -> Option<SeoMeta> {
    /* 1. Check cache */
    {
        let entries = cache.entries.read().await;
        if let Some(cached) = entries.get(path) {
            if cached.fetched_at.elapsed() < cache.ttl {
                return Some(SeoMeta {
                    title: cached.meta.title.clone(),
                    description: cached.meta.description.clone(),
                    og_image: cached.meta.og_image.clone(),
                    canonical: canonical.to_string(),
                    og_type: cached.meta.og_type,
                    json_ld: cached.meta.json_ld.clone(),
                });
            }
        }
    }

    /* 2. Consultar DB */
    let db_setting = SeoSettingsRepository::find_by_path(pool, path)
        .await
        .ok()
        .flatten();

    /* 3. Construir SeoMeta con JSON-LD */
    let json_ld = static_json_ld(path, app_url);
    let meta = if let Some(ref setting) = db_setting {
        SeoMeta {
            title: setting.title.clone(),
            description: setting.description.clone(),
            og_image: setting.og_image_url.clone(),
            canonical: canonical.to_string(),
            og_type: "website",
            json_ld,
        }
    } else {
        let mut fb = hardcoded_fallback(path, canonical)?;
        fb.json_ld = json_ld;
        fb
    };

    /* 4. Guardar en cache */
    {
        let mut entries = cache.entries.write().await;
        entries.insert(
            path.to_string(),
            CachedSeoEntry {
                meta: SeoMeta {
                    title: meta.title.clone(),
                    description: meta.description.clone(),
                    og_image: meta.og_image.clone(),
                    canonical: meta.canonical.clone(),
                    og_type: meta.og_type,
                    json_ld: meta.json_ld.clone(),
                },
                fetched_at: Instant::now(),
            },
        );
    }

    Some(meta)
}

/* Fallback hardcoded para cuando la migración seo_settings no se ha aplicado */
fn hardcoded_fallback(path: &str, canonical: &str) -> Option<SeoMeta> {
    let (title, description) = match path {
        "/" => ("Nakomi Studio — Agencia Creativa Digital", "Estudio creativo basado en Copenhague. Diseño web, apps e IA construidos con Rust para rendimiento real."),
        "/servicios" => ("Nuestros Servicios — Nakomi Studio", "Servicios de desarrollo web, diseño UI/UX, branding y soluciones digitales a medida."),
        "/proyectos" => ("Nuestros Proyectos y Casos de Éxito — Nakomi Studio", "Explora nuestros proyectos y casos de éxito en desarrollo web, diseño digital y soluciones de software."),
        "/nosotros" => ("Sobre Nosotros — Nakomi Studio", "Conoce al equipo de Nakomi Studio: diseñadores y desarrolladores basados en Copenhague, especializados en web, apps e IA."),
        "/soluciones/hosting" => ("Hosting Administrado — Nakomi Studio", "Hosting web administrado con SSL, backups automáticos y soporte técnico."),
        "/soluciones/hosting-wordpress" => ("Hosting WordPress — Nakomi Studio", "WordPress hosting optimizado con WP-CLI, backups automáticos y soporte experto."),
        "/soluciones/vps" => ("Servidores VPS — Nakomi Studio", "Servidores VPS dedicados con acceso root, bootstrap inicial y precios transparentes."),
        "/blog" => ("Blog — Nakomi Studio", "Artículos sobre desarrollo web, diseño, tecnología e inteligencia artificial."),
        "/contacto" => ("Contacto — Nakomi Studio", "Contacta con Nakomi Studio para tu proyecto web, app o solución digital."),
        "/politica-privacidad" => ("Política de Privacidad — Nakomi Studio", "Política de privacidad y protección de datos de Nakomi Studio."),
        _ => return None,
    };
    Some(SeoMeta {
        title: title.into(),
        description: description.into(),
        og_image: None,
        canonical: canonical.to_string(),
        og_type: "website",
        json_ld: None,
    })
}

/* Rutas dinámicas: /servicios/:slug y /proyectos/:slug consultan la BD */
async fn resolve_dynamic_meta(
    path: &str,
    pool: &PgPool,
    canonical: &str,
    app_url: &str,
) -> Option<SeoMeta> {
    if let Some(slug) = path.strip_prefix("/servicios/") {
        let slug = slug.trim_end_matches('/');
        if slug.is_empty() {
            return None;
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro
        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT title, description FROM services WHERE slug = $1 AND is_active = true",
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
        .ok()??;

        let json_ld = dynamic_service_json_ld(
            &json_escape(&row.0),
            &json_escape(row.1.as_deref().unwrap_or("")),
            slug,
            app_url,
        );
        Some(SeoMeta {
            title: format!("{} — Nakomi Studio", row.0),
            description: row.1.unwrap_or_default(),
            og_image: None,
            canonical: canonical.to_string(),
            og_type: "website",
            json_ld: Some(json_ld),
        })
    } else if let Some(slug) = path.strip_prefix("/proyectos/") {
        let slug = slug.trim_end_matches('/');
        if slug.is_empty() {
            return None;
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro
        let row: (String, String, Option<String>) = sqlx::query_as(
            "SELECT COALESCE(meta_title, title), \
                    COALESCE(meta_description, description), \
                    featured_image \
             FROM projects WHERE slug = $1 AND status = 'published'",
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
        .ok()??;

        Some(SeoMeta {
            title: format!("{} — Nakomi Studio", row.0),
            description: row.1,
            og_image: row.2,
            canonical: canonical.to_string(),
            og_type: "article",
            json_ld: None,
        })
    } else if let Some(slug) = path.strip_prefix("/blog/") {
        let slug = slug.trim_end_matches('/');
        if slug.is_empty() {
            return None;
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro
        let row: (String, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT COALESCE(meta_title, title), \
                    COALESCE(meta_description, excerpt), \
                    image_url \
             FROM blog_posts WHERE slug = $1 AND status = 'published'",
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
        .ok()??;

        let json_ld = dynamic_blog_json_ld(
            &json_escape(&row.0),
            &json_escape(row.1.as_deref().unwrap_or("")),
            slug,
            app_url,
        );
        Some(SeoMeta {
            title: format!("{} — Nakomi Studio", row.0),
            description: row.1.unwrap_or_default(),
            og_image: row.2,
            canonical: canonical.to_string(),
            og_type: "article",
            json_ld: Some(json_ld),
        })
    } else {
        None
    }
}
