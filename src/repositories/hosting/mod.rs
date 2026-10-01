/* [054A-2] Repositorio de hosting: CRUD para suscripciones y eventos.
 * [114A-3] Plan configs: CRUD para configuración de recursos por plan.
 * Queries verificadas con sqlx offline.
 * [01AA-4-F3k] Partido por dominio (era god-object 600+):
 * types = params + validación; subscriptions = CRUD + eventos + stripe;
 * server = provisioning/SFTP/dominio; plans = plan configs; mail = aliases + buzones. */

mod mail;
mod plans;
mod server;
mod subscriptions;
mod types;

pub use types::{CreateHostingParams, ServerInfo, UpdateHostingParams};

pub struct HostingRepository;
