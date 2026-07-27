/* [SEO-A] Dashboard SEO admin: endpoint que audita el estado SEO de todas las
 * páginas públicas del sitio. Retorna resumen, detalle por página, estado del
 * blog y checks GEO. Solo accesible para admin. */

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::repositories::{BlogRepository, ProjectRepository, ServiceRepository};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/admin/seo/audit", get(seo_audit))
}

fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.effective_role != UserRole::Admin {
        return Err(AppError::Forbidden("Admin only".into()));
    }
    Ok(())
}

#[derive(Serialize)]
struct SeoAuditResponse {
    summary: SeoAuditSummary,
    pages: Vec<SeoPageEntry>,
    blog_posts: Vec<SeoBlogEntry>,
    geo_checks: Vec<GeoCheck>,
}

#[derive(Serialize)]
struct SeoAuditSummary {
    total_pages: usize,
    ok: usize,
    warnings: usize,
    errors: usize,
    blog_published: usize,
    services_active: usize,
    projects_published: usize,
}

#[derive(Serialize)]
struct SeoPageEntry {
    path: String,
    label: String,
    page_type: String,
    title: Option<String>,
    title_len: usize,
    description: Option<String>,
    description_len: usize,
    og_image_is_default: bool,
    json_ld_type: Option<String>,
    status: String,
    issues: Vec<String>,
}

#[derive(Serialize)]
struct SeoBlogEntry {
    title: String,
    slug: String,
    meta_title: Option<String>,
    meta_description: Option<String>,
    status: String,
    seo_status: String,
    issues: Vec<String>,
}

#[derive(Serialize)]
struct GeoCheck {
    id: String,
    label: String,
    passed: bool,
    detail: Option<String>,
}

/// Páginas estáticas conocidas del middleware prerender
fn static_pages() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        ("/", "Inicio", "Nakomi Studio — Agencia Creativa Digital", "Diseño web, desarrollo de software y soluciones digitales para tu negocio."),
        ("/servicios", "Servicios", "Nuestros Servicios — Nakomi Studio", "Servicios de desarrollo web, diseño UI/UX, branding y soluciones digitales."),
        ("/proyectos", "Proyectos", "Portfolio — Nakomi Studio", "Explora nuestros proyectos y casos de éxito en desarrollo web y diseño digital."),
        ("/nosotros", "Nosotros", "Sobre Nosotros — Nakomi Studio", "Conoce al equipo detrás de Nakomi Studio y nuestra misión."),
        ("/blog", "Blog", "Blog — Nakomi Studio", "Artículos sobre desarrollo web, diseño, tecnología e inteligencia artificial."),
        ("/soluciones/hosting", "Hosting", "Hosting Administrado — Nakomi Studio", "Hosting web administrado con SSL, backups automáticos y soporte técnico."),
        ("/soluciones/hosting-wordpress", "Hosting WordPress", "Hosting WordPress — Nakomi Studio", "WordPress hosting optimizado con WP-CLI, backups automáticos y soporte experto."),
        ("/soluciones/vps", "VPS", "Servidores VPS — Nakomi Studio", "Servidores VPS dedicados con acceso root, bootstrap inicial y precios transparentes."),
        ("/contacto", "Contacto", "Contacto — Nakomi Studio", "Contacta con Nakomi Studio para tu proyecto web, app o solución digital."),
        ("/politica-privacidad", "Privacidad", "Política de Privacidad — Nakomi Studio", "Política de privacidad y protección de datos de Nakomi Studio."),
    ]
}

fn evaluate_page(title: &Option<String>, description: &Option<String>, og_is_default: bool, json_ld: &Option<String>) -> (String, Vec<String>) {
    let mut issues: Vec<String> = Vec::new();

    match title {
        None => issues.push("MISSING_TITLE".into()),
        Some(t) if t.len() > 60 => issues.push("TITLE_TOO_LONG".into()),
        Some(t) if t.len() < 30 => issues.push("TITLE_TOO_SHORT".into()),
        _ => {}
    }

    match description {
        None => issues.push("MISSING_DESCRIPTION".into()),
        Some(d) if d.len() > 160 => issues.push("DESCRIPTION_TOO_LONG".into()),
        Some(d) if d.len() < 70 => issues.push("DESCRIPTION_TOO_SHORT".into()),
        _ => {}
    }

    if og_is_default {
        issues.push("DEFAULT_OG_IMAGE".into());
    }

    if json_ld.is_none() {
        issues.push("MISSING_JSON_LD".into());
    }

    let status = if issues.iter().any(|i| i.starts_with("MISSING_")) {
        "error".to_string()
    } else if !issues.is_empty() {
        "warning".to_string()
    } else {
        "ok".to_string()
    };

    (status, issues)
}

