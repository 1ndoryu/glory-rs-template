/* [01AA-4-F3] Recolección y persistencia de muestras (era parte de
 * services/infrastructure_metrics.rs). */

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use sqlx::PgPool;

use super::ssh::{
    fetch_server_snapshot, fetch_service_storage_overrides, secret_ref_for, ssh_secret_ref_for,
    storage_probe_targets_for_service,
};
use super::types::{
    f64_to_i32_rounded, f64_to_i64_rounded, i64_to_f64, CpuCounters, DeploymentContainerRole,
    DeploymentRuntimeLimits, ServerSshSnapshot, ServiceStorageProbe, CPU_HISTORY, STORAGE_HISTORY,
};
use crate::models::HostingSubscription;
/* [07AA-7] Import sin uso (warning rustc al validar el bloque). */
use crate::repositories::{
    BandwidthSnapshotInput, ConfiguredServerInput, InfrastructureRepository,
    InfrastructureServerRecord, ResourceSampleInput,
};
use crate::services::coolify::{CoolifyConfig, CoolifyServiceSummary};
use crate::services::docker_stats::ContainerStats;
use crate::services::CoolifyService;

#[must_use]
fn current_hourly_storage_window() -> bool {
    let ten_minute_window = Utc::now().timestamp() / 600;
    ten_minute_window % 6 == 0
}

async fn should_collect_storage(server_ip: &str) -> bool {
    if current_hourly_storage_window() {
        return true;
    }

    let history = STORAGE_HISTORY.get_or_init(|| tokio::sync::RwLock::new(HashSet::new()));
    let guard = history.read().await;
    !guard.contains(server_ip)
}

async fn mark_storage_collected(server_ip: &str) {
    let history = STORAGE_HISTORY.get_or_init(|| tokio::sync::RwLock::new(HashSet::new()));
    history.write().await.insert(server_ip.to_string());
}

async fn compute_server_cpu_percent(server_ip: &str, counters: Option<CpuCounters>) -> Option<f64> {
    let counters = counters?;
    let history = CPU_HISTORY.get_or_init(|| tokio::sync::RwLock::new(HashMap::new()));
    let mut guard = history.write().await;
    let previous = guard.insert(server_ip.to_string(), counters)?;
    let total_delta = counters.total - previous.total;
    let idle_delta = counters.idle - previous.idle;
    if total_delta <= 0.0 || idle_delta < 0.0 {
        return None;
    }
    Some(((1.0 - idle_delta / total_delta) * 100.0).clamp(0.0, 100.0))
}

fn matching_containers<'a>(
    containers: &'a [ContainerStats],
    deployment_uuid: &str,
    service_name: &str,
) -> Vec<&'a ContainerStats> {
    containers
        .iter()
        .filter(|container| {
            container.name.contains(deployment_uuid) || container.name.contains(service_name)
        })
        .collect()
}

fn deployment_container_role(container_name: &str) -> Option<DeploymentContainerRole> {
    let normalized = container_name.trim().to_ascii_lowercase();

    if normalized.contains("mariadb")
        || normalized.contains("mysql")
        || normalized.contains("postgres")
    {
        return Some(DeploymentContainerRole::Db);
    }

    if normalized.contains("ssh") || normalized.contains("sftp") {
        return Some(DeploymentContainerRole::Ssh);
    }

    if normalized.contains("wordpress") || normalized.contains("site") {
        return Some(DeploymentContainerRole::Site);
    }

    None
}

fn accumulate_limit(target: &mut Option<f64>, value: Option<f64>) {
    let Some(value) = value else {
        return;
    };

    *target = Some(target.unwrap_or(0.0) + value);
}

fn aggregate_runtime_limits(
    containers: &[&ContainerStats],
    container_runtime_limits: &HashMap<String, super::types::ContainerRuntimeLimits>,
) -> DeploymentRuntimeLimits {
    let mut aggregated = DeploymentRuntimeLimits::default();

    for container in containers {
        let Some(role) = deployment_container_role(&container.name) else {
            continue;
        };
        let Some(limits) = container_runtime_limits.get(&container.name) else {
            continue;
        };

        match role {
            DeploymentContainerRole::Site => {
                accumulate_limit(&mut aggregated.site_cpu_limit_cores, limits.cpu_limit_cores);
                accumulate_limit(&mut aggregated.site_ram_limit_mb, limits.mem_limit_mb);
            }
            DeploymentContainerRole::Db => {
                accumulate_limit(&mut aggregated.db_cpu_limit_cores, limits.cpu_limit_cores);
                accumulate_limit(&mut aggregated.db_ram_limit_mb, limits.mem_limit_mb);
            }
            DeploymentContainerRole::Ssh => {
                accumulate_limit(&mut aggregated.ssh_cpu_limit_cores, limits.cpu_limit_cores);
                accumulate_limit(&mut aggregated.ssh_ram_limit_mb, limits.mem_limit_mb);
            }
        }
    }

    aggregated
}

