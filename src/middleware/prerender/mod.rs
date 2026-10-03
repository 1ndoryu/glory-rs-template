/* [214A-2] Middleware SEO dinamico para crawlers.
 * Reemplaza el enfoque estatico de 114A-SEO3 (HTML pre-renderizado con Puppeteer)
 * que no soportaba contenido CMS editable.
 *
 * Nuevo enfoque: lee index.html del SPA y le inyecta meta tags dinamicos
 * (title, description, OG, canonical, twitter) usando datos de la BD.
 * Rutas estaticas (/servicios, /nosotros, etc.) usan meta fijos.
 * Rutas dinamicas (/servicios/:slug, /proyectos/:slug) consultan la BD.
 * Usuarios normales reciben el SPA sin cambios. */

/* [01AA-4-f3m] Partido por area: middleware/prerender.rs 619L superaba limite.
 * state = SeoCache + PrerenderState; helpers = SeoMeta + escapes + builders;
 * resolve = SEO meta estatico/dinamico con cache+DB; inject = inyeccion HTML +
 * JSON-LD; middleware = fn prerender Axum.
 * Gotcha: waivers sentinel-disable sqlx-query-as-sin-macro viven en resolve.rs.
 * Pendiente: resto (payments, deployments, hosting_domains, ai_tools_misc, main). */

pub mod helpers;
pub mod inject;
pub mod middleware;
pub mod resolve;
pub mod state;

pub use middleware::prerender;
pub use state::{PrerenderState, SeoCache};
