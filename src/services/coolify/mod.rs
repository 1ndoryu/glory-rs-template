/* [07AA-14] Fachada del servicio Coolify (split de coolify.rs sin cambios de lógica).
 * Módulos: config (CoolifyConfig), types (tipos API), parsing (helpers),
 * wordpress (install/create/finalize), compose (builders WP/static),
 * compose_ssh (sidecars SSH/SFTP), compose_backup (sidecars backup),
 * lifecycle (list/stop/start/restart/domain), provisioning (provision/update/delete).
 * Re-exports preservan `crate::services::coolify::{CoolifyConfig, CoolifyService, ...}`
 * y `super::coolify::{...}` consumidos fuera (services/mod.rs).
 */

mod compose;
mod compose_backup;
mod compose_ssh;
mod config;
mod lifecycle;
mod parsing;
mod provisioning;
mod types;
mod wordpress;

#[cfg(test)]
mod tests_compose_a;
#[cfg(test)]
mod tests_compose_b;
#[cfg(test)]
mod tests_units;

pub use config::CoolifyConfig;
pub use types::{
    CoolifyProvisionResult, CoolifyService, CoolifyServiceSummary, HostingComposeUpdate,
    HostingProvisionPreferences,
};
