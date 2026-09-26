/* [225A-4] Sampler de recursos de infraestructura.
 * Coolify sigue siendo el inventario autoritativo; este loop toma promedios
 * aproximados por SSH cada 10 minutos y guarda snapshots para que el panel no
 * dispare SSH en cada render. */

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::models::HostingSubscription;
use crate::repositories::{
    BandwidthSnapshotInput, ConfiguredServerInput, HostingRepository, InfrastructureRepository,
    InfrastructureServerRecord, ResourceSampleInput,
};
use crate::services::coolify::{CoolifyConfig, CoolifyServiceSummary};
use crate::services::docker_stats::storage_targets;
use crate::services::docker_stats::{parse_docker_stats_public, ContainerStats};
use crate::services::infrastructure::coolify_server_targets;
use crate::services::CoolifyService;

const SAMPLER_INTERVAL: Duration = Duration::from_mins(10);
const SAMPLER_STARTUP_RETRY_INTERVAL: Duration = Duration::from_secs(45);
/* [235A-4] En VPS1 el snapshot base (docker stats + du de volúmenes) roza los 20s
 * cuando el host está cargado. Con 20s exactos el sampler cae en timeout y deja
 * `disk_used_mb = NULL` aunque la lógica de storage sea correcta. */
const SSH_TIMEOUT: Duration = Duration::from_secs(40);
const STORAGE_PROBE_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Copy)]
struct CpuCounters {
    total: f64,
    idle: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct ContainerRuntimeLimits {
    cpu_limit_cores: Option<f64>,
    mem_limit_mb: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default)]
struct DeploymentRuntimeLimits {
    site_cpu_limit_cores: Option<f64>,
    site_ram_limit_mb: Option<f64>,
    db_cpu_limit_cores: Option<f64>,
    db_ram_limit_mb: Option<f64>,
    ssh_cpu_limit_cores: Option<f64>,
    ssh_ram_limit_mb: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
enum DeploymentContainerRole {
    Site,
    Db,
    Ssh,
}

#[derive(Debug, Clone, Default)]
struct ServerSshSnapshot {
    cpu_counters: Option<CpuCounters>,
    cpu_cores: Option<f64>,
    ram_used_mb: Option<f64>,
    ram_limit_mb: Option<f64>,
    disk_used_mb: Option<f64>,
    disk_limit_mb: Option<f64>,
    containers: Vec<ContainerStats>,
    container_runtime_limits: HashMap<String, ContainerRuntimeLimits>,
    storage_by_uuid: HashMap<String, i64>,
}

#[derive(Debug, Clone)]
struct ServiceStorageProbe {
    deployment_uuid: String,
    container_name: String,
    path: &'static str,
}

static CPU_HISTORY: OnceLock<RwLock<HashMap<String, CpuCounters>>> = OnceLock::new();
static STORAGE_HISTORY: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

#[must_use]
fn secret_ref_for(label: &str) -> &'static str {
    if label.to_ascii_lowercase().contains("principal") || label.contains("VPS1") {
        "COOLIFY_VPS1_API_TOKEN"
    } else {
        "COOLIFY_API_TOKEN"
    }
}

#[must_use]
fn ssh_secret_ref_for(label: &str, config: &CoolifyConfig) -> Option<&'static str> {
    config.ssh_key_path.as_ref()?;
    if label.to_ascii_lowercase().contains("principal") || label.contains("VPS1") {
        Some("COOLIFY_VPS1_SSH_KEY_PATH")
    } else {
        Some("COOLIFY_SSH_KEY_PATH")
    }
}

#[must_use]
fn current_hourly_storage_window() -> bool {
    let ten_minute_window = Utc::now().timestamp() / 600;
    ten_minute_window % 6 == 0
}

async fn should_collect_storage(server_ip: &str) -> bool {
    if current_hourly_storage_window() {
        return true;
    }

    let history = STORAGE_HISTORY.get_or_init(|| RwLock::new(HashSet::new()));
    let guard = history.read().await;
    !guard.contains(server_ip)
}

async fn mark_storage_collected(server_ip: &str) {
    let history = STORAGE_HISTORY.get_or_init(|| RwLock::new(HashSet::new()));
    history.write().await.insert(server_ip.to_string());
}

