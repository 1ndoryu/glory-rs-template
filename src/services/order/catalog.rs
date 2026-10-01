//! Catálogo: lectura de servicios, planes y fases de plantilla.

use sqlx::PgPool;

use super::OrderService;
use crate::errors::AppError;
use crate::models::{
    parse_service_categories, ServiceDetailResponse, ServicePlanPhaseResponse, ServicePlanResponse,
};
use crate::repositories::ServiceRepository;

impl OrderService {
    /// Lista todos los servicios activos con planes y fases
    pub async fn list_services(pool: &PgPool) -> Result<Vec<ServiceDetailResponse>, AppError> {
        let services = ServiceRepository::list_services(pool).await?;
        let mut result = Vec::with_capacity(services.len());

        for svc in services {
            let plans = ServiceRepository::list_plans_for_service(pool, svc.id).await?;
            let mut plan_responses = Vec::with_capacity(plans.len());

            for plan in plans {
                let phases = ServiceRepository::list_plan_phases(pool, plan.id).await?;
                plan_responses.push(ServicePlanResponse {
                    id: plan.id,
                    slug: plan.slug,
                    name: plan.name,
                    price_cents: plan.price_cents,
                    description: plan.description,
                    features: plan.features,
                    is_highlighted: plan.is_highlighted,
                    is_custom: plan.is_custom,
                    phases: phases
                        .into_iter()
                        .map(ServicePlanPhaseResponse::from)
                        .collect(),
                });
            }

            result.push(ServiceDetailResponse {
                id: svc.id,
                slug: svc.slug,
                title: svc.title,
                description: svc.description,
                categories: parse_service_categories(&svc.categories),
                image_url: svc.image_url,
                base_price_cents: svc.base_price_cents,
                skills: svc.skills,
                content: svc.content,
                gallery: svc.gallery,
                meta_title: svc.meta_title,
                meta_description: svc.meta_description,
                plans: plan_responses,
            });
        }

        Ok(result)
    }

    /// Detalle completo de un servicio por slug
    pub async fn get_service(pool: &PgPool, slug: &str) -> Result<ServiceDetailResponse, AppError> {
        let svc = ServiceRepository::find_service_by_slug(pool, slug)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Servicio '{slug}' no encontrado")))?;

        let plans = ServiceRepository::list_plans_for_service(pool, svc.id).await?;
        let mut plan_responses = Vec::with_capacity(plans.len());

        for plan in plans {
            let phases = ServiceRepository::list_plan_phases(pool, plan.id).await?;
            plan_responses.push(ServicePlanResponse {
                id: plan.id,
                slug: plan.slug,
                name: plan.name,
                price_cents: plan.price_cents,
                description: plan.description,
                features: plan.features,
                is_highlighted: plan.is_highlighted,
                is_custom: plan.is_custom,
                phases: phases
                    .into_iter()
                    .map(ServicePlanPhaseResponse::from)
                    .collect(),
            });
        }

        Ok(ServiceDetailResponse {
            id: svc.id,
            slug: svc.slug,
            title: svc.title,
            description: svc.description,
            categories: parse_service_categories(&svc.categories),
            image_url: svc.image_url,
            base_price_cents: svc.base_price_cents,
            skills: svc.skills,
            content: svc.content,
            gallery: svc.gallery,
            meta_title: svc.meta_title,
            meta_description: svc.meta_description,
            plans: plan_responses,
        })
    }
}
