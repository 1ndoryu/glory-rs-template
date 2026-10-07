/* [01AA-4-f3o] Listado de despliegues con caché stale-while-revalidate
 * (extraido de deployments.rs). */

use std::collections::HashMap;

use axum::extract::State;
use axum::Json;

use super::super::deployment_helpers::{
    build_subscription_lookups, collect_pending_deployment_batches, deployments_cache,
    duplicate_name_keys,
};
use super::mapping::{
    build_failed_runtime_fallback_batches, dedupe_deployments, enrich_deployment_resources,
    failed_runtime_labels, map_runtime_deployments,
};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{CoolifyDeploymentResponse, UserRole};
use crate::repositories::HostingRepository;
use crate::services::infrastructure::coolify_server_targets;
use crate::services::HostingRuntimeService;
use crate::AppState;

/// Listar despliegues reales de infraestructura por runtime (admin only)
#[utoipa::path(
    get,
    path = "/api/hosting/deployments",
    responses(
        (status = 200, description = "Lista de despliegues reales de infraestructura", body = Vec<CoolifyDeploymentResponse>),
        (status = 403, description = "Sin permisos"),
        (status = 503, description = "Ningun runtime configurado"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
#[allow(clippy::too_many_lines)]
pub(crate) async fn list_deployments(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<CoolifyDeploymentResponse>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    if coolify_server_targets(
        state.coolify_config_vps1.as_ref(),
        state.coolify_config.as_ref(),
    )
    .is_empty()
        && !HostingRuntimeService::lightweight_manager_configured()
    {
        return Err(AppError::ServiceUnavailable(
            "No hay runtimes configurados para listar despliegues".into(),
        ));
    }

    let cache = deployments_cache();

    let needs_refresh;
    let mut current_deployments = Vec::new();
    {
        let cache_guard = cache.read().await;
        if cache_guard.deployments.is_empty() {
            needs_refresh = true;
        } else {
            current_deployments = cache_guard.deployments.clone();
            // Stale-while-revalidate: cache dura 30 segundos, pero devolvemos viejo mientras carga el nuevo
            needs_refresh = cache_guard.fetched_at.elapsed() > std::time::Duration::from_secs(30);
        }
    }

    if needs_refresh {
        if current_deployments.is_empty() {
            tracing::info!("[deployments] Primer carga, esperando datos...");
            current_deployments = build_deployments(state.clone()).await?;
            let mut cache_guard = cache.write().await;
            cache_guard.deployments.clone_from(&current_deployments);
            cache_guard.fetched_at = std::time::Instant::now();
        } else {
            tracing::info!(
                "[deployments] Devolviendo de caché (stale), refrescando en background..."
            );
            let state_clone = state.clone();
            tokio::spawn(async move {
                if let Ok(new_deployments) = build_deployments(state_clone).await {
                    let cache = deployments_cache();
                    let mut cache_guard = cache.write().await;
                    cache_guard.deployments = new_deployments;
                    cache_guard.fetched_at = std::time::Instant::now();
                    tracing::info!("[deployments] Caché refrescado en background");
                } else {
                    tracing::warn!("[deployments] Falló el refresco en background");
                }
            });
        }
    } else {
        tracing::info!("[deployments] Devolviendo de caché (fresco)");
    }

    Ok(Json(current_deployments))
}

async fn build_deployments(state: AppState) -> Result<Vec<CoolifyDeploymentResponse>, AppError> {
    tracing::info!("[deployments] -> Inicio build_deployments");

    tracing::info!("[deployments] Consultando repositorios...");
    let subscriptions = HostingRepository::list_all(&state.pool).await?;
    let (subscriptions_by_uuid, subscriptions_by_name) = build_subscription_lookups(&subscriptions);

    let plan_configs = HostingRepository::list_plan_configs(&state.pool).await?;
    let plan_configs_by_name: HashMap<String, _> = plan_configs
        .into_iter()
        .map(|config| (config.plan_name.clone(), config))
        .collect();

    let batch_collection = collect_pending_deployment_batches(&state).await;
    let mut pending_batches = batch_collection.pending_batches;

    if !batch_collection.failed_lookups.is_empty() {
        let fallback_batches =
            build_failed_runtime_fallback_batches(&subscriptions, &batch_collection.failed_lookups);
        let fallback_count: usize = fallback_batches
            .iter()
            .map(|batch| batch.services.len())
            .sum();
        if fallback_count > 0 {
            tracing::warn!(
                "[deployments] {} runtime(s) fallaron; usando {} despliegue(s) persistidos como fallback.",
                batch_collection.failed_lookups.len(),
                fallback_count
            );
            pending_batches.extend(fallback_batches);
        }
    }

    let duplicate_name_keys = duplicate_name_keys(&pending_batches);
    let mut deployments = dedupe_deployments(
        pending_batches
            .into_iter()
            .flat_map(|batch| {
                map_runtime_deployments(
                    batch.services,
                    &batch.fallback_label,
                    &duplicate_name_keys,
                    &subscriptions_by_uuid,
                    &subscriptions_by_name,
                    &plan_configs_by_name,
                )
            })
            .collect(),
    );

    if deployments.is_empty() && !batch_collection.failed_lookups.is_empty() {
        return Err(AppError::ServiceUnavailable(format!(
            "No se pudo consultar la infraestructura para listar despliegues reales. Fallaron: {}.",
            failed_runtime_labels(&batch_collection.failed_lookups)
        )));
    }

    tracing::info!("[deployments] Iniciando enrich_deployment_resources...");
    enrich_deployment_resources(&state, &mut deployments).await;
    tracing::info!("[deployments] Finalizó enrich_deployment_resources");

    deployments.sort_by(|left, right| {
        right
            .linked_subscription_id
            .is_some()
            .cmp(&left.linked_subscription_id.is_some())
            .then(left.name.cmp(&right.name))
            .then(left.uuid.cmp(&right.uuid))
    });

    tracing::info!("[deployments] -> Fin build_deployments");
    Ok(deployments)
}
