use std::collections::{HashMap, HashSet};

use crate::errors::AppError;
use crate::models::CoolifyDeploymentResponse;
use crate::services::infrastructure::coolify_server_targets;
use crate::services::{
    CoolifyConfig, HostingRuntimeDeploymentSummary, HostingRuntimeKind, HostingRuntimeService,
};
use crate::AppState;

pub(super) struct PendingRuntimeDeployments {
    pub fallback_label: String,
    pub services: Vec<HostingRuntimeDeploymentSummary>,
}

#[derive(Debug, Clone)]
pub(super) struct FailedRuntimeLookup {
    pub fallback_label: String,
    pub runtime_kind: HostingRuntimeKind,
    pub target_server_ip: Option<String>,
}

pub(super) struct DeploymentBatchCollection {
    pub pending_batches: Vec<PendingRuntimeDeployments>,
    pub failed_lookups: Vec<FailedRuntimeLookup>,
}

pub(super) struct LocatedRuntimeDeployment<'a> {
    pub target_config: Option<&'a CoolifyConfig>,
    pub target_name: String,
    pub runtime_kind: HostingRuntimeKind,
    pub deployment_name_counts: HashMap<String, usize>,
}

pub(super) type SubscriptionLookup<'a> = HashMap<String, &'a crate::models::HostingSubscription>;

pub(super) struct DeploymentsCacheEntry {
    pub fetched_at: std::time::Instant,
    pub deployments: Vec<CoolifyDeploymentResponse>,
}

static DEPLOYMENTS_CACHE: std::sync::OnceLock<tokio::sync::RwLock<DeploymentsCacheEntry>> =
    std::sync::OnceLock::new();

fn stale_cache_instant() -> std::time::Instant {
    std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(9999))
        .unwrap_or_else(std::time::Instant::now)
}

pub(super) fn deployments_cache() -> &'static tokio::sync::RwLock<DeploymentsCacheEntry> {
    DEPLOYMENTS_CACHE.get_or_init(|| {
        tokio::sync::RwLock::new(DeploymentsCacheEntry {
            fetched_at: stale_cache_instant(),
            deployments: Vec::new(),
        })
    })
}

pub(super) async fn invalidate_deployments_cache(deployment_uuid: Option<&str>) {
    let Some(cache) = DEPLOYMENTS_CACHE.get() else {
        return;
    };

    let mut cache_guard = cache.write().await;
    if let Some(deployment_uuid) = deployment_uuid {
        cache_guard
            .deployments
            .retain(|deployment| deployment.uuid != deployment_uuid);
    } else {
        cache_guard.deployments.clear();
    }
    cache_guard.fetched_at = stale_cache_instant();
}

pub(super) fn runtime_link_key(runtime_kind: HostingRuntimeKind, identifier: &str) -> String {
    format!("{}::{identifier}", runtime_kind.as_str())
}

pub(super) fn runtime_link_key_for_subscription(
    subscription: &crate::models::HostingSubscription,
    identifier: &str,
) -> String {
    runtime_link_key(
        HostingRuntimeKind::from_persisted(&subscription.runtime_kind),
        identifier,
    )
}

fn is_generic_server_name(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "localhost" | "127.0.0.1" | "::1" | "local"
    )
}

pub(super) fn resolve_server_label(
    service: &HostingRuntimeDeploymentSummary,
    fallback_label: &str,
) -> String {
    service
        .target_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !is_generic_server_name(value))
        .unwrap_or(fallback_label)
        .to_string()
}

pub(super) fn duplicate_name_keys(batches: &[PendingRuntimeDeployments]) -> HashSet<String> {
    let mut counts = HashMap::new();

    for batch in batches {
        for service in &batch.services {
            *counts
                .entry(runtime_link_key(service.runtime_kind, &service.name))
                .or_insert(0usize) += 1;
        }
    }

    counts
        .into_iter()
        .filter_map(|(key, count)| (count > 1).then_some(key))
        .collect()
}

pub(super) fn build_subscription_lookups(
    subscriptions: &[crate::models::HostingSubscription],
) -> (SubscriptionLookup<'_>, SubscriptionLookup<'_>) {
    let subscriptions_by_uuid = subscriptions
        .iter()
        .filter_map(|subscription| {
            subscription.deployment_id_or_legacy().map(|deployment_id| {
                (
                    runtime_link_key_for_subscription(subscription, deployment_id),
                    subscription,
                )
            })
        })
        .collect();
    let subscriptions_by_name = subscriptions
        .iter()
        .filter_map(|subscription| {
            subscription.coolify_site_name.as_deref().map(|site_name| {
                (
                    runtime_link_key_for_subscription(subscription, site_name),
                    subscription,
                )
            })
        })
        .collect();

    (subscriptions_by_uuid, subscriptions_by_name)
}

