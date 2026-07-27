/* [214A-2] Middleware SEO dinámico para crawlers.
 * Reemplaza el enfoque estático de 114A-SEO3 (HTML pre-renderizado con Puppeteer)
 * que no soportaba contenido CMS editable.
 *
 * Nuevo enfoque: lee index.html del SPA y le inyecta meta tags dinámicos
 * (title, description, OG, canonical, twitter) usando datos de la BD.
 * Rutas estáticas (/servicios, /nosotros, etc.) usan meta fijos.
 * Rutas dinámicas (/servicios/:slug, /proyectos/:slug) consultan la BD.
 * Usuarios normales reciben el SPA sin cambios. */

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use std::fmt::Write;

use crate::models::Project;
use crate::repositories::{ProjectRepository, SeoSettingsRepository};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/* [277A-13] Cache entry para SEO settings de páginas estáticas.
 * Evita consultar la DB en cada request de crawler. TTL: 5 minutos. */
struct CachedSeoEntry {
    meta: SeoMeta,
    fetched_at: Instant,
}

#[derive(Clone)]
pub struct SeoCache {
    entries: Arc<RwLock<HashMap<String, CachedSeoEntry>>>,
    ttl: Duration,
}

impl SeoCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(300),
        }
    }
}

/// Estado necesario para inyectar meta SEO dinámico
#[derive(Clone)]
pub struct PrerenderState {
    pub pool: PgPool,
    pub static_dir: String,
    pub app_url: String,
    pub seo_cache: SeoCache,
}

const CRAWLER_AGENTS: &[&str] = &[
    "googlebot",
    "bingbot",
    "yandexbot",
    "baiduspider",
    "duckduckbot",
    "slurp",
    "facebot",
    "facebookexternalhit",
    "twitterbot",
    "linkedinbot",
    "applebot",
    "semrushbot",
    "ahrefsbot",
    "quora link preview",
    "outbrain",
    "pinterestbot",
];

fn is_crawler(user_agent: &str) -> bool {
    let ua = user_agent.to_ascii_lowercase();
    CRAWLER_AGENTS.iter().any(|bot| ua.contains(bot))
}

/* Metadatos SEO por ruta */
struct SeoMeta {
    title: String,
    description: String,
    og_image: Option<String>,
    canonical: String,
    og_type: &'static str,
    json_ld: Option<String>,
}

impl SeoMeta {
    /* Genera tags OG + Twitter que se inyectan antes de </head> */
    fn og_tags(&self, app_url: &str) -> String {
        let title = html_escape(&self.title);
        let desc = html_escape(&self.description);
        let mut tags = format!(
            "<link rel=\"canonical\" href=\"{canonical}\">\n\
             <meta property=\"og:title\" content=\"{title}\">\n\
             <meta property=\"og:description\" content=\"{desc}\">\n\
             <meta property=\"og:url\" content=\"{canonical}\">\n\
             <meta property=\"og:type\" content=\"{og_type}\">\n\
             <meta property=\"og:site_name\" content=\"Nakomi Studio\">\n\
             <meta name=\"twitter:card\" content=\"summary_large_image\">\n\
             <meta name=\"twitter:title\" content=\"{title}\">\n\
             <meta name=\"twitter:description\" content=\"{desc}\">\n",
            canonical = self.canonical,
            og_type = self.og_type,
        );
        if let Some(ref img) = self.og_image {
            /* Usar el proxy de imágenes para servir OG image optimizada */
            let img_url = format!("{app_url}/api/img/{img}?w=1200&q=80&fmt=webp");
            let _ = write!(
                tags,
                "<meta property=\"og:image\" content=\"{img_url}\">\n\
                 <meta name=\"twitter:image\" content=\"{img_url}\">\n",
            );
        }
        tags
    }
}

/* [175A-1] Construye la URL del proxy de imágenes para una ruta local.
 * Refleja la lógica de imageUtils.ts: strip /uploads/, encode por segmento. */
