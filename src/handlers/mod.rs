#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

mod ask;
mod auth;
mod chat;
mod chat_staff;
mod chat_tools;
mod health;
mod ia;
mod inmuebles;
pub mod marketplace;
mod notes;
mod public;
mod solicitud;
mod sombra;
mod suscriptor;
mod uploads;
mod users;
mod visita;
mod whatsapp;

use std::path::{Path, PathBuf};

use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::AppState;

/// Define el esquema de seguridad Bearer para Swagger UI
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        /* components existe porque el derive ya registra schemas */
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::register,
        auth::login,
        ask::get_ficha,
        ask::set_ficha,
        users::create_user,
        notes::create_note,
        notes::get_note,
        notes::list_notes,
        notes::update_note,
        notes::delete_note,
        inmuebles::create_inmueble,
        inmuebles::list_inmuebles,
        inmuebles::get_inmueble,
        inmuebles::update_inmueble,
        inmuebles::set_publicacion,
        inmuebles::delete_inmueble,
        inmuebles::add_foto,
        inmuebles::delete_foto,
        uploads::upload_foto,
        uploads::servir_archivo,
        uploads::servir_archivo_solicitud,
        uploads::servir_archivo_whatsapp,
        solicitud::subir_foto_solicitud,
        solicitud::create_solicitud,
        solicitud::list_solicitudes,
        solicitud::revisar_solicitud,
        visita::list_visitas,
        visita::revisar_visita,
        marketplace::emitir_token,
        marketplace::emitir_token_cli,
        marketplace::borrador,
        marketplace::regenerar,
        marketplace::corregir,
        marketplace::audit,
        marketplace::uso,
        public::list_public,
        public::get_public,
        suscriptor::suscribir,
    ),
    components(schemas(
        health::HealthResponse,
        crate::models::RegisterRequest,
        crate::models::LoginRequest,
        crate::models::AuthResponse,
        crate::models::UserResponse,
        crate::models::CreateUserRequest,
        crate::models::FichaAskRequest,
        crate::models::FichaAskResponse,
        crate::models::Note,
        crate::models::CreateNoteRequest,
        crate::models::UpdateNoteRequest,
        crate::models::PaginatedNotes,
        crate::models::Inmueble,
        crate::models::Foto,
        crate::models::FotoPublica,
        crate::models::CopyInmueble,
        crate::models::CreateInmuebleRequest,
        crate::models::UpdateInmuebleRequest,
        crate::models::PublicacionRequest,
        crate::models::AddFotoRequest,
        crate::models::PaginatedInmuebles,
        crate::models::Solicitud,
        crate::models::FotoSolicitud,
        crate::models::FotoSolicitudSubida,
        crate::models::CreateSolicitudRequest,
        crate::models::UpdateEstadoSolicitud,
        crate::models::PaginatedSolicitudes,
        crate::models::Visita,
        crate::models::PaginatedVisitas,
        crate::models::UpdateEstadoVisita,
        crate::services::marketplace::PromptSeguro,
        crate::services::marketplace::BorradorRequest,
        crate::services::marketplace::ExcerptIn,
        crate::services::marketplace::ExtrasIn,
        crate::services::marketplace::Tono,
        crate::services::marketplace::Largo,
        crate::handlers::marketplace::TokenResponse,
        crate::handlers::marketplace::BorradorResponse,
        crate::handlers::marketplace::AuditIn,
        crate::handlers::marketplace::EventoAudit,
        crate::handlers::marketplace::CliTokenRequest,
        crate::handlers::marketplace::CorregirRequest,
        crate::handlers::marketplace::CorregirResponse,
        crate::handlers::marketplace::UsoQuery,
        crate::services::marketplace::UsoDia,
        crate::models::CreateSuscriptorRequest,
        crate::models::Suscriptor,
        crate::errors::ErrorResponse,
    )),
    modifiers(&SecurityAddon),
    info(
        title = "Glory RS API",
        version = "0.1.0",
        description = "Template API — Rust + Axum + OpenAPI"
    )
)]
#[allow(clippy::needless_for_each)]
pub struct ApiDoc;

