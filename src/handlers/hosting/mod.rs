mod backups;
mod checkout;
mod control;
mod deployment_helpers;
mod deployments;
mod domain;
pub mod email_aliases;
mod infrastructure;
mod plans;
mod provisioning;
mod routes;
mod self_service;
mod stats;
mod subscriptions;
mod vps;

/* [225A-1] El antiguo controlador monolítico de hosting se dividió por responsabilidad.
 * La raíz queda como fachada para que los límites de tamaño no vuelvan a ocultar
 * archivos enormes y para que cada área compile/testee aislada. */
pub use routes::hosting_routes;