pub(super) async fn collect_pending_deployment_batches(
    state: &AppState,
) -> DeploymentBatchCollection {
    let mut pending_batches = Vec::new();
    let mut failed_lookups = Vec::new();

    for target in coolify_server_targets(
        state.coolify_config_vps1.as_ref(),
        state.coolify_config.as_ref(),
    ) {
        let target_label = target.label.clone();
        tracing::info!("[deployments] Consultando {} en Coolify...", target_label);
        match HostingRuntimeService::list_deployments(
            &state.http_client,
            Some(target.config),
            Some(HostingRuntimeKind::Coolify),
        )
        .await
        {
            Ok(services) => {
                tracing::info!(
                    "[deployments] {} devolvió {} servicios",
                    target_label,
                    services.len()
                );
                pending_batches.push(PendingRuntimeDeployments {
                    fallback_label: target_label,
                    services,
                });
            }
            Err(error) => {
                tracing::warn!("[deployments] Error listando {}: {error}", target_label);
                failed_lookups.push(FailedRuntimeLookup {
                    fallback_label: target_label,
                    runtime_kind: HostingRuntimeKind::Coolify,
                    target_server_ip: Some(target.config.server_ip.clone()),
                });
            }
        }
    }

    if HostingRuntimeService::lightweight_manager_configured() {
        tracing::info!("[deployments] Consultando runtime ligero...");
        match HostingRuntimeService::list_deployments(
            &state.http_client,
            None,
            Some(HostingRuntimeKind::Lightweight),
        )
        .await
        {
            Ok(services) => {
                tracing::info!(
                    "[deployments] Runtime ligero devolvió {} servicios",
                    services.len()
                );
                pending_batches.push(PendingRuntimeDeployments {
                    fallback_label: "Runtime ligero".to_string(),
                    services,
                });
            }
            Err(error) => {
                tracing::warn!("[deployments] Error listando runtime ligero: {error}");
                failed_lookups.push(FailedRuntimeLookup {
                    fallback_label: "Runtime ligero".to_string(),
                    runtime_kind: HostingRuntimeKind::Lightweight,
                    target_server_ip: None,
                });
            }
        }
    }

    DeploymentBatchCollection {
        pending_batches,
        failed_lookups,
    }
}

pub(super) async fn locate_runtime_deployment<'a>(
    state: &'a AppState,
    deployment_uuid: &str,
) -> Result<LocatedRuntimeDeployment<'a>, AppError> {
    let targets = coolify_server_targets(
        state.coolify_config_vps1.as_ref(),
        state.coolify_config.as_ref(),
    );

    if targets.is_empty() && !HostingRuntimeService::lightweight_manager_configured() {
        return Err(AppError::ServiceUnavailable(
            "No hay runtimes configurados para eliminar despliegues".into(),
        ));
    }

    let mut lookup_failed = false;
    let mut target_config: Option<&CoolifyConfig> = None;
    let mut target_name: Option<String> = None;
    let mut runtime_kind: Option<HostingRuntimeKind> = None;
    let mut deployment_name_counts = HashMap::new();

    for target in &targets {
        match HostingRuntimeService::list_deployments(
            &state.http_client,
            Some(target.config),
            Some(HostingRuntimeKind::Coolify),
        )
        .await
        {
            Ok(services) => {
                for service in services {
                    *deployment_name_counts
                        .entry(runtime_link_key(service.runtime_kind, &service.name))
                        .or_insert(0usize) += 1;

                    if service.deployment_id == deployment_uuid {
                        target_name = Some(service.name);
                        target_config = Some(target.config);
                        runtime_kind = Some(HostingRuntimeKind::Coolify);
                    }
                }
            }
            Err(error) => {
                lookup_failed = true;
                tracing::warn!(
                    "[deployments] Error buscando despliegue {} en {}: {}",
                    deployment_uuid,
                    target.label,
                    error
                );
            }
        }
    }

    if runtime_kind.is_none() && HostingRuntimeService::lightweight_manager_configured() {
        match HostingRuntimeService::list_deployments(
            &state.http_client,
            None,
            Some(HostingRuntimeKind::Lightweight),
        )
        .await
        {
            Ok(services) => {
                for service in services {
                    *deployment_name_counts
                        .entry(runtime_link_key(service.runtime_kind, &service.name))
                        .or_insert(0usize) += 1;

                    if service.deployment_id == deployment_uuid {
                        target_name = Some(service.name);
                        runtime_kind = Some(HostingRuntimeKind::Lightweight);
                    }
                }
            }
            Err(error) => {
                lookup_failed = true;
                tracing::warn!(
                    "[deployments] Error buscando despliegue {} en runtime ligero: {}",
                    deployment_uuid,
                    error
                );
            }
        }
    }

    match (runtime_kind, target_name) {
        (Some(runtime_kind), Some(target_name)) => Ok(LocatedRuntimeDeployment {
            target_config,
            target_name,
            runtime_kind,
            deployment_name_counts,
        }),
        _ if lookup_failed => Err(AppError::ServiceUnavailable(
            "No se pudo consultar la infraestructura para ubicar el despliegue".into(),
        )),
        _ => Err(AppError::NotFound(
            "Despliegue no encontrado en la infraestructura".into(),
        )),
    }
}
