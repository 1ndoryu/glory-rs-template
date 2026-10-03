/* [01AA-4-f3m] Fn middleware Axum prerender (extraida de prerender.rs). */

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::repositories::ProjectRepository;

use super::helpers::{build_hero_preload_from_path, build_initial_data_script, is_crawler};
use super::inject::inject_seo_into_html;
use super::resolve::resolve_seo_meta;
use super::state::PrerenderState;

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
    let Some(meta) = resolve_seo_meta(path, &state.pool, &state.app_url, &state.seo_cache).await
    else {
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
            /* [259A-1] Fallback infalible: Response::new + status_mut no pueden
             * fallar (el builder con valores fijos solo falla por input invalido,
             * y aqui ya fallo una vez). Sin expect. */
            let mut fallback = Response::new(Body::empty());
            *fallback.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            fallback
        })
}
