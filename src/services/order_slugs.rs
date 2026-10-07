/* [095A-19] Compatibilidad de slugs para checkout.
 * El CMS publico y los fixtures legacy han usado slugs distintos para los mismos
 * servicios. El backend acepta ambos para que crear ordenes no dependa de mapas
 * del frontend que se desactualizan al editar contenido. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{ServicePlan, ServiceRecord};
use crate::repositories::ServiceRepository;

const BASE_PLAN_SLUGS: &[&str] = &["basico", "medio", "avanzado", "personalizado"];

fn push_unique_slug(candidates: &mut Vec<String>, slug: &str) {
    let normalized = slug.trim().to_lowercase();
    if !normalized.is_empty() && !candidates.iter().any(|candidate| candidate == &normalized) {
        candidates.push(normalized);
    }
}

fn service_slug_aliases(slug: &str) -> &'static [&'static str] {
    match slug {
        "diseno-de-sitios-web" => &["diseno-web"],
        "diseno-web" => &["diseno-de-sitios-web"],
        "desarrollo-de-aplicaciones" => &["desarrollo-apps"],
        "desarrollo-apps" => &["desarrollo-de-aplicaciones"],
        "agentes-de-ia" => &["agentes-ia"],
        "agentes-ia" => &["agentes-de-ia"],
        "identidad-de-marca" => &["branding", "identidad-marca"],
        "identidad-marca" | "branding" => &["identidad-de-marca"],
        "e-commerce" => &["ecommerce"],
        "ecommerce" => &["e-commerce"],
        _ => &[],
    }
}

fn service_slug_candidates(slug: &str) -> Vec<String> {
    let normalized = slug.trim().to_lowercase();
    let mut candidates = Vec::new();
    push_unique_slug(&mut candidates, &normalized);

    for alias in service_slug_aliases(&normalized) {
        push_unique_slug(&mut candidates, alias);
    }

    candidates
}

fn plan_slug_candidates(slug: &str) -> Vec<String> {
    let normalized = slug.trim().to_lowercase();
    let mut candidates = Vec::new();
    push_unique_slug(&mut candidates, &normalized);

    if let Some(suffix) = normalized.rsplit('-').next() {
        if BASE_PLAN_SLUGS.contains(&suffix) {
            push_unique_slug(&mut candidates, suffix);
        }
    }

    candidates
}

pub(crate) async fn find_service_for_order(
    pool: &PgPool,
    requested_slug: &str,
) -> Result<ServiceRecord, AppError> {
    for candidate in service_slug_candidates(requested_slug) {
        if let Some(service) = ServiceRepository::find_service_by_slug(pool, &candidate).await? {
            return Ok(service);
        }
    }

    Err(AppError::NotFound(format!(
        "Servicio '{requested_slug}' no encontrado"
    )))
}

pub(crate) async fn find_plan_for_order(
    pool: &PgPool,
    service_id: Uuid,
    requested_slug: &str,
) -> Result<ServicePlan, AppError> {
    for candidate in plan_slug_candidates(requested_slug) {
        if let Some(plan) =
            ServiceRepository::find_plan_by_slug(pool, service_id, &candidate).await?
        {
            return Ok(plan);
        }
    }

    Err(AppError::NotFound(format!(
        "Plan '{requested_slug}' no encontrado"
    )))
}

#[cfg(test)]
mod tests {
    use super::{plan_slug_candidates, service_slug_candidates};

    #[test]
    fn service_slug_candidates_keep_cms_slug_first() {
        assert_eq!(
            service_slug_candidates("diseno-de-sitios-web"),
            vec!["diseno-de-sitios-web".to_string(), "diseno-web".to_string()]
        );
    }

    #[test]
    fn service_slug_candidates_include_current_alias_for_legacy_slug() {
        assert_eq!(
            service_slug_candidates("branding"),
            vec!["branding".to_string(), "identidad-de-marca".to_string()]
        );
    }

    #[test]
    fn plan_slug_candidates_keep_exact_then_known_suffix() {
        assert_eq!(
            plan_slug_candidates("web-basico"),
            vec!["web-basico".to_string(), "basico".to_string()]
        );
    }
}