fn total_runtime_ram_limit_mb(limits: &DeploymentRuntimeLimits) -> Option<f64> {
    let values = [
        limits.site_ram_limit_mb,
        limits.db_ram_limit_mb,
        limits.ssh_ram_limit_mb,
    ];

    let mut total = 0.0;
    let mut has_any = false;

    for value in values.into_iter().flatten() {
        total += value;
        has_any = true;
    }

    has_any.then_some(total)
}

async fn record_bandwidth_delta(
    pool: &PgPool,
    server: &InfrastructureServerRecord,
    subscription: &HostingSubscription,
    deployment_uuid: &str,
    sampled_at: chrono::DateTime<Utc>,
    containers: &[&ContainerStats],
) -> Result<(), crate::errors::AppError> {
    let net_input_mb: f64 = containers
        .iter()
        .map(|container| container.net_input_mb)
        .sum();
    let net_output_mb: f64 = containers
        .iter()
        .map(|container| container.net_output_mb)
        .sum();

    if let Some(previous) =
        InfrastructureRepository::find_bandwidth_snapshot(pool, subscription.id, deployment_uuid)
            .await?
    {
        let rx_delta_mb = net_input_mb - previous.net_input_mb;
        let tx_delta_mb = net_output_mb - previous.net_output_mb;
        if rx_delta_mb >= 0.0 || tx_delta_mb >= 0.0 {
            InfrastructureRepository::add_bandwidth_delta(
                pool,
                subscription.id,
                f64_to_i64_rounded(rx_delta_mb.max(0.0) * 1_000_000.0),
                f64_to_i64_rounded(tx_delta_mb.max(0.0) * 1_000_000.0),
            )
            .await?;
        }
    }

    InfrastructureRepository::upsert_bandwidth_snapshot(
        pool,
        BandwidthSnapshotInput {
            subscription_id: subscription.id,
            deployment_uuid,
            server_id: server.id,
            net_input_mb,
            net_output_mb,
            sampled_at,
        },
    )
    .await
}

pub(crate) async fn sample_target(
    pool: &PgPool,
    http_client: &reqwest::Client,
    label: String,
    config: CoolifyConfig,
    subscriptions_by_uuid: &HashMap<String, HostingSubscription>,
    subscriptions_by_name: &HashMap<String, HostingSubscription>,
) -> Result<(), crate::errors::AppError> {
    let server = InfrastructureRepository::upsert_configured_server(
        pool,
        ConfiguredServerInput {
            label: &label,
            config: &config,
            secret_ref: secret_ref_for(&label),
            ssh_secret_ref: ssh_secret_ref_for(&label, &config),
        },
    )
    .await?;
    InfrastructureRepository::ensure_capacity_row(pool, &server).await?;

    let services = CoolifyService::list_services(http_client, &config)
        .await
        .map_err(|error| {
            tracing::warn!("[infra-metrics] Coolify {label} no disponible: {error}");
            crate::errors::AppError::ServiceUnavailable(error.to_string())
        })?;

    let Some(ssh_key_path) = config.ssh_key_path.as_deref() else {
        tracing::warn!(
            "[infra-metrics] {label} sin SSH key; se sincroniza inventario sin muestras"
        );
        return Ok(());
    };
    let include_storage = should_collect_storage(&config.server_ip).await;
    let sampled_at = Utc::now();
    let mut snapshot = fetch_server_snapshot(&config.server_ip, ssh_key_path, include_storage)
        .await
        .map_err(|error| {
            tracing::warn!("[infra-metrics] {error}");
            crate::errors::AppError::ServiceUnavailable(error)
        })?;

    if include_storage {
        collect_storage_overrides(
            &label,
            &config.server_ip,
            ssh_key_path,
            &services,
            &mut snapshot,
            subscriptions_by_uuid,
            subscriptions_by_name,
        )
        .await;
    }

    let cpu_percent = compute_server_cpu_percent(&config.server_ip, snapshot.cpu_counters).await;
    InfrastructureRepository::insert_sample(
        pool,
        ResourceSampleInput {
            entity_kind: "server",
            server_id: server.id,
            deployment_uuid: None,
            sampled_at,
            cpu_percent,
            ram_used_mb: snapshot.ram_used_mb,
            ram_limit_mb: snapshot.ram_limit_mb,
            disk_used_mb: snapshot.disk_used_mb,
            disk_limit_mb: snapshot.disk_limit_mb,
            site_cpu_limit_cores: None,
            site_ram_limit_mb: None,
            db_cpu_limit_cores: None,
            db_ram_limit_mb: None,
            ssh_cpu_limit_cores: None,
            ssh_ram_limit_mb: None,
        },
    )
    .await?;

    if let Some(server_uuid) = server.coolify_server_uuid.as_deref() {
        InfrastructureRepository::update_capacity_specs(
            pool,
            server_uuid,
            snapshot.cpu_cores,
            snapshot.ram_limit_mb.map(f64_to_i32_rounded),
            snapshot.disk_limit_mb.map(f64_to_i32_rounded),
        )
        .await?;
    }

    for service in services {
        record_deployment_sample(
            pool,
            &server,
            &service,
            &snapshot,
            sampled_at,
            subscriptions_by_uuid,
            subscriptions_by_name,
        )
        .await?;
    }

    Ok(())
}

