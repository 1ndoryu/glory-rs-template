#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::format_in_format_args)]
#![allow(clippy::must_use_candidate)]

pub mod bootstrap;
pub mod config;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod services;
pub mod util;

use sqlx::PgPool;

use crate::services::docker_stats::DockerStatsCache;
use crate::services::{
    AiChatConfig, ChatHub, ChatTimingService, ContaboService, CoolifyConfig, EmailConfig,
    NotificationHub,
};

/// Estado compartido de la aplicación — accesible desde handlers y middleware
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub static_dir: Option<String>,
    pub http_client: reqwest::Client,
    pub stripe_publishable_key: Option<String>,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub chat_hub: ChatHub,
    pub ai_config: AiChatConfig,
    pub notification_hub: NotificationHub,
    pub chat_timing: ChatTimingService,
    pub contabo_service: Option<ContaboService>,
    /// [104A-42] Config de Coolify para provisioning automático de hostings (VPS2)
    pub coolify_config: Option<CoolifyConfig>,
    /// [VPS1-support] Config de Coolify para la VPS principal (`COOLIFY_VPS1_*`)
    pub coolify_config_vps1: Option<CoolifyConfig>,
    /// [154A-15c] Config de SMTP para emails transaccionales
    pub email_config: Option<EmailConfig>,
    /// [114A-15+] Cache de stats de contenedores Docker (30s TTL)
    pub docker_stats_cache: DockerStatsCache,
    /* [154A-2] Fixture manager para sincronizar archivos TOML de content/ con la BD desde el panel admin.
     * Arc porque ContentManager no es Clone (contiene Box<dyn Fn>). None si content/ no existe. */
    pub fixture_manager: Option<std::sync::Arc<glory_rs::fixtures::ContentManager>>,
}
