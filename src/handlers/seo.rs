/* [044A-28] Handlers SEO: robots.txt y sitemap.xml.
 * [114A-SEO3] sitemap.xml ahora incluye rutas dinámicas (servicios, proyectos) desde BD.
 * [124A-SENT-R1] Queries directas → ServiceRepository::public_slugs y ProjectRepository::public_slugs.
 * [074A-marketing] Blog posts publicados añadidos al sitemap. */
use axum::http::header;
use axum::{extract::State, response::IntoResponse, routing::get, Router};

use crate::repositories::{BlogRepository, ProjectRepository, ServiceRepository};
use crate::AppState;

const SITE_URL: &str = "https://nakomi.studio";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/robots.txt", get(robots_txt))
        .route("/sitemap.xml", get(sitemap_xml))
        .route("/llms.txt", get(llms_txt))
        .route("/blog/feed.xml", get(blog_feed_xml))
}

/* [SEO-C] llms.txt: resumen del sitio en formato Markdown para crawlers de IA.
 * Estándar propuesto para que ChatGPT, Perplexity, Claude y Gemini entiendan
 * la estructura y servicios del sitio. */
async fn llms_txt() -> impl IntoResponse {
    let body = "# Nakomi Studio\n\
\n\
        ## Descripción\n\
        Nakomi Studio es una agencia digital con sede en Copenhague, especializada en\n\
        desarrollo web, aplicaciones móviles, agentes de IA e identidad de marca.\n\
        Opera globalmente con clientes en Europa, América y Asia.\n\
\n\
        ## Servicios\n\
        - Diseño y desarrollo web: Sitios corporativos, landings, portafolios desde $350\n\
        - Desarrollo de aplicaciones: Apps web y móviles con React, Rust, Node.js\n\
        - Agentes de IA: Chatbots, asistentes virtuales, automatización con IA\n\
        - Identidad de marca: Branding, logos, guías de estilo\n\
        - Hosting administrado: WordPress y Nginx desde $2.48/mes\n\
        - Servidores VPS: Infraestructura dedicada desde $4.73/mes\n\
        - E-commerce: Tiendas online con pasarelas de pago integradas\n\
\n\
        ## Oferta especial\n\
        50% de descuento en el primer servicio para nuevos clientes.\n\
\n\
        ## Contacto\n\
        - Web: https://nakomi.studio\n\
        - Chat en vivo disponible en el sitio\n\
\n\
        ## Stack tecnológico\n\
        - Backend: Rust (Axum), PostgreSQL, SQLx\n\
        - Frontend: React 18, TypeScript, Vite\n\
        - Infraestructura: Coolify, Docker, Traefik\n\
        - IA: DeepSeek, Groq, Gemini (multi-proveedor con fallback)\n\
\n\
        ## Páginas principales\n\
        - Servicios: https://nakomi.studio/servicios\n\
        - Proyectos: https://nakomi.studio/proyectos\n\
        - Blog: https://nakomi.studio/blog\n\
        - Hosting: https://nakomi.studio/soluciones/hosting\n\
        - VPS: https://nakomi.studio/soluciones/vps\n\
        - Nosotros: https://nakomi.studio/nosotros";
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

async fn robots_txt() -> impl IntoResponse {
    let body = format!(
        "User-agent: *\n\
         Allow: /\n\
         Disallow: /panel\n\
         Disallow: /swagger-ui\n\
         Disallow: /api-docs\n\n\
         Sitemap: {SITE_URL}/sitemap.xml"
    );
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

async fn sitemap_xml(State(state): State<AppState>) -> impl IntoResponse {
    /* Rutas estáticas del SPA */
    let mut rutas: Vec<(String, &str, &str)> = vec![
        ("/".into(), "1.0", "weekly"),
        ("/servicios".into(), "0.9", "weekly"),
        ("/proyectos".into(), "0.9", "weekly"),
        ("/nosotros".into(), "0.7", "monthly"),
        ("/blog".into(), "0.8", "daily"),
        ("/soluciones".into(), "0.8", "monthly"),
        ("/soluciones/hosting".into(), "0.8", "monthly"),
        ("/soluciones/hosting-wordpress".into(), "0.8", "monthly"),
        ("/soluciones/vps".into(), "0.8", "monthly"),
        ("/contacto".into(), "0.6", "monthly"),
        ("/politica-privacidad".into(), "0.3", "yearly"),
    ];

    /* [114A-SEO3] Rutas dinámicas desde BD: servicios con slug público */
    if let Ok(slugs) = ServiceRepository::public_slugs(&state.pool).await {
        for slug in slugs {
            rutas.push((format!("/servicios/{slug}"), "0.8", "monthly"));
        }
    }

    /* [114A-SEO3] Rutas dinámicas: proyectos con slug público */
    if let Ok(slugs) = ProjectRepository::public_slugs(&state.pool).await {
        for slug in slugs {
            rutas.push((format!("/proyectos/{slug}"), "0.7", "monthly"));
        }
    }

    /* [074A-marketing] Rutas dinámicas: posts publicados del blog */
    if let Ok(slugs) = BlogRepository::public_slugs(&state.pool).await {
        for slug in slugs {
            rutas.push((format!("/blog/{slug}"), "0.9", "weekly"));
        }
    }

    let urls: String = rutas
        .iter()
        .map(|(path, priority, freq)| {
            format!(
                "  <url>\n\
                 \x20   <loc>{SITE_URL}{path}</loc>\n\
                 \x20   <changefreq>{freq}</changefreq>\n\
                 \x20   <priority>{priority}</priority>\n\
                 \x20 </url>"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         {urls}\n\
         </urlset>"
    );

    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
}

/* [277A-14] RSS feed para el blog: /blog/feed.xml
 * Genera Atom feed con posts publicados para suscriptores y agregadores. */
async fn blog_feed_xml(State(state): State<AppState>) -> impl IntoResponse {
    let posts = BlogRepository::list_all(&state.pool).await.unwrap_or_default();
    let published: Vec<_> = posts.into_iter().filter(|p| p.status == "published").collect();

    let entries: String = published
        .iter()
        .map(|post| {
            let link = format!("{SITE_URL}/blog/{}", post.slug);
            let published = post
                .published_at
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();
            let updated = post.updated_at.to_rfc3339();
            let summary = post.excerpt.as_deref().unwrap_or("");
            format!(
                "  <entry>\n\
                 \x20   <title>{}</title>\n\
                 \x20   <link href=\"{}\"/>\n\
                 \x20   <id>{}</id>\n\
                 \x20   <published>{}</published>\n\
                 \x20   <updated>{}</updated>\n\
                 \x20   <summary>{}</summary>\n\
                 \x20   <author><name>Nakomi Studio</name></author>\n\
                 \x20 </entry>",
                html_escape_atom(&post.title),
                link,
                link,
                published,
                updated,
                html_escape_atom(summary),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let feed = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <feed xmlns=\"http://www.w3.org/2005/Atom\">\n\
         \x20 <title>Nakomi Studio Blog</title>\n\
         \x20 <link href=\"{SITE_URL}/blog\" rel=\"alternate\"/>\n\
         \x20 <link href=\"{SITE_URL}/blog/feed.xml\" rel=\"self\"/>\n\
         \x20 <id>{SITE_URL}/blog</id>\n\
         \x20 <subtitle>Artículos sobre desarrollo web, diseño, tecnología e inteligencia artificial.</subtitle>\n\
         \x20 <updated>{}</updated>\n\
         {entries}\n\
         </feed>",
        chrono::Utc::now().to_rfc3339(),
    );

    ([(header::CONTENT_TYPE, "application/atom+xml; charset=utf-8")], feed)
}

fn html_escape_atom(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
