use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::collections::{HashMap, HashSet};

use super::deployment_helpers::{
    build_subscription_lookups, collect_pending_deployment_batches, deployments_cache,
    duplicate_name_keys, invalidate_deployments_cache, locate_runtime_deployment,
    resolve_server_label, runtime_link_key, FailedRuntimeLookup,
};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{CoolifyDeploymentResponse, HostingSubscription, UserRole};
use crate::repositories::{HostingRepository, InfrastructureRepository};
use crate::services::infrastructure::coolify_server_targets;
use crate::services::{HostingRuntimeDeploymentSummary, HostingRuntimeKind, HostingRuntimeService};
use crate::AppState;

fn map_runtime_deployments(
    services: Vec<HostingRuntimeDeploymentSummary>,
    fallback_label: &str,
    duplicate_name_keys: &HashSet<String>,
    subscriptions_by_uuid: &HashMap<String, &crate::models::HostingSubscription>,
    subscriptions_by_name: &HashMap<String, &crate::models::HostingSubscription>,
    plan_configs_by_name: &HashMap<String, crate::models::HostingPlanConfig>,
) -> Vec<CoolifyDeploymentResponse> {
    services
        .into_iter()
        .map(|service| {
            let deployment_key = runtime_link_key(service.runtime_kind, &service.deployment_id);
            let name_key = runtime_link_key(service.runtime_kind, &service.name);
            let linked_subscription =
                subscriptions_by_uuid
                    .get(&deployment_key)
                    .copied()
                    .or_else(|| {
                        (!duplicate_name_keys.contains(&name_key))
                            .then(|| subscriptions_by_name.get(&name_key).copied())
                            .flatten()
                    });

            let server_label = resolve_server_label(&service, fallback_label);
            let plan_config =
                linked_subscription.and_then(|sub| plan_configs_by_name.get(&sub.plan));

            CoolifyDeploymentResponse {
                uuid: service.deployment_id.clone(),
                runtime_kind: service.runtime_kind.as_str().to_string(),
                deployment_id: service.deployment_id,
                name: service.name,
                status: service.status,
                fqdn: service.fqdn,
                server_uuid: service.target_id,
                server_name: Some(server_label.clone()),
                project_uuid: service.project_id,
                environment_name: service.environment_name,
                linked_subscription_id: linked_subscription.map(|subscription| subscription.id),
                linked_subscription_domain: linked_subscription
                    .and_then(|subscription| subscription.domain.clone()),
                linked_subscription_status: linked_subscription
                    .map(|subscription| subscription.status.clone()),
                linked_subscription_plan: linked_subscription
                    .map(|subscription| subscription.plan.clone()),
                linked_subscription_client: linked_subscription
                    .map(|subscription| subscription.client_name.clone()),
                runtime_sampled_at: None,
                storage_limit_mb: linked_subscription
                    .map(|subscription| subscription.storage_limit_mb),
                runtime_site_cpu_limit_cores: None,
                runtime_site_ram_limit_mb: None,
                runtime_db_cpu_limit_cores: None,
                runtime_db_ram_limit_mb: None,
                runtime_ssh_cpu_limit_cores: None,
                runtime_ssh_ram_limit_mb: None,
                plan_wp_cpu_millicores: plan_config.map(|c| c.wp_cpu_millicores),
                plan_db_cpu_millicores: plan_config.map(|c| c.db_cpu_millicores),
                plan_ssh_cpu_millicores: plan_config.map(|c| c.ssh_cpu_millicores),
                plan_wp_memory_mb: plan_config.map(|c| c.wp_memory_mb),
                plan_db_memory_mb: plan_config.map(|c| c.db_memory_mb),
                plan_ssh_memory_mb: plan_config.map(|c| c.ssh_memory_mb),
                cpu_percent: None,
                ram_used_mb: None,
                ram_limit_mb: None,
                storage_used_mb: None,
                server_label,
            }
        })
        .collect()
}

fn f64_to_i64_rounded(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    format!("{value:.0}").parse::<i64>().ok()
}

fn should_include_subscription_fallback(subscription: &HostingSubscription) -> bool {
    subscription.deployment_id_or_legacy().is_some()
        && !subscription.status.trim().eq_ignore_ascii_case("cancelled")
}

fn subscription_fallback_name(subscription: &HostingSubscription, deployment_id: &str) -> String {
    subscription
        .coolify_site_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            subscription
                .domain
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| deployment_id.to_string())
}

fn subscription_matches_failed_lookup(
    subscription: &HostingSubscription,
    failed_lookup: &FailedRuntimeLookup,
    failed_lookup_count: usize,
) -> bool {
    if HostingRuntimeKind::from_persisted(&subscription.runtime_kind) != failed_lookup.runtime_kind
        || !should_include_subscription_fallback(subscription)
    {
        return false;
    }

    match (
        failed_lookup.target_server_ip.as_deref(),
        subscription.server_ip.as_deref(),
    ) {
        (Some(expected_ip), Some(actual_ip)) => actual_ip == expected_ip,
        (Some(_), None) => failed_lookup_count == 1,
        (None, _) => true,
    }
}

