/* [01AA-4-F3] SSH + parseo del sampler (era parte de
 * services/infrastructure_metrics.rs). */

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::process::Stdio;

use super::types::{
    i64_to_f64, ContainerRuntimeLimits, ServerSshSnapshot, ServiceStorageProbe, SSH_TIMEOUT,
    STORAGE_PROBE_TIMEOUT,
};
use crate::models::HostingSubscription;
use crate::services::coolify::{CoolifyConfig, CoolifyServiceSummary};
use crate::services::docker_stats::storage_targets;
use crate::services::docker_stats::parse_docker_stats_public;

#[must_use]
pub(crate) fn secret_ref_for(label: &str) -> &'static str {
    if label.to_ascii_lowercase().contains("principal") || label.contains("VPS1") {
        "COOLIFY_VPS1_API_TOKEN"
    } else {
        "COOLIFY_API_TOKEN"
    }
}

#[must_use]
pub(crate) fn ssh_secret_ref_for(label: &str, config: &CoolifyConfig) -> Option<&'static str> {
    config.ssh_key_path.as_ref()?;
    if label.to_ascii_lowercase().contains("principal") || label.contains("VPS1") {
        Some("COOLIFY_VPS1_SSH_KEY_PATH")
    } else {
        Some("COOLIFY_SSH_KEY_PATH")
    }
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

pub(crate) async fn fetch_server_snapshot(
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
    use super::types::CpuCounters;

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

pub(crate) fn storage_probe_targets_for_service(
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

pub(crate) async fn fetch_service_storage_overrides(
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