fn sampler_command(include_storage: bool) -> String {
    let mut command = concat!(
        "printf '__CPU__\\n'; ",
        "awk '/^cpu /{print $2+$3+$4+$5+$6+$7+$8, $5+$6}' /proc/stat; ",
        "printf '__NPROC__\\n'; nproc 2>/dev/null || true; ",
        "printf '__MEM__\\n'; free -m | awk '/^Mem:/{print $3, $2}'; ",
        "printf '__DISK__\\n'; df -Pm / | awk 'NR==2{print $3, $2}'; ",
        "printf '__DOCKER__\\n'; ",
        "docker stats --no-stream --format '{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}' 2>/dev/null || true;"
    )
    .to_string();

    command.push_str(
        " printf '__DOCKER_LIMITS__\\n'; docker ps --format '{{.Names}}' 2>/dev/null | while IFS= read -r name; do [ -n \"$name\" ] || continue; printf '%s\\t' \"$name\"; docker inspect --format '{{.HostConfig.NanoCpus}}\\t{{.HostConfig.Memory}}\\t{{.HostConfig.CpuQuota}}\\t{{.HostConfig.CpuPeriod}}' \"$name\" 2>/dev/null || printf '0\\t0\\t0\\t0\\n'; done;",
    );

    if include_storage {
        command.push_str(
            " printf '__STORAGE__\\n'; du -sm /var/lib/docker/volumes/*/_data 2>/dev/null || true;",
        );
    }

    command
}

async fn fetch_server_snapshot(
    server_ip: &str,
    ssh_key_path: &str,
    include_storage: bool,
) -> Result<ServerSshSnapshot, String> {
    let mut command = tokio::process::Command::new("ssh");
    command
        .args([
            "-i",
            ssh_key_path,
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=8",
            "-o",
            "BatchMode=yes",
            &format!("root@{server_ip}"),
            &sampler_command(include_storage),
        ])
        .stdin(Stdio::null());

    let output = tokio::time::timeout(SSH_TIMEOUT, command.output())
        .await
        .map_err(|_| format!("Timeout SSH sampler para {server_ip}"))?
        .map_err(|error| format!("SSH sampler fallo para {server_ip}: {error}"))?;

    if !output.status.success() && output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("SSH sampler exit {}: {stderr}", output.status));
    }

    Ok(parse_sampler_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_sampler_output(output: &str) -> ServerSshSnapshot {
    let mut snapshot = ServerSshSnapshot::default();
    let mut section = "";
    let mut docker_lines = Vec::new();

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "__CPU__" | "__NPROC__" | "__MEM__" | "__DISK__" | "__DOCKER__"
            | "__DOCKER_LIMITS__" | "__STORAGE__" => {
                section = line;
                continue;
            }
            _ => {}
        }

        match section {
            "__CPU__" => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let (Ok(total), Ok(idle)) = (parts[0].parse(), parts[1].parse()) {
                        snapshot.cpu_counters = Some(CpuCounters { total, idle });
                    }
                }
            }
            "__NPROC__" => snapshot.cpu_cores = line.parse::<f64>().ok(),
            "__MEM__" => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    snapshot.ram_used_mb = parts[0].parse::<f64>().ok();
                    snapshot.ram_limit_mb = parts[1].parse::<f64>().ok();
                }
            }
            "__DISK__" => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    snapshot.disk_used_mb = parts[0].parse::<f64>().ok();
                    snapshot.disk_limit_mb = parts[1].parse::<f64>().ok();
                }
            }
            "__DOCKER__" => docker_lines.push(line.to_string()),
            "__DOCKER_LIMITS__" => {
                parse_container_runtime_limit_line(line, &mut snapshot.container_runtime_limits);
            }
            "__STORAGE__" => parse_storage_line(line, &mut snapshot.storage_by_uuid),
            _ => {}
        }
    }

    snapshot.containers = parse_docker_stats_public(&docker_lines.join("\n"));
    snapshot
}

fn parse_container_runtime_limit_line(
    line: &str,
    container_runtime_limits: &mut HashMap<String, ContainerRuntimeLimits>,
) {
    /* [265A-2] `docker inspect --format` devuelve `\t` literales en vez de tabs
     * reales. El sampler antepone el nombre con `printf`, así que la salida queda
     * mezclada: primer separador real, resto escapados. Sin normalizar esa forma,
     * el parser pierde todos los runtime limits y el burst nunca encuentra baseline
     * runtime en hostings existentes. */
    let normalized_line = line.replace("\\t", "\t");
    let parts: Vec<&str> = normalized_line.split('\t').collect();
    if parts.len() < 5 {
        return;
    }

    let name = parts[0].trim();
    if name.is_empty() {
        return;
    }

    let nanocpus = parts[1].trim().parse::<i64>().ok();
    let memory_bytes = parts[2].trim().parse::<i64>().ok();
    let cpu_quota = parts[3].trim().parse::<i64>().ok();
    let cpu_period = parts[4].trim().parse::<i64>().ok();

    container_runtime_limits.insert(
        name.to_string(),
        ContainerRuntimeLimits {
            cpu_limit_cores: compute_container_cpu_limit_cores(nanocpus, cpu_quota, cpu_period),
            mem_limit_mb: compute_container_memory_limit_mb(memory_bytes),
        },
    );
}