fn build_img_proxy_url(img_path: &str, width: u32, quality: u32) -> String {
    let relative = if img_path.starts_with("/uploads/") {
        img_path.trim_start_matches("/uploads/")
    } else {
        img_path.trim_start_matches('/')
    };
    let encoded: String = relative
        .split('/')
        .map(|seg| urlencoding::encode(seg).into_owned())
        .collect::<Vec<_>>()
        .join("/");
    format!("/api/img/{encoded}?w={width}&q={quality}&fmt=webp")
}

/* [175A-2] Genera el tag <link rel="preload"> responsivo para la imagen hero.
 * Widths sincronizados con ALLOWED_WIDTHS de imageUtils.ts para que el browser
 * pueda hacer cache-hit al crear el elemento <picture> con el mismo srcset.
 * La imagen empieza a descargarse al parsear el HTML, antes de que React monte. */
fn build_hero_preload_from_path(img_path: &str) -> String {
    let sizes = "(max-width: 768px) calc(100vw - 32px), min(100vw - 48px, 1200px)";
    /* Mismos buckets que ALLOWED_WIDTHS en frontend/src/utils/imageUtils.ts */
    let widths: &[u32] = &[150, 300, 480, 640, 800, 1024, 1200, 1600, 2400];
    let srcset: String = widths
        .iter()
        .map(|&w| format!("{} {}w", build_img_proxy_url(img_path, w, 72), w))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "<link rel=\"preload\" as=\"image\" type=\"image/webp\" imagesrcset=\"{srcset}\" imagesizes=\"{sizes}\" fetchpriority=\"high\">\n"
    )
}

/* [175A-2] Serializa proyectos publicados como script de datos iniciales.
 * El frontend lee window.__INITIAL_DATA__.projects para pre-poblar React Query
 * sin esperar el round-trip API, eliminando ~300-500ms del LCP en 4G slow.
 * XSS mitigation: escapamos </ para evitar inyección de cierre de script tag.
 * Toma ownership del Vec para no requerir Clone en Project. */
