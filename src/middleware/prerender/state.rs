/* [01AA-4-f3m] Estado del prerender SEO (extraido de prerender.rs).
 * CachedSeoEntry/entries/ttl son pub(crate): los usa resolve.rs para el
 * cache de estaticas (codigo movido verbatim, sin refactor). */

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::helpers::SeoMeta;

/* [277A-13] Cache entry para SEO settings de paginas estaticas.
 * Evita consultar la DB en cada request de crawler. TTL: 5 minutos. */
pub(crate) struct CachedSeoEntry {
    pub(crate) meta: SeoMeta,
    pub(crate) fetched_at: Instant,
}

#[derive(Clone)]
pub struct SeoCache {
    pub(crate) entries: Arc<RwLock<HashMap<String, CachedSeoEntry>>>,
    pub(crate) ttl: Duration,
}

impl SeoCache {
    pub fn new() -> Self {
        Self::default()
    }
}

/* [259A-1] Default requerido por clippy (new_without_default). */
impl Default for SeoCache {
    fn default() -> Self {
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
