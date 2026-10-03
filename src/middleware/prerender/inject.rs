/* [01AA-4-f3m] Inyeccion HTML + JSON-LD (extraido de prerender.rs).
 * Visibilidad pub(crate): lo usan middleware.rs e resolve.rs. */

use super::helpers::{SeoMeta, html_escape};

/* Inyecta meta SEO en el HTML del SPA: reemplaza <title> y description
 * existentes, y agrega OG/canonical antes de </head>. */
pub(crate) fn inject_seo_into_html(html: &str, meta: &SeoMeta, app_url: &str) -> String {
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
        let script = format!("<script type=\"application/ld+json\">{json_ld}</script>\n");
        result = result.replace("</head>", &format!("{script}</head>"));
    }

    result
}

/* [277A-14] Genera JSON-LD para rutas estáticas conocidas.
 * organization+website para home, organization para catálogos,
 * breadcrumb para detalle. */
pub(crate) fn static_json_ld(path: &str, app_url: &str) -> Option<String> {
    let site = app_url;
    let org = format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":[\"ProfessionalService\",\"LocalBusiness\"],\"name\":\"Nakomi Studio\",\"url\":\"{site}\",\"description\":\"Estudio creativo especializado en desarrollo web, aplicaciones, agentes de IA e identidad de marca.\",\"address\":{{\"@type\":\"PostalAddress\",\"addressLocality\":\"Copenhagen\",\"addressCountry\":\"DK\"}},\"sameAs\":[\"https://github.com/1ndoryu\",\"https://www.linkedin.com/company/nakomi-studio\"]}}"
    );
    match path {
        "/" => {
            let web = format!(
                "{{\"@context\":\"https://schema.org\",\"@type\":\"WebSite\",\"name\":\"Nakomi Studio\",\"url\":\"{site}\"}}"
            );
            Some(format!(
                "{{\"@context\":\"https://schema.org\",\"@graph\":[{org},{web}]}}"
            ))
        }
        "/servicios" | "/proyectos" => Some(org),
        "/nosotros" => {
            let breadcrumb = format!(
                "{{\"@context\":\"https://schema.org\",\"@type\":\"BreadcrumbList\",\"itemListElement\":[{{\"@type\":\"ListItem\",\"position\":1,\"name\":\"Inicio\",\"item\":\"{site}\"}},{{\"@type\":\"ListItem\",\"position\":2,\"name\":\"Nosotros\",\"item\":\"{site}/nosotros\"}}]}}"
            );
            Some(format!(
                "{{\"@context\":\"https://schema.org\",\"@graph\":[{breadcrumb}]}}"
            ))
        }
        _ => None,
    }
}

/* [277A-14] Genera JSON-LD dinámico para servicios desde DB */
pub(crate) fn dynamic_service_json_ld(title: &str, desc: &str, slug: &str, app_url: &str) -> String {
    let breadcrumb = format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":\"BreadcrumbList\",\"itemListElement\":[{{\"@type\":\"ListItem\",\"position\":1,\"name\":\"Inicio\",\"item\":\"{app_url}\"}},{{\"@type\":\"ListItem\",\"position\":2,\"name\":\"Servicios\",\"item\":\"{app_url}/servicios\"}},{{\"@type\":\"ListItem\",\"position\":3,\"name\":\"{title}\",\"item\":\"{app_url}/servicios/{slug}\"}}]}}"
    );
    let service = format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":\"Service\",\"name\":\"{title}\",\"description\":\"{desc}\",\"url\":\"{app_url}/servicios/{slug}\",\"provider\":{{\"@type\":\"ProfessionalService\",\"name\":\"Nakomi Studio\",\"url\":\"{app_url}\"}}}}"
    );
    format!("{{\"@context\":\"https://schema.org\",\"@graph\":[{breadcrumb},{service}]}}")
}

/* [277A-14] Genera JSON-LD para blog posts desde DB */
pub(crate) fn dynamic_blog_json_ld(title: &str, desc: &str, slug: &str, app_url: &str) -> String {
    format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":\"BlogPosting\",\"headline\":\"{title}\",\"description\":\"{desc}\",\"url\":\"{app_url}/blog/{slug}\",\"author\":{{\"@type\":\"Organization\",\"name\":\"Nakomi Studio\",\"url\":\"{app_url}\"}},\"publisher\":{{\"@type\":\"Organization\",\"name\":\"Nakomi Studio\",\"url\":\"{app_url}\"}}}}"
    )
}