/* [255A-2] Cuando Coolify responde 500, el panel no debe degradar a "cero despliegues".
 * Si un runtime concreto falla, reconstruimos un inventario mínimo desde suscripciones
 * persistidas para conservar la tabla utilizable hasta que el proveedor vuelva. */
fn build_failed_runtime_fallback_batches(
    subscriptions: &[HostingSubscription],
    failed_lookups: &[FailedRuntimeLookup],
) -> Vec<super::deployment_helpers::PendingRuntimeDeployments> {
    failed_lookups
        .iter()
        .filter_map(|failed_lookup| {
            let failed_lookup_count = failed_lookups
                .iter()
                .filter(|lookup| lookup.runtime_kind == failed_lookup.runtime_kind)
                .count();

            let services: Vec<_> = subscriptions
                .iter()
                .filter(|subscription| {
                    subscription_matches_failed_lookup(
                        subscription,
                        failed_lookup,
                        failed_lookup_count,
                    )
                })
                .filter_map(|subscription| {
                    let deployment_id = subscription.deployment_id_or_legacy()?;
                    Some(HostingRuntimeDeploymentSummary {
                        runtime_kind: HostingRuntimeKind::from_persisted(
                            &subscription.runtime_kind,
                        ),
                        deployment_id: deployment_id.to_string(),
                        name: subscription_fallback_name(subscription, deployment_id),
                        status: subscription.status.clone(),
                        fqdn: subscription
                            .domain
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned),
                        target_id: None,
                        target_name: Some(failed_lookup.fallback_label.clone()),
                        project_id: None,
                        environment_name: None,
                    })
                })
                .collect();

            (!services.is_empty()).then_some(super::deployment_helpers::PendingRuntimeDeployments {
                fallback_label: failed_lookup.fallback_label.clone(),
                services,
            })
        })
        .collect()
}

fn dedupe_deployments(
    deployments: Vec<CoolifyDeploymentResponse>,
) -> Vec<CoolifyDeploymentResponse> {
    let mut seen = HashSet::new();

    deployments
        .into_iter()
        .filter(|deployment| {
            seen.insert(format!(
                "{}::{}",
                deployment.runtime_kind, deployment.deployment_id
            ))
        })
        .collect()
}