fn build_initial_data_script(projects: Vec<Project>) -> Option<String> {
    let responses: Vec<_> = projects.into_iter().map(Project::into_response).collect();
    let json = serde_json::to_string(&responses).ok()?;
    /* Escapar </script> para prevenir XSS si algún campo contiene ese literal */
    let safe_json = json.replace("</", "<\\/");
    Some(format!(
        "<script>window.__INITIAL_DATA__={{\"projects\":{safe_json}}}</script>\n"
    ))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/* Escapa caracteres especiales para interpolación segura en strings JSON */
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/* [277A-13] Rutas estáticas conocidas por el prerender. */
const KNOWN_STATIC: &[&str] = &[
    "/", "/servicios", "/proyectos", "/nosotros",
    "/soluciones/hosting", "/soluciones/hosting-wordpress", "/soluciones/vps",
    "/blog", "/contacto", "/politica-privacidad",
];

/* Construye SeoMeta: estáticas con cache+DB fallback, dinámicas consultan BD. */
async fn resolve_seo_meta(path: &str, pool: &PgPool, app_url: &str, cache: &SeoCache) -> Option<SeoMeta> {
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
    let db_setting = SeoSettingsRepository::find_by_path(pool, path).await.ok().flatten();

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
        entries.insert(path.to_string(), CachedSeoEntry {
            meta: SeoMeta {
                title: meta.title.clone(),
                description: meta.description.clone(),
                og_image: meta.og_image.clone(),
                canonical: meta.canonical.clone(),
                og_type: meta.og_type,
                json_ld: meta.json_ld.clone(),
            },
            fetched_at: Instant::now(),
        });
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
async fn resolve_dynamic_meta(path: &str, pool: &PgPool, canonical: &str, app_url: &str) -> Option<SeoMeta> {
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

        let json_ld = dynamic_service_json_ld(&json_escape(&row.0), &json_escape(row.1.as_deref().unwrap_or("")), slug, app_url);
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

        let json_ld = dynamic_blog_json_ld(&json_escape(&row.0), &json_escape(row.1.as_deref().unwrap_or("")), slug, app_url);
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

/* Inyecta meta SEO en el HTML del SPA: reemplaza <title> y description
 * existentes, y agrega OG/canonical antes de </head>. */
fn inject_seo_into_html(html: &str, meta: &SeoMeta, app_url: &str) -> String {
    let mut result = html.to_string();

    /* Reemplazar contenido de <title>...</title> */
    if let Some(start) = result.find("<title>") {
        let title_start = start + "<title>".len();
        if let Some(end_rel) = result[title_start..].find("</title>") {
            result.replace_range(
                title_start..title_start + end_rel,
                &html_escape(&meta.title),
            );
        }
    }

    /* Reemplazar contenido de <meta name="description" content="..."> */
    let desc_prefix = "<meta name=\"description\" content=\"";
    if let Some(start) = result.find(desc_prefix) {
        let content_start = start + desc_prefix.len();
        if let Some(end_rel) = result[content_start..].find('"') {
            result.replace_range(
                content_start..content_start + end_rel,
                &html_escape(&meta.description),
            );
        }
    }

    /* Agregar OG tags + canonical antes de </head> */
    let og = meta.og_tags(app_url);
    result = result.replace("</head>", &format!("{og}</head>"));

    /* [277A-14] Inyectar JSON-LD structured data si existe */
    if let Some(ref json_ld) = meta.json_ld {
        let script = format!(
            "<script type=\"application/ld+json\">{json_ld}</script>\n"
        );
        result = result.replace("</head>", &format!("{script}</head>"));
    }

    result
}

/* [277A-14] Genera JSON-LD para rutas estáticas conocidas.
 * organization+website para home, organization para catálogos,
 * breadcrumb para detalle. */
fn static_json_ld(path: &str, app_url: &str) -> Option<String> {
    let site = app_url;
    let org = format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":[\"ProfessionalService\",\"LocalBusiness\"],\"name\":\"Nakomi Studio\",\"url\":\"{site}\",\"description\":\"Estudio creativo especializado en desarrollo web, aplicaciones, agentes de IA e identidad de marca.\",\"address\":{{\"@type\":\"PostalAddress\",\"addressLocality\":\"Copenhagen\",\"addressCountry\":\"DK\"}},\"sameAs\":[\"https://github.com/1ndoryu\",\"https://www.linkedin.com/company/nakomi-studio\"]}}"
    );
    match path {
        "/" => {
            let web = format!(
                "{{\"@context\":\"https://schema.org\",\"@type\":\"WebSite\",\"name\":\"Nakomi Studio\",\"url\":\"{site}\"}}"
            );
            Some(format!("{{\"@context\":\"https://schema.org\",\"@graph\":[{org},{web}]}}"))
        }
        "/servicios" | "/proyectos" => Some(org),
        "/nosotros" => {
            let breadcrumb = format!(
                "{{\"@context\":\"https://schema.org\",\"@type\":\"BreadcrumbList\",\"itemListElement\":[{{\"@type\":\"ListItem\",\"position\":1,\"name\":\"Inicio\",\"item\":\"{site}\"}},{{\"@type\":\"ListItem\",\"position\":2,\"name\":\"Nosotros\",\"item\":\"{site}/nosotros\"}}]}}"
            );
            Some(format!("{{\"@context\":\"https://schema.org\",\"@graph\":[{breadcrumb}]}}"))
        }
        _ => None,
    }
}

/* [277A-14] Genera JSON-LD dinámico para servicios desde DB */
fn dynamic_service_json_ld(title: &str, desc: &str, slug: &str, app_url: &str) -> String {
    let breadcrumb = format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":\"BreadcrumbList\",\"itemListElement\":[{{\"@type\":\"ListItem\",\"position\":1,\"name\":\"Inicio\",\"item\":\"{app_url}\"}},{{\"@type\":\"ListItem\",\"position\":2,\"name\":\"Servicios\",\"item\":\"{app_url}/servicios\"}},{{\"@type\":\"ListItem\",\"position\":3,\"name\":\"{title}\",\"item\":\"{app_url}/servicios/{slug}\"}}]}}"
    );
    let service = format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":\"Service\",\"name\":\"{title}\",\"description\":\"{desc}\",\"url\":\"{app_url}/servicios/{slug}\",\"provider\":{{\"@type\":\"ProfessionalService\",\"name\":\"Nakomi Studio\",\"url\":\"{app_url}\"}}}}"
    );
    format!("{{\"@context\":\"https://schema.org\",\"@graph\":[{breadcrumb},{service}]}}")
}