async fn collect_storage_overrides(
    label: &str,
    server_ip: &str,
    ssh_key_path: &str,
    services: &[CoolifyServiceSummary],
    snapshot: &mut ServerSshSnapshot,
    subscriptions_by_uuid: &HashMap<String, HostingSubscription>,
    subscriptions_by_name: &HashMap<String, HostingSubscription>,
) {
    let storage_probes: Vec<ServiceStorageProbe> = services
        .iter()
        .flat_map(|service| {
            let subscription = subscriptions_by_uuid
                .get(&service.uuid)
                .or_else(|| subscriptions_by_name.get(&service.name));
            storage_probe_targets_for_service(service, subscription)
        })
        .collect();

    match fetch_service_storage_overrides(server_ip, ssh_key_path, &storage_probes).await {
        Ok(storage_overrides) => {
            for (deployment_uuid, used_mb) in storage_overrides {
                snapshot.storage_by_uuid.insert(deployment_uuid, used_mb);
            }
        }
        Err(error) => tracing::warn!("[infra-metrics] {error}"),
    }

    if snapshot.storage_by_uuid.is_empty() {
        tracing::warn!(
            "[infra-metrics] {label} sin lecturas de storage; se reintentará en el próximo ciclo"
        );
        return;
    }

    mark_storage_collected(server_ip).await;
}

async fn record_deployment_sample(
    pool: &PgPool,
    server: &InfrastructureServerRecord,
    service: &CoolifyServiceSummary,
    snapshot: &ServerSshSnapshot,
    sampled_at: chrono::DateTime<Utc>,
    subscriptions_by_uuid: &HashMap<String, HostingSubscription>,
    subscriptions_by_name: &HashMap<String, HostingSubscription>,
) -> Result<(), crate::errors::AppError> {
    let containers = matching_containers(&snapshot.containers, &service.uuid, &service.name);
    if containers.is_empty() && !snapshot.storage_by_uuid.contains_key(&service.uuid) {
        return Ok(());
    }

    let subscription = subscriptions_by_uuid
        .get(&service.uuid)
        .or_else(|| subscriptions_by_name.get(&service.name));

    let cpu_percent = containers
        .iter()
        .map(|container| container.cpu_percent)
        .sum();
    let ram_used_mb = containers
        .iter()
        .map(|container| container.mem_used_mb)
        .sum();
    let runtime_limits = aggregate_runtime_limits(&containers, &snapshot.container_runtime_limits);
    let ram_limit_mb = total_runtime_ram_limit_mb(&runtime_limits);
    let storage_used_mb = snapshot
        .storage_by_uuid
        .get(&service.uuid)
        .map(|value| i64_to_f64(*value));
    let storage_limit_mb = subscription.map(|sub| f64::from(sub.storage_limit_mb));

    InfrastructureRepository::insert_sample(
        pool,
        ResourceSampleInput {
            entity_kind: "deployment",
            server_id: server.id,
            deployment_uuid: Some(&service.uuid),
            sampled_at,
            cpu_percent: Some(cpu_percent),
            ram_used_mb: Some(ram_used_mb),
            ram_limit_mb,
            disk_used_mb: storage_used_mb,
            disk_limit_mb: storage_limit_mb,
            site_cpu_limit_cores: runtime_limits.site_cpu_limit_cores,
            site_ram_limit_mb: runtime_limits.site_ram_limit_mb,
            db_cpu_limit_cores: runtime_limits.db_cpu_limit_cores,
            db_ram_limit_mb: runtime_limits.db_ram_limit_mb,
            ssh_cpu_limit_cores: runtime_limits.ssh_cpu_limit_cores,
            ssh_ram_limit_mb: runtime_limits.ssh_ram_limit_mb,
        },
    )
    .await?;

    if let Some(subscription) = subscription {
        record_bandwidth_delta(
            pool,
            server,
            subscription,
            &service.uuid,
            sampled_at,
            &containers,
        )
        .await?;
    }

    Ok(())
}
