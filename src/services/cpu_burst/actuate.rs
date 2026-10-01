use std::collections::HashMap;
use std::process::Stdio;
use std::time::Instant;

use sqlx::PgPool;

use crate::errors::AppError;
use crate::repositories::{CpuBurstCandidate, HostingRepository, InfrastructureRepository};
use crate::services::coolify::CoolifyConfig;
use crate::services::infrastructure::coolify_server_targets;

use super::policy::{baseline_site_limit, desired_cpu_limit, observed_limit_target};
use super::state::{
    state_map, CpuLimitTarget, CpuScalingPolicy, CPU_BURST_INTERVAL, CPU_BURST_LOWER_WINDOW,
    CPU_BURST_RAISE_WINDOW, CPU_EPSILON, CPU_SSH_TIMEOUT,
};

fn ssh_key_for(
    server_ip: &str,
    vps1: Option<&CoolifyConfig>,
    default: Option<&CoolifyConfig>,
) -> Option<String> {
    coolify_server_targets(vps1, default)
        .into_iter()
        .find(|target| server_ip == target.config.server_ip)
        .and_then(|target| target.config.ssh_key_path.clone())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/* [265A-3] Coolify usa el deployment UUID como `com.docker.compose.project` en
 * stacks runtime/legacy. El burst intenta primero ese project y solo cae al
 * slug del sitio cuando realmente coincide con el compose project. */
fn compose_project_candidates(deployment_uuid: &str, site_name: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    for value in [deployment_uuid, site_name] {
        let value = value.trim();
        if value.is_empty() || candidates.iter().any(|existing| existing == value) {
            continue;
        }
        candidates.push(value.to_string());
    }

    candidates
}

async fn run_ssh(server_ip: &str, ssh_key_path: &str, cmd: &str) -> Result<String, String> {
    let mut command = tokio::process::Command::new("ssh");
    command
        .kill_on_drop(true)
        .args([
            "-i",
            ssh_key_path,
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "BatchMode=yes",
            &format!("root@{server_ip}"),
            cmd,
        ])
        .stdin(Stdio::null());

    let output = tokio::time::timeout(CPU_SSH_TIMEOUT, command.output())
        .await
        .map_err(|_| format!("cpu burst ssh timeout {server_ip}"))?
        .map_err(|e| format!("cpu burst ssh error {server_ip}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cpu burst ssh exit {}: {stderr}", output.status));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn cpu_update_script(deployment_uuid: &str, site_name: &str, target: CpuLimitTarget) -> String {
    let projects = compose_project_candidates(deployment_uuid, site_name)
        .into_iter()
        .map(|value| shell_quote(&value))
        .collect::<Vec<_>>()
        .join(" ");
    let update_command = match target {
        CpuLimitTarget::Limited(cpu_cores) => {
            format!("docker update --cpus {cpu_cores:.2} \"$CID\" >/dev/null")
        }
        CpuLimitTarget::Unlimited => "docker update --cpu-quota -1 \"$CID\" >/dev/null".into(),
    };
    format!(
        r#"CID=""
for PROJECT in {projects}; do
    [ -n "$PROJECT" ] || continue
    CID=$(docker compose -p "$PROJECT" ps -q site 2>/dev/null | head -1)
    if [ -z "$CID" ]; then
        CID=$(docker compose -p "$PROJECT" ps -q wordpress 2>/dev/null | head -1)
    fi
    if [ -z "$CID" ]; then
        CID=$(docker compose -p "$PROJECT" ps -q app 2>/dev/null | head -1)
    fi
    if [ -n "$CID" ]; then
        break
    fi
done
if [ -z "$CID" ]; then
  echo "container_not_found"
  exit 10
fi
{update_command}
docker inspect -f '{{{{.Name}}}}' "$CID" 2>/dev/null | sed 's#^/##'"#,
    )
}

async fn apply_site_cpu_limit(
    server_ip: &str,
    ssh_key_path: &str,
    deployment_uuid: &str,
    site_name: &str,
    target: CpuLimitTarget,
) -> Result<String, String> {
    let script = cpu_update_script(deployment_uuid, site_name, target);
    run_ssh(server_ip, ssh_key_path, &script).await
}

async fn apply_target(
    pool: &PgPool,
    candidate: &CpuBurstCandidate,
    ssh_key: &str,
    target: CpuLimitTarget,
) -> bool {
    match apply_site_cpu_limit(
        &candidate.server_ip,
        ssh_key,
        &candidate.deployment_uuid,
        &candidate.coolify_site_name,
        target,
    )
    .await
    {
        Ok(container_name) => {
            let baseline = baseline_site_limit(candidate);
            let policy = CpuScalingPolicy::from_candidate(candidate);
            let event_type = match (policy, target) {
                (CpuScalingPolicy::BaselineBurst, CpuLimitTarget::Limited(value))
                    if value <= baseline + CPU_EPSILON =>
                {
                    "cpu_burst_restored"
                }
                (CpuScalingPolicy::BaselineBurst, _) => "cpu_burst_applied",
                (CpuScalingPolicy::ContentionThrottle, CpuLimitTarget::Unlimited) => {
                    "cpu_contention_throttle_released"
                }
                (CpuScalingPolicy::ContentionThrottle, _) => "cpu_contention_throttle_applied",
            };
            let target_cpu_cores = target.as_limit_cores();
            let details = serde_json::json!({
                "policy": policy.as_str(),
                "target_state": target.as_state_label(),
                "target_cpu_cores": target_cpu_cores,
                "baseline_cpu_cores": baseline,
                "observed_site_cpu_limit_cores": candidate.current_site_cpu_limit_cores,
                "deployment_cpu_percent": candidate.deployment_cpu_percent,
                "server_cpu_percent": candidate.server_cpu_percent,
                "container_name": container_name,
                "site_name": candidate.coolify_site_name,
            });
            let _ = HostingRepository::add_event(
                pool,
                candidate.subscription_id,
                event_type,
                Some(details),
            )
            .await;
            let log_target = match target {
                CpuLimitTarget::Limited(value) => format!("{value:.2} cores"),
                CpuLimitTarget::Unlimited => "unlimited".to_string(),
            };
            tracing::info!(
                "[cpu-burst] {} -> {} en {} ({})",
                candidate.subscription_id,
                log_target,
                container_name,
                policy.as_str()
            );
            true
        }
        Err(error) => {
            tracing::warn!(
                "[cpu-burst] {} {}: {error}",
                candidate.subscription_id,
                candidate.coolify_site_name
            );
            false
        }
    }
}

async fn evaluate_cpu_burst(
    pool: &PgPool,
    vps1_config: Option<&CoolifyConfig>,
    default_config: Option<&CoolifyConfig>,
) -> Result<(), AppError> {
    let candidates = InfrastructureRepository::cpu_burst_candidates(pool).await?;
    if candidates.is_empty() {
        return Ok(());
    }

    let demanders = super::policy::demanders_by_server(&candidates);
    let now = Instant::now();
    let mut state_map = state_map().write().await;
    let mut ssh_cache: HashMap<String, String> = HashMap::new();

    for candidate in &candidates {
        let current_limit = observed_limit_target(candidate);

        let state = state_map.entry(candidate.subscription_id).or_default();
        if let Some(last_requested) = state.last_requested_target {
            if last_requested.approx_eq(current_limit) {
                state.last_requested_target = None;
            }
        }

        let Some(desired_limit) = desired_cpu_limit(candidate, &demanders) else {
            state.raise_since = None;
            state.lower_since = None;
            continue;
        };

        if state
            .last_requested_target
            .is_some_and(|value| value.approx_eq(desired_limit))
        {
            state.raise_since = None;
            state.lower_since = None;
            continue;
        }

        let is_raise = desired_limit.is_more_permissive_than(current_limit);
        let timer = if is_raise {
            state.lower_since = None;
            &mut state.raise_since
        } else {
            state.raise_since = None;
            &mut state.lower_since
        };

        let start = timer.get_or_insert(now);
        let elapsed = now.duration_since(*start);
        let required = if is_raise {
            CPU_BURST_RAISE_WINDOW
        } else {
            CPU_BURST_LOWER_WINDOW
        };

        if elapsed < required {
            continue;
        }

        let ssh_key = if let Some(existing) = ssh_cache.get(&candidate.server_ip) {
            existing.clone()
        } else {
            let Some(ssh_key) = ssh_key_for(&candidate.server_ip, vps1_config, default_config)
            else {
                tracing::warn!("[cpu-burst] Sin SSH key para {}", candidate.server_ip);
                continue;
            };
            ssh_cache.insert(candidate.server_ip.clone(), ssh_key.clone());
            ssh_key
        };

        if apply_target(pool, candidate, &ssh_key, desired_limit).await {
            state.last_requested_target = Some(desired_limit);
            state.raise_since = None;
            state.lower_since = None;
        }
    }

    Ok(())
}

pub async fn cpu_burst_loop(
    pool: PgPool,
    vps1_config: Option<CoolifyConfig>,
    default_config: Option<CoolifyConfig>,
) {
    loop {
        if let Err(error) =
            evaluate_cpu_burst(&pool, vps1_config.as_ref(), default_config.as_ref()).await
        {
            tracing::warn!("[cpu-burst] ciclo: {error}");
        }

        tokio::time::sleep(CPU_BURST_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_update_script_uses_negative_quota_for_unlimited_targets() {
        let script = cpu_update_script("dep", "hosting-test", CpuLimitTarget::Unlimited);
        assert!(script.contains("docker update --cpu-quota -1 \"$CID\" >/dev/null"));
    }

    #[test]
    fn compose_project_candidates_prefers_deployment_uuid() {
        let candidates = compose_project_candidates("v77j8dfkb8rat8mlhzoid2eh", "hosting-0fa1d5da");
        assert_eq!(
            candidates,
            vec!["v77j8dfkb8rat8mlhzoid2eh", "hosting-0fa1d5da"]
        );
    }

    #[test]
    fn compose_project_candidates_deduplicates_identifiers() {
        let candidates = compose_project_candidates("hosting-0fa1d5da", "hosting-0fa1d5da");
        assert_eq!(candidates, vec!["hosting-0fa1d5da"]);
    }
}
