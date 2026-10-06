#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod config;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod services;

use std::path::PathBuf;

use sqlx::PgPool;

/// Estado compartido de la aplicación — accesible desde handlers y middleware
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub upload_dir: PathBuf,
    /* [239A-1] Monorepo: `Some` = servir el front SPA compilado (solo si el
     * dir trae `index.html`); `None` = solo API, sin cambio en dev. */
    pub static_dir: Option<PathBuf>,
    /* [169A-4] Mismo hub que el router visitante: el staff emite al WS del
     * visitante (`ChatHub` es `Clone` con interiores `Arc`). */
    pub hub: glory_agent::session::ChatHub,
    /* [03AA-3 M4] Singleflight del asistente Marketplace (una generación de
     * IA por clave en vuelo). En `Arc` porque `Mutex` no es `Clone`. */
    pub mp_vuelo: std::sync::Arc<crate::services::marketplace::Singleflight>,
}