fn compute_container_cpu_limit_cores(
    nanocpus: Option<i64>,
    cpu_quota: Option<i64>,
    cpu_period: Option<i64>,
) -> Option<f64> {
    /* [265A-5] `docker update --cpu-quota -1` deslimita de verdad el contenedor,
     * pero Docker deja `NanoCpus` stale con el valor viejo del cap. Si el sampler
     * prioriza `NanoCpus`, el backend cree falsamente que sigue en baseline y el
     * modo de contención nunca vuelve a `unlimited`. */
    if cpu_quota.is_some_and(|value| value < 0) {
        return None;
    }

    if let Some(nanocpus) = nanocpus.filter(|value| *value > 0) {
        return Some(i64_to_f64(nanocpus) / 1_000_000_000.0);
    }

    match (cpu_quota, cpu_period) {
        (Some(quota), Some(period)) if quota > 0 && period > 0 => {
            Some(i64_to_f64(quota) / i64_to_f64(period))
        }
        _ => None,
    }
}

fn compute_container_memory_limit_mb(memory_bytes: Option<i64>) -> Option<f64> {
    memory_bytes
        .filter(|value| *value > 0)
        .map(|value| i64_to_f64(value) / 1024.0 / 1024.0)
}

fn parse_storage_line(line: &str, storage_by_uuid: &mut HashMap<String, i64>) {
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 2 {
        return;
    }
    let Ok(mb) = cols[0].parse::<i64>() else {
        return;
    };
    let Some(folder_path) = cols[1].strip_suffix("/_data") else {
        return;
    };
    if let Some(volume_name) = folder_path.split('/').next_back() {
        let uuid = volume_name.split('_').next().unwrap_or(volume_name);
        *storage_by_uuid.entry(uuid.to_string()).or_insert(0) += mb;
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn storage_probe_targets_for_service(
    service: &CoolifyServiceSummary,
    subscription: Option<&HostingSubscription>,
) -> Vec<ServiceStorageProbe> {
    let site_name = subscription
        .and_then(|sub| sub.coolify_site_name.as_deref())
        .unwrap_or(service.name.as_str());

    /* [235A-3] No confiar solo en `subscription.plan` para deducir contenedores de storage.
     * Hay hostings legacy marcados como `normal-*` cuya topología real sigue siendo
     * `wordpress-{uuid}` + `mariadb-{uuid}`. Si limitamos el probe al plan guardado,
     * el sampler vuelve a escribir `disk_used_mb = NULL` aunque el sitio tenga datos. */
    let mut seen = HashSet::new();
    let mut probes = Vec::new();
    let mut push_plan_targets = |plan: &str| {
        for (container_name, path) in storage_targets(site_name, Some(service.uuid.as_str()), plan)
        {
            let dedupe_key = format!("{container_name}\n{path}");
            if seen.insert(dedupe_key) {
                probes.push(ServiceStorageProbe {
                    deployment_uuid: service.uuid.clone(),
                    container_name,
                    path,
                });
            }
        }
    };

    push_plan_targets("normal-unknown");
    push_plan_targets("wp-unknown");

    if let Some(subscription) = subscription {
        push_plan_targets(subscription.plan.as_str());
    }

    probes
}

fn build_service_storage_command(probes: &[ServiceStorageProbe]) -> Option<String> {
    if probes.is_empty() {
        return None;
    }

    let mut command = String::from("set +e");
    for probe in probes {
        let _ = write!(
            command,
            "; if docker inspect {container} >/dev/null 2>&1; then value=$(docker exec {container} du -sm {path} 2>/dev/null | awk '{{print $1}}' || true); case \"$value\" in ''|*[!0-9]* ) ;; *) printf '%s\\t%s\\n' {deployment_uuid} \"$value\" ;; esac; fi",
            container = shell_quote(&probe.container_name),
            path = shell_quote(probe.path),
            deployment_uuid = shell_quote(&probe.deployment_uuid),
        );
    }

    Some(command)
}

fn parse_service_storage_output(output: &str) -> HashMap<String, i64> {
    let mut storage_by_uuid = HashMap::new();

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(deployment_uuid) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next() else {
            continue;
        };
        let Ok(mb) = value.parse::<i64>() else {
            continue;
        };

        *storage_by_uuid
            .entry(deployment_uuid.to_string())
            .or_insert(0) += mb;
    }

    storage_by_uuid
}

async fn fetch_service_storage_overrides(
    server_ip: &str,
    ssh_key_path: &str,
    probes: &[ServiceStorageProbe],
) -> Result<HashMap<String, i64>, String> {
    let Some(command) = build_service_storage_command(probes) else {
        return Ok(HashMap::new());
    };

    let mut ssh_command = tokio::process::Command::new("ssh");
    ssh_command
        .args([
            "-i",
            ssh_key_path,
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=8",
            "-o",
            "BatchMode=yes",
            &format!("root@{server_ip}"),
            &command,
        ])
        .stdin(Stdio::null());

    let output = tokio::time::timeout(STORAGE_PROBE_TIMEOUT, ssh_command.output())
        .await
        .map_err(|_| format!("Timeout SSH storage probe para {server_ip}"))?
        .map_err(|error| format!("SSH storage probe fallo para {server_ip}: {error}"))?;

    if !output.status.success() && output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "SSH storage probe exit {}: {stderr}",
            output.status
        ));
    }

    Ok(parse_service_storage_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn f64_to_i64_rounded(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    format!("{value:.0}").parse::<i64>().unwrap_or(i64::MAX)
}

fn f64_to_i32_rounded(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    format!("{value:.0}").parse::<i32>().unwrap_or(i32::MAX)
}

fn i64_to_f64(value: i64) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0)
}

