/* [01AA-4-f3m] Helpers puros del prerender (extraido de prerender.rs).
 * SeoMeta y sus campos son pub(crate): los construyen resolve.rs e inject.rs. */

use std::fmt::Write;

use crate::models::Project;

pub(crate) const CRAWLER_AGENTS: &[&str] = &[
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

pub(crate) fn is_crawler(user_agent: &str) -> bool {
    let ua = user_agent.to_ascii_lowercase();
    CRAWLER_AGENTS.iter().any(|bot| ua.contains(bot))
}

/* Metadatos SEO por ruta */
pub(crate) struct SeoMeta {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) og_image: Option<String>,
    pub(crate) canonical: String,
    pub(crate) og_type: &'static str,
    pub(crate) json_ld: Option<String>,
}

impl SeoMeta {
    /* Genera tags OG + Twitter que se inyectan antes de </head> */
    pub(crate) fn og_tags(&self, app_url: &str) -> String {
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
pub(crate) fn build_hero_preload_from_path(img_path: &str) -> String {
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
pub(crate) fn build_initial_data_script(projects: Vec<Project>) -> Option<String> {
    let responses: Vec<_> = projects.into_iter().map(Project::into_response).collect();
    let json = serde_json::to_string(&responses).ok()?;
    /* Escapar </script> para prevenir XSS si algún campo contiene ese literal */
    let safe_json = json.replace("</", "<\\/");
    Some(format!(
        "<script>window.__INITIAL_DATA__={{\"projects\":{safe_json}}}</script>\n"
    ))
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/* Escapa caracteres especiales para interpolación segura en strings JSON */
pub(crate) fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