/* [277A-14] Genera JSON-LD para blog posts desde DB */
fn dynamic_blog_json_ld(title: &str, desc: &str, slug: &str, app_url: &str) -> String {
    format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":\"BlogPosting\",\"headline\":\"{title}\",\"description\":\"{desc}\",\"url\":\"{app_url}/blog/{slug}\",\"author\":{{\"@type\":\"Organization\",\"name\":\"Nakomi Studio\",\"url\":\"{app_url}\"}},\"publisher\":{{\"@type\":\"Organization\",\"name\":\"Nakomi Studio\",\"url\":\"{app_url}\"}}}}"
    )
}

/// Middleware SEO: para crawlers, inyecta meta tags dinámicos en index.html.
/// Rutas API, assets, uploads, WS, swagger y panel se ignoran.
pub async fn prerender(
    State(state): State<PrerenderState>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() != axum::http::Method::GET {
        return next.run(request).await;
    }

    let path = request.uri().path();

    /* Rutas que nunca reciben meta SEO */
    if path.starts_with("/api/")
        || path.starts_with("/assets/")
        || path.starts_with("/uploads/")
        || path.starts_with("/ws/")
        || path.starts_with("/swagger-ui")
        || path == "/robots.txt"
        || path == "/sitemap.xml"
        || path.starts_with("/panel")
    {
        return next.run(request).await;
    }

    let is_bot = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(is_crawler);

    /* [175A-2] Home page: inyectar preload de imagen hero + datos iniciales para usuarios normales.
     * Una sola llamada a BD obtiene los proyectos publicados:
     *   - El primer in_carousel genera el <link rel="preload"> con ALLOWED_WIDTHS correctos
     *   - Todos los proyectos se inyectan como window.__INITIAL_DATA__ para pre-poblar React Query
     * Esto elimina la cadena: React mount → API round-trip → descubrimiento de imagen.
     * LCP esperado: ~2s (límite del bundle JS) en lugar de ~8s. */
    if path == "/" && !is_bot {
        let index_path = format!("{}/index.html", state.static_dir);
        if let Ok(template) = tokio::fs::read_to_string(&index_path).await {
            let projects = ProjectRepository::list_published(&state.pool).await.ok();
            let mut inject = String::new();

            if let Some(projs) = projects {
                /* Extraer path del hero ANTES de consumir el Vec (Project no es Clone) */
                let hero_path: Option<String> = projs
                    .iter()
                    .filter(|p| p.in_carousel)
                    .find_map(|p| p.gallery_image.clone().or_else(|| p.featured_image.clone()));

                if let Some(ref img_path) = hero_path {
                    inject.push_str(&build_hero_preload_from_path(img_path));
                }

                /* Script de datos iniciales: consume el Vec con into_iter */
                if let Some(script) = build_initial_data_script(projs) {
                    inject.push_str(&script);
                }
            }

            let html = if inject.is_empty() {
                template
            } else {
                template.replace("</head>", &format!("{inject}</head>"))
            };
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::from(html))
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }
    }

    if !is_bot {
        return next.run(request).await;
    }

    /* Leer index.html del SPA como template */
    let index_path = format!("{}/index.html", state.static_dir);
    let Ok(template) = tokio::fs::read_to_string(&index_path).await else {
        return next.run(request).await;
    };

    /* Resolver meta SEO para esta ruta (con cache DB para estáticas) */
    let Some(meta) = resolve_seo_meta(path, &state.pool, &state.app_url, &state.seo_cache).await else {
        /* Ruta desconocida: el SPA se encarga */
        return next.run(request).await;
    };

    let html = inject_seo_into_html(&template, &meta, &state.app_url);

    tracing::info!(path, "SEO meta dinámico inyectado para crawler");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(html))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .expect("fallback response")
        })
}