async fn compute_server_cpu_percent(server_ip: &str, counters: Option<CpuCounters>) -> Option<f64> {
    let counters = counters?;
    let history = CPU_HISTORY.get_or_init(|| RwLock::new(HashMap::new()));
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
    container_runtime_limits: &HashMap<String, ContainerRuntimeLimits>,
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

async fn sample_target(
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

pub async fn sample_infrastructure_once(
    pool: PgPool,
    http_client: reqwest::Client,
    vps1_config: Option<CoolifyConfig>,
    default_config: Option<CoolifyConfig>,
) -> Result<(), crate::errors::AppError> {
    let targets = coolify_server_targets(vps1_config.as_ref(), default_config.as_ref());
    if targets.is_empty() {
        tracing::warn!("[infra-metrics] Coolify no configurado; sampler desactivado");
        return Ok(());
    }

    let subscriptions = HostingRepository::list_all(&pool).await?;
    let subscriptions_by_uuid: HashMap<String, HostingSubscription> = subscriptions
        .iter()
        .filter(|subscription| subscription.is_coolify_runtime())
        .filter_map(|subscription| {
            subscription
                .deployment_id_or_legacy()
                .map(|deployment_id| (deployment_id.to_string(), subscription.clone()))
        })
        .collect();
    let subscriptions_by_name: HashMap<String, HostingSubscription> = subscriptions
        .iter()
        .filter(|subscription| subscription.is_coolify_runtime())
        .filter_map(|subscription| {
            subscription
                .coolify_site_name
                .as_ref()
                .map(|site_name| (site_name.clone(), subscription.clone()))
        })
        .collect();

    let futures = targets.into_iter().map(|target| {
        sample_target(
            &pool,
            &http_client,
            target.label,
            target.config.clone(),
            &subscriptions_by_uuid,
            &subscriptions_by_name,
        )
    });

    for result in futures::future::join_all(futures).await {
        if let Err(error) = result {
            tracing::warn!("[infra-metrics] muestra parcial fallida: {error}");
        }
    }

    InfrastructureRepository::purge_old_samples(&pool).await?;
    Ok(())
}

pub async fn infrastructure_metrics_loop(
    pool: PgPool,
    http_client: reqwest::Client,
    vps1_config: Option<CoolifyConfig>,
    default_config: Option<CoolifyConfig>,
) {
    let mut startup_retry_pending = true;
    loop {
        if let Err(error) = sample_infrastructure_once(
            pool.clone(),
            http_client.clone(),
            vps1_config.clone(),
            default_config.clone(),
        )
        .await
        {
            tracing::warn!("[infra-metrics] ciclo incompleto: {error}");
        }

        let next_interval = if startup_retry_pending {
            startup_retry_pending = false;
            SAMPLER_STARTUP_RETRY_INTERVAL
        } else {
            SAMPLER_INTERVAL
        };
        tokio::time::sleep(next_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_container_runtime_limit_line_accepts_literal_docker_tabs() {
        let mut limits = HashMap::new();

        parse_container_runtime_limit_line(
            "wordpress-v77j8dfkb8rat8mlhzoid2eh\t500000000\\t268435456\\t0\\t0",
            &mut limits,
        );

        let parsed = limits
            .get("wordpress-v77j8dfkb8rat8mlhzoid2eh")
            .expect("runtime limit parsed");

        assert_eq!(parsed.cpu_limit_cores, Some(0.5));
        assert_eq!(parsed.mem_limit_mb, Some(256.0));
    }

    #[test]
    fn compute_container_cpu_limit_cores_treats_negative_quota_as_unlimited() {
        let parsed = compute_container_cpu_limit_cores(Some(500_000_000), Some(-1), Some(0));
        assert_eq!(parsed, None);
    }
}
