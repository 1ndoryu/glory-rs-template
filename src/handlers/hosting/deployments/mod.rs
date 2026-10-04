/* [01AA-4-f3o] Fachada del inventario de despliegues (extraido de deployments.rs).
 * mapping = inventario mínimo + fallback + enriquecido;
 * list = GET + construcción con caché stale-while-revalidate;
 * delete = DELETE de huérfanos con resolución de runtime. */

mod delete;
mod list;
mod mapping;

pub(super) use delete::delete_deployment;
pub(super) use list::list_deployments;