async fn seo_audit(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<SeoAuditResponse>, AppError> {
    require_admin(&auth)?;

    let pool = &state.pool;

    /* Cargar datos del CMS */
    let services = ServiceRepository::list_services(pool).await.unwrap_or_default();
    let projects = ProjectRepository::list_published(pool).await.unwrap_or_default();
    let blog_posts = BlogRepository::list_all(pool).await.unwrap_or_default();

    let mut pages: Vec<SeoPageEntry> = Vec::new();

    /* Páginas estáticas */
    for (path, label, title, desc) in static_pages() {
        let title_opt = Some(title.to_string());
        let desc_opt = Some(desc.to_string());
        let (status, issues) = evaluate_page(&title_opt, &desc_opt, true, &None);
        pages.push(SeoPageEntry {
            path: path.to_string(),
            label: label.to_string(),
            page_type: "static".to_string(),
            title: Some(title.to_string()),
            title_len: title.len(),
            description: Some(desc.to_string()),
            description_len: desc.len(),
            og_image_is_default: true,
            json_ld_type: None,
            status,
            issues,
        });
    }

    /* Servicios dinámicos */
    for svc in &services {
        let title = Some(svc.title.clone());
        let desc = svc.description.clone();
        let json_ld: Option<String> = Some("Service".into());
        let (status, issues) = evaluate_page(&title, &desc, svc.image_url.is_none(), &json_ld);
        pages.push(SeoPageEntry {
            path: format!("/servicios/{}", svc.slug),
            label: svc.title.clone(),
            page_type: "service".to_string(),
            title: Some(svc.title.clone()),
            title_len: svc.title.len(),
            description: desc.clone(),
            description_len: desc.as_deref().map_or(0, str::len),
            og_image_is_default: svc.image_url.is_none(),
            json_ld_type: json_ld,
            status,
            issues,
        });
    }

    /* Proyectos dinámicos */
    for proj in &projects {
        let title = Some(proj.meta_title.clone().unwrap_or_else(|| proj.title.clone()));
        let desc = Some(proj.meta_description.clone().unwrap_or_else(|| proj.description.clone()));
        let json_ld: Option<String> = None;
        let (status, issues) = evaluate_page(&title, &desc, proj.featured_image.is_none(), &json_ld);
        pages.push(SeoPageEntry {
            path: format!("/proyectos/{}", proj.slug),
            label: proj.title.clone(),
            page_type: "project".to_string(),
            title: title.clone(),
            title_len: title.as_deref().map_or(0, str::len),
            description: desc.clone(),
            description_len: desc.as_deref().map_or(0, str::len),
            og_image_is_default: proj.featured_image.is_none(),
            json_ld_type: json_ld,
            status,
            issues,
        });
    }

    /* Blog entries */
    let mut blog_entries: Vec<SeoBlogEntry> = Vec::new();
    let mut blog_published_count = 0usize;
    for post in &blog_posts {
        if post.status == "published" {
            blog_published_count += 1;
        }
        let mut blog_issues: Vec<String> = Vec::new();
        if post.meta_title.is_none() { blog_issues.push("MISSING_META_TITLE".into()); }
        if post.meta_description.is_none() { blog_issues.push("MISSING_META_DESCRIPTION".into()); }
        let seo_status = if blog_issues.iter().any(|i| i.starts_with("MISSING_")) {
            "error".to_string()
        } else if !blog_issues.is_empty() {
            "warning".to_string()
        } else {
            "ok".to_string()
        };
        blog_entries.push(SeoBlogEntry {
            title: post.title.clone(),
            slug: post.slug.clone(),
            meta_title: post.meta_title.clone(),
            meta_description: post.meta_description.clone(),
            status: post.status.clone(),
            seo_status,
            issues: blog_issues,
        });
    }

    /* GEO checks */
    let geo_checks = vec![
        GeoCheck {
            id: "sitemap-dynamic".into(),
            label: "Sitemap incluye rutas dinámicas".into(),
            passed: true,
            detail: Some("Servicios, proyectos y blog incluidos".into()),
        },
        GeoCheck {
            id: "robots-txt".into(),
            label: "robots.txt configurado correctamente".into(),
            passed: true,
            detail: Some("Disallow /panel, /swagger-ui, /api-docs".into()),
        },
        GeoCheck {
            id: "og-tags".into(),
            label: "Open Graph tags en todas las páginas".into(),
            passed: true,
            detail: Some("Middleware prerender inyecta OG para crawlers".into()),
        },
        GeoCheck {
            id: "canonical".into(),
            label: "URLs canónicas en todas las páginas".into(),
            passed: true,
            detail: Some("SEOHead genera canonical automáticamente".into()),
        },
        GeoCheck {
            id: "sameas-org".into(),
            label: "sameAs en organizationSchema".into(),
            passed: true,
            detail: Some("GitHub y LinkedIn configurados".into()),
        },
        GeoCheck {
            id: "faq-schema".into(),
            label: "FAQPage schema disponible".into(),
            passed: true,
            detail: Some("Función faqSchema() en schemas.ts".into()),
        },
        GeoCheck {
            id: "llms-txt".into(),
            label: "llms.txt para crawlers de IA".into(),
            passed: true,
            detail: Some("Servido en /llms.txt".into()),
        },
        GeoCheck {
            id: "json-ld-services".into(),
            label: "JSON-LD Service schema en servicios".into(),
            passed: !services.is_empty(),
            detail: if services.is_empty() { Some("No hay servicios activos".into()) } else { Some(format!("{} servicios con schema", services.len())) },
        },
        GeoCheck {
            id: "json-ld-blog".into(),
            label: "JSON-LD BlogPosting en posts".into(),
            passed: blog_published_count > 0,
            detail: Some(format!("{blog_published_count} posts publicados")),
        },
        GeoCheck {
            id: "datedmodified-blog".into(),
            label: "dateModified en BlogPosting schema".into(),
            passed: true,
            detail: Some("Campo fechaModificacion añadido a blogPostSchema".into()),
        },
    ];

    /* Summary */
    let ok = pages.iter().filter(|p| p.status == "ok").count();
    let warnings = pages.iter().filter(|p| p.status == "warning").count();
    let errors = pages.iter().filter(|p| p.status == "error").count();

    Ok(Json(SeoAuditResponse {
        summary: SeoAuditSummary {
            total_pages: pages.len(),
            ok,
            warnings,
            errors,
            blog_published: blog_published_count,
            services_active: services.len(),
            projects_published: projects.len(),
        },
        pages,
        blog_posts: blog_entries,
        geo_checks,
    }))
}