/// Crea el router principal con CORS, tracing, Swagger UI y todas las rutas
pub fn create_router(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    /* [169A-1] El chat trae estado propio (AgentState): se anida ya con
     * estado en /api para exponer /api/agent/{ws,messages,history}.
     * [169A-4] Comparte un `ChatHub` con las rutas staff: el humano
     * responde por el mismo WS que escucha el visitante. */
    let hub = glory_agent::session::ChatHub::new();
    let agent = chat::agent_router(pool.clone(), hub.clone());
    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        upload_dir: config.upload_dir.into(),
        hub,
        static_dir: config.static_dir.map(PathBuf::from),
        /* [03AA-3 M4] Singleflight por proceso del asistente Marketplace. */
        mp_vuelo: std::sync::Arc::new(crate::services::marketplace::Singleflight::default()),
    };

    /* [239A-1] CORS: abierto solo si no hay `CORS_ORIGINS` (dev). En prod se
     * fija `CORS_ORIGINS=https://mn-inmobiliaria.com` y el resto se rechaza. */
    let permitidos: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors = if permitidos.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(permitidos))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(Any)
    };

    /* [239A-1] Monorepo: si `STATIC_DIR` trae `index.html`, el front SPA se
     * sirve desde el propio binario (fichero tal cual o `index.html`). En dev
     * (sin `STATIC_DIR`) no se registra nada y todo sigue igual. */
    let sirve_front = state
        .static_dir
        .as_ref()
        .is_some_and(|d| d.join("index.html").is_file());
    if sirve_front {
        tracing::info!("Front SPA embebido desde {:?}", state.static_dir);
    }

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/uploads/:inmueble/:archivo", get(uploads::servir_archivo))
        .route(
            "/uploads/solicitudes/:sesion/:archivo",
            get(uploads::servir_archivo_solicitud),
        )
        /* [279A-2] Fotos entrantes de WhatsApp archivadas en disco local. */
        .route(
            "/uploads/whatsapp/:telefono/:archivo",
            get(uploads::servir_archivo_whatsapp),
        )
        /* [249A-1] Sitemap dinámico (ruta explícita: no cae al fallback). */
        .route("/sitemap.xml", get(sitemap))
        /* [249A-4] `llms.txt` y catálogo agente dinámicos desde la BD
         * (siempre frescos, sin depender del prebuild): el estático
         * `public/llms.txt` nunca llegaba al build de Coolify y el
         * fallback servía `index.html` en su lugar (agéntica 1/4). */
        .route("/llms.txt", get(llms_txt))
        .route("/.well-known/ai-catalog.json", get(ai_catalog))
        .nest("/api", api_routes())
        /* nest_service porque el chat trae Router<()> (estado propio):
         * nest exige el mismo estado. Despoja /api igual que nest. */
        .nest_service("/api", agent);
    /* [249A-2] El fallback va ANTES de los layers: en axum un layer solo
     * envuelve lo ya registrado; con el fallback despues de los layers el
     * SPA (JS/CSS/HTML) salia sin gzip aunque la API si comprimia
     * (verificado en prod: `Content-Encoding` ausente en el JS/CSS/HTML).
     * `with_state` sigue ultimo y da estado a rutas y fallback por igual. */
    let app = if sirve_front {
        app.fallback(fallback_spa)
    } else {
        app
    };
    let app = app
        .layer(TraceLayer::new_for_http())
        /* [249A-1] Compresión gzip de respuestas (el JS de 612 KB viaja
         * comprimido; PageSpeed lo exigía: no había Content-Encoding).
         * [249A-2] Salvo imágenes (JPEG/PNG/WebP ya van comprimidos;
         * comprimirlos quema CPU sin ahorrar bytes) y cuerpos < 1 KB. */
        .layer(
            CompressionLayer::new()
                .compress_when(SizeAbove::new(1024).and(NotForContentType::IMAGES)),
        )
        .layer(cors);
    app.with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(notes::routes())
        .nest("/admin", admin_routes())
        .nest("/public", public::routes())
}

/// Rutas de administración: todo requiere JWT (`AuthUser` por handler)
fn admin_routes() -> Router<AppState> {
    Router::new()
        .merge(inmuebles::routes())
        /* [279A-3] Ficha /ask de la dueña (rutas bajo /api/admin/...). */
        .merge(ask::routes())
        .merge(solicitud::admin_routes())
        .merge(visita::admin_routes())
        .merge(uploads::routes())
        .merge(users::routes())
        /* [03AA-3 M3] Asistente Marketplace: token mp, borrador y audit. */
        .merge(marketplace::routes())
        /* [169A-4] Atención del chat: bandeja, hilo, responder, tomar/soltar
         * IA y config (rutas bajo /api/admin/agent). */
        .merge(chat_staff::staff_routes())
        /* [199A-1] Centro de IA de texto: estado/config/probar/completar
         * (rutas bajo /api/admin/ia). */
        .merge(ia::routes())
}

/* [239A-1] SPA del monorepo: sirve el fichero tal cual si existe y cae a
 * `index.html` en cualquier otra ruta (el front resuelve sus rutas). Las
 * rutas de API nunca caen aquí: devuelven 404 seco para no enmascarar
 * errores del backend con HTML. */
async fn fallback_spa(
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
    /* `is_file` y no `exists`: `/` resuelve al propio directorio base y debe
     * caer a `index.html` en vez de intentar leer el directorio. */
    let candidato = Path::new(&base).join(ruta.trim_start_matches('/'));
    let archivo = if es_fichero(&candidato).await {
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
async fn sitemap(
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

async fn llms_txt(
    axum::extract::State(est): axum::extract::State<AppState>,
) -> Result<impl axum::response::IntoResponse, crate::errors::AppError> {
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
async fn ai_catalog(
    axum::extract::State(est): axum::extract::State<AppState>,
) -> Result<impl axum::response::IntoResponse, crate::errors::AppError> {
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
    use super::*;

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