fn failed_runtime_labels(failed_lookups: &[FailedRuntimeLookup]) -> String {
    let mut seen = HashSet::new();

    failed_lookups
        .iter()
        .filter_map(|failed_lookup| {
            let label = format!(
                "{} ({})",
                failed_lookup.fallback_label,
                failed_lookup.runtime_kind.as_str()
            );
            seen.insert(label.clone()).then_some(label)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/* [225A-4] Enriquece despliegues desde snapshots del sampler, no desde SSH en render.
 * Si aún no hay muestras, el panel muestra guiones hasta que el loop background
 * capture el primer promedio. */
async fn enrich_deployment_resources(
    state: &AppState,
    deployments: &mut [CoolifyDeploymentResponse],
) {
    for deployment in deployments {
        match InfrastructureRepository::latest_deployment_sample(&state.pool, &deployment.uuid)
            .await
        {
            Ok(Some(sample)) => {
                deployment.runtime_sampled_at = Some(sample.sampled_at);
                deployment.cpu_percent = sample.cpu_percent;
                deployment.ram_used_mb = sample.ram_used_mb;
                deployment.ram_limit_mb = sample.ram_limit_mb;
                deployment.storage_used_mb = sample.disk_used_mb.and_then(f64_to_i64_rounded);
                deployment.runtime_site_cpu_limit_cores = sample.site_cpu_limit_cores;
                deployment.runtime_site_ram_limit_mb = sample.site_ram_limit_mb;
                deployment.runtime_db_cpu_limit_cores = sample.db_cpu_limit_cores;
                deployment.runtime_db_ram_limit_mb = sample.db_ram_limit_mb;
                deployment.runtime_ssh_cpu_limit_cores = sample.ssh_cpu_limit_cores;
                deployment.runtime_ssh_ram_limit_mb = sample.ssh_ram_limit_mb;
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(
                "[deployments] No se pudo leer snapshot para {}: {error}",
                deployment.uuid
            ),
        }
    }
}

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
pub(super) async fn list_deployments(
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

/* [165A-4] Permite limpiar despliegues huérfanos desde el panel admin.
 * Solo borra stacks sin vínculo en BD para evitar desalinear suscripciones reales.
 * [245A-8] Ahora resuelve el runtime antes de borrar para soportar coexistencia
 * entre Coolify legacy y runtime lightweight sin mezclar identidades. */
#[utoipa::path(
    delete,
    path = "/api/hosting/deployments/{deployment_uuid}",
    params(("deployment_uuid" = String, Path, description = "UUID del despliegue en Coolify")),
    responses(
        (status = 204, description = "Despliegue eliminado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Despliegue no encontrado"),
        (status = 409, description = "El despliegue ya está vinculado a una suscripción"),
        (status = 503, description = "Runtime no configurado o no disponible"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
#[allow(clippy::too_many_lines)]
pub(super) async fn delete_deployment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(deployment_uuid): Path<String>,
) -> Result<StatusCode, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let subscriptions = HostingRepository::list_all(&state.pool).await?;
    let located = locate_runtime_deployment(&state, &deployment_uuid).await?;

    let can_link_by_name = located
        .deployment_name_counts
        .get(&runtime_link_key(
            located.runtime_kind,
            &located.target_name,
        ))
        .copied()
        .unwrap_or(0)
        <= 1;
    let linked_subscription = subscriptions.iter().find(|subscription| {
        HostingRuntimeKind::from_persisted(&subscription.runtime_kind) == located.runtime_kind
            && (subscription.deployment_id_or_legacy() == Some(deployment_uuid.as_str())
                || (can_link_by_name
                    && subscription.coolify_site_name.as_deref()
                        == Some(located.target_name.as_str())))
    });

    if let Some(subscription) = linked_subscription {
        return Err(AppError::Conflict(format!(
            "El despliegue {} ya está vinculado a la suscripción {}. Elimínalo desde la suscripción para no dejar datos huérfanos.",
            located.target_name, subscription.id
        )));
    }

    HostingRuntimeService::delete_deployment(
        &state.http_client,
        located.target_config,
        Some(located.runtime_kind),
        &deployment_uuid,
        true,
    )
    .await?;
    invalidate_deployments_cache(Some(&deployment_uuid)).await;
    tracing::info!(
        "[deployments] Despliegue huérfano {} ({}) eliminado desde el panel admin.",
        located.target_name,
        deployment_uuid
    );

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn sample_subscription(
        runtime_kind: &str,
        status: &str,
        deployment_id: Option<&str>,
        coolify_site_name: Option<&str>,
        server_ip: Option<&str>,
    ) -> HostingSubscription {
        let now = Utc::now();

        HostingSubscription {
            id: Uuid::new_v4(),
            user_id: None,
            client_name: "Cliente Test".to_string(),
            client_email: "test@example.com".to_string(),
            plan: "normal-mini".to_string(),
            domain: Some("example.test".to_string()),
            domain_verification_status: "pending".to_string(),
            domain_verification_token: None,
            domain_verified_at: None,
            runtime_kind: runtime_kind.to_string(),
            deployment_id: deployment_id.map(str::to_string),
            coolify_site_name: coolify_site_name.map(str::to_string),
            status: status.to_string(),
            stripe_subscription_id: None,
            monthly_price_cents: 1000,
            storage_limit_mb: 1024,
            server_uuid: None,
            server_ip: server_ip.map(str::to_string),
            sftp_user: None,
            sftp_password: None,
            sftp_port: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn build_failed_runtime_fallback_batches_filters_runtime_status_and_target() {
        let failed_lookups = vec![FailedRuntimeLookup {
            fallback_label: "VPS2".to_string(),
            runtime_kind: HostingRuntimeKind::Coolify,
            target_server_ip: Some("173.249.50.44".to_string()),
        }];
        let subscriptions = vec![
            sample_subscription(
                "coolify",
                "active",
                Some("dep-ok"),
                Some("hosting-ok"),
                Some("173.249.50.44"),
            ),
            sample_subscription(
                "coolify",
                "cancelled",
                Some("dep-cancelled"),
                Some("hosting-cancelled"),
                Some("173.249.50.44"),
            ),
            sample_subscription(
                "coolify",
                "active",
                Some("dep-other-ip"),
                Some("hosting-other-ip"),
                Some("66.94.100.241"),
            ),
            sample_subscription(
                "lightweight",
                "active",
                Some("dep-lightweight"),
                Some("hosting-lightweight"),
                Some("173.249.50.44"),
            ),
            sample_subscription(
                "coolify",
                "active",
                None,
                Some("hosting-without-id"),
                Some("173.249.50.44"),
            ),
        ];

        let batches = build_failed_runtime_fallback_batches(&subscriptions, &failed_lookups);

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].fallback_label, "VPS2");
        assert_eq!(batches[0].services.len(), 1);
        assert_eq!(batches[0].services[0].deployment_id, "dep-ok");
        assert_eq!(batches[0].services[0].name, "hosting-ok");
    }

    #[test]
    fn build_failed_runtime_fallback_batches_accepts_missing_server_ip_if_runtime_failure_is_unique(
    ) {
        let failed_lookups = vec![FailedRuntimeLookup {
            fallback_label: "VPS2".to_string(),
            runtime_kind: HostingRuntimeKind::Coolify,
            target_server_ip: Some("173.249.50.44".to_string()),
        }];
        let subscriptions = vec![sample_subscription(
            "coolify",
            "active",
            Some("dep-without-ip"),
            Some("hosting-without-ip"),
            None,
        )];

        let batches = build_failed_runtime_fallback_batches(&subscriptions, &failed_lookups);

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].services.len(), 1);
        assert_eq!(batches[0].services[0].deployment_id, "dep-without-ip");
    }
}
