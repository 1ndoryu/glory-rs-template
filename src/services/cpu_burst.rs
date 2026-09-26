/* [265A-1][265A-5] Politica dinamica de CPU para hostings Coolify.
 * `baseline_burst` mantiene el baseline comercial siempre y habilita burst con
 * holgura sostenida. `contention_throttle` deja el sitio sin cap mientras el
 * host esta sano y solo aplica un limite compartido cuando la VPS entra en
 * contencion real. */

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    normalize_cpu_scaling_policy, CPU_SCALING_POLICY_BASELINE_BURST,
    CPU_SCALING_POLICY_CONTENTION_THROTTLE,
};
use crate::repositories::{CpuBurstCandidate, HostingRepository, InfrastructureRepository};
use crate::services::coolify::CoolifyConfig;
use crate::services::infrastructure::coolify_server_targets;

const CPU_BURST_INTERVAL: Duration = Duration::from_mins(5);
const CPU_BURST_RAISE_WINDOW: Duration = Duration::from_mins(10);
const CPU_BURST_LOWER_WINDOW: Duration = Duration::from_mins(10);
const CPU_BURST_MAX_SAMPLE_AGE_MINUTES: i64 = 20;
const CPU_ACTIVATION_UTILIZATION: f64 = 0.85;
const CPU_DEACTIVATION_UTILIZATION: f64 = 0.35;
const SERVER_LOW_PRESSURE_PCT: f64 = 45.0;
const SERVER_HIGH_PRESSURE_PCT: f64 = 70.0;
const SERVER_RESERVE_RATIO: f64 = 0.25;
const SERVER_RESERVE_MIN_CORES: f64 = 1.0;
const CPU_STEP_CORES: f64 = 0.25;
const CPU_EPSILON: f64 = 0.01;
/* [265A-4] En VPS cargados, `docker compose ps` + `docker update` puede tardar
 * mas de 15s. Si el future de `output()` vence sin matar el proceso, el SSH
 * sigue vivo y aplica el cambio fuera del control del loop, dejando un falso
 * negativo en logs/eventos. */
const CPU_SSH_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Default)]
struct CpuBurstState {
    raise_since: Option<Instant>,
    lower_since: Option<Instant>,
    last_requested_target: Option<CpuLimitTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CpuScalingPolicy {
    BaselineBurst,
    ContentionThrottle,
}

impl CpuScalingPolicy {
    fn from_candidate(candidate: &CpuBurstCandidate) -> Self {
        match normalize_cpu_scaling_policy(&candidate.cpu_scaling_policy) {
            Some(CPU_SCALING_POLICY_BASELINE_BURST) => Self::BaselineBurst,
            _ => Self::ContentionThrottle,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::BaselineBurst => CPU_SCALING_POLICY_BASELINE_BURST,
            Self::ContentionThrottle => CPU_SCALING_POLICY_CONTENTION_THROTTLE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CpuLimitTarget {
    Limited(f64),
    Unlimited,
}

impl CpuLimitTarget {
    fn limited(value: f64) -> Self {
        Self::Limited(round_cpu_target(value.max(CPU_STEP_CORES)))
    }

    fn approx_eq(self, other: Self) -> bool {
        match (self, other) {
            (Self::Unlimited, Self::Unlimited) => true,
            (Self::Limited(left), Self::Limited(right)) => (left - right).abs() <= CPU_EPSILON,
            _ => false,
        }
    }

    fn is_more_permissive_than(self, current: Self) -> bool {
        target_rank(self) > target_rank(current) + CPU_EPSILON
    }

    fn as_limit_cores(self) -> Option<f64> {
        match self {
            Self::Limited(value) => Some(value),
            Self::Unlimited => None,
        }
    }

    fn as_state_label(self) -> &'static str {
        match self {
            Self::Limited(_) => "limited",
            Self::Unlimited => "unlimited",
        }
    }
}

static CPU_STATE_MAP: OnceLock<RwLock<HashMap<Uuid, CpuBurstState>>> = OnceLock::new();

fn state_map() -> &'static RwLock<HashMap<Uuid, CpuBurstState>> {
    CPU_STATE_MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn target_rank(target: CpuLimitTarget) -> f64 {
    match target {
        CpuLimitTarget::Limited(value) => value,
        CpuLimitTarget::Unlimited => f64::INFINITY,
    }
}

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

fn usize_to_f64(value: usize) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(1.0)
}

fn round_cpu_target(value: f64) -> f64 {
    ((value / CPU_STEP_CORES).floor() * CPU_STEP_CORES).max(CPU_STEP_CORES)
}

fn reserve_cores(total_cores: Option<f64>) -> f64 {
    let Some(total_cores) = total_cores.filter(|value| *value > 0.0) else {
        return 0.0;
    };

    (total_cores * SERVER_RESERVE_RATIO).max(SERVER_RESERVE_MIN_CORES)
}

fn available_cores(total_cores: Option<f64>) -> Option<f64> {
    total_cores
        .filter(|value| *value > 0.0)
        .map(|value| (value - reserve_cores(Some(value))).max(0.0))
}

fn normalized_cpu_utilization(cpu_percent: Option<f64>, limit_cores: f64) -> Option<f64> {
    if limit_cores <= 0.0 {
        return None;
    }

    cpu_percent
        .filter(|value| *value >= 0.0)
        .map(|value| value / (limit_cores * 100.0))
}

fn sample_is_fresh(sampled_at: Option<DateTime<Utc>>) -> bool {
    sampled_at.is_some_and(|value| {
        value >= Utc::now() - chrono::Duration::minutes(CPU_BURST_MAX_SAMPLE_AGE_MINUTES)
    })
}

fn server_low_pressure(candidate: &CpuBurstCandidate) -> bool {
    sample_is_fresh(candidate.server_sampled_at)
        && candidate
            .server_cpu_percent
            .is_some_and(|value| value < SERVER_LOW_PRESSURE_PCT)
}

fn server_high_pressure(candidate: &CpuBurstCandidate) -> bool {
    !sample_is_fresh(candidate.server_sampled_at)
        || candidate
            .server_cpu_percent
            .is_some_and(|value| value >= SERVER_HIGH_PRESSURE_PCT)
}

fn current_site_limit(candidate: &CpuBurstCandidate) -> Option<f64> {
    candidate
        .current_site_cpu_limit_cores
        .filter(|value| *value > 0.0)
        .map(round_cpu_target)
}

fn baseline_site_limit(candidate: &CpuBurstCandidate) -> f64 {
    round_cpu_target(candidate.baseline_site_cpu_cores.max(CPU_STEP_CORES))
}

fn observed_limit_target(candidate: &CpuBurstCandidate) -> CpuLimitTarget {
    current_site_limit(candidate).map_or(CpuLimitTarget::Unlimited, CpuLimitTarget::limited)
}

fn utilization_reference_limit(observed: CpuLimitTarget, baseline: f64) -> f64 {
    observed.as_limit_cores().unwrap_or(baseline)
}

fn is_cpu_demander(candidate: &CpuBurstCandidate) -> bool {
    if !sample_is_fresh(candidate.deployment_sampled_at) {
        return false;
    }

    let policy = CpuScalingPolicy::from_candidate(candidate);
    if matches!(policy, CpuScalingPolicy::BaselineBurst) && !server_low_pressure(candidate) {
        return false;
    }

    let baseline = baseline_site_limit(candidate);
    let observed = observed_limit_target(candidate);
    let reference_limit = utilization_reference_limit(observed, baseline);

    normalized_cpu_utilization(candidate.deployment_cpu_percent, reference_limit)
        .is_some_and(|value| value >= CPU_ACTIVATION_UTILIZATION)
}

fn demanders_by_server(candidates: &[CpuBurstCandidate]) -> HashMap<Uuid, usize> {
    let mut counts = HashMap::new();
    for candidate in candidates {
        if is_cpu_demander(candidate) {
            *counts.entry(candidate.server_id).or_insert(0) += 1;
        }
    }
    counts
}

fn compute_burst_target(
    baseline_cores: f64,
    server_cpu_cores: Option<f64>,
    demander_count: usize,
) -> f64 {
    let demander_count = demander_count.max(1);

    if let Some(available) = available_cores(server_cpu_cores) {
        let fair_share = (available / usize_to_f64(demander_count)).max(baseline_cores);
        return round_cpu_target(fair_share.min(available.max(baseline_cores)));
    }

    round_cpu_target((baseline_cores * 2.0).max(baseline_cores))
}

fn desired_cpu_limit(
    candidate: &CpuBurstCandidate,
    demanders: &HashMap<Uuid, usize>,
) -> Option<CpuLimitTarget> {
    match CpuScalingPolicy::from_candidate(candidate) {
        CpuScalingPolicy::BaselineBurst => desired_baseline_burst_limit(candidate, demanders),
        CpuScalingPolicy::ContentionThrottle => {
            desired_contention_throttle_limit(candidate, demanders)
        }
    }
}

fn desired_baseline_burst_limit(
    candidate: &CpuBurstCandidate,
    demanders: &HashMap<Uuid, usize>,
) -> Option<CpuLimitTarget> {
    let baseline = baseline_site_limit(candidate);
    let current_limit = observed_limit_target(candidate);
    let baseline_target = CpuLimitTarget::limited(baseline);
    let boosted = !current_limit.approx_eq(baseline_target);

    if !sample_is_fresh(candidate.deployment_sampled_at) {
        return None;
    }

    let utilization = normalized_cpu_utilization(
        candidate.deployment_cpu_percent,
        utilization_reference_limit(current_limit, baseline),
    )?;

    if server_high_pressure(candidate) || utilization <= CPU_DEACTIVATION_UTILIZATION {
        return boosted.then_some(baseline_target);
    }

    if !server_low_pressure(candidate) {
        return None;
    }

    if utilization < CPU_ACTIVATION_UTILIZATION {
        return boosted.then_some(baseline_target);
    }

    let demander_count = demanders
        .get(&candidate.server_id)
        .copied()
        .unwrap_or(1)
        .max(1);
    let target = CpuLimitTarget::limited(compute_burst_target(
        baseline,
        candidate.server_cpu_cores,
        demander_count,
    ));
    (!target.approx_eq(current_limit)).then_some(target)
}

fn desired_contention_throttle_limit(
    candidate: &CpuBurstCandidate,
    demanders: &HashMap<Uuid, usize>,
) -> Option<CpuLimitTarget> {
    let baseline = baseline_site_limit(candidate);
    let current_limit = observed_limit_target(candidate);

    if !sample_is_fresh(candidate.deployment_sampled_at) {
        return None;
    }

    if server_low_pressure(candidate) {
        return matches!(current_limit, CpuLimitTarget::Limited(_))
            .then_some(CpuLimitTarget::Unlimited);
    }

    if !server_high_pressure(candidate) {
        return None;
    }

    let utilization = normalized_cpu_utilization(
        candidate.deployment_cpu_percent,
        utilization_reference_limit(current_limit, baseline),
    )?;

    if matches!(current_limit, CpuLimitTarget::Unlimited)
        && utilization < CPU_ACTIVATION_UTILIZATION
    {
        return None;
    }

    if matches!(current_limit, CpuLimitTarget::Limited(_))
        && utilization <= CPU_DEACTIVATION_UTILIZATION
    {
        return Some(CpuLimitTarget::Unlimited);
    }

    let demander_count = demanders
        .get(&candidate.server_id)
        .copied()
        .unwrap_or(1)
        .max(1);
    let target = CpuLimitTarget::limited(compute_burst_target(
        baseline,
        candidate.server_cpu_cores,
        demander_count,
    ));
    (!target.approx_eq(current_limit)).then_some(target)
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

    let demanders = demanders_by_server(&candidates);
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

    fn candidate() -> CpuBurstCandidate {
        CpuBurstCandidate {
            subscription_id: Uuid::nil(),
            deployment_uuid: "dep".into(),
            coolify_site_name: "hosting-test".into(),
            server_ip: "127.0.0.1".into(),
            server_id: Uuid::nil(),
            server_cpu_cores: Some(8.0),
            server_cpu_percent: Some(20.0),
            server_sampled_at: Some(Utc::now()),
            baseline_site_cpu_cores: 0.5,
            cpu_scaling_policy: CPU_SCALING_POLICY_BASELINE_BURST.to_string(),
            current_site_cpu_limit_cores: Some(0.5),
            deployment_cpu_percent: Some(50.0),
            deployment_sampled_at: Some(Utc::now()),
        }
    }

    #[test]
    fn normalized_utilization_respects_cpu_limit() {
        let utilization = normalized_cpu_utilization(Some(50.0), 0.5).unwrap_or_default();
        assert!((utilization - 1.0).abs() < 0.001);
    }

    #[test]
    fn compute_burst_target_keeps_host_reserve() {
        let target = compute_burst_target(0.5, Some(8.0), 1);
        /* [259A-1] comparacion con epsilon en vez de assert_eq float (clippy float_cmp). */
        assert!((target - 6.0).abs() < 0.001);
    }

    #[test]
    fn desired_cpu_limit_restores_to_baseline_when_pressure_rises() {
        let mut row = candidate();
        row.current_site_cpu_limit_cores = Some(2.0);
        row.server_cpu_percent = Some(85.0);
        row.deployment_cpu_percent = Some(30.0);
        let desired = desired_cpu_limit(&row, &HashMap::new());
        assert_eq!(desired, Some(CpuLimitTarget::Limited(0.5)));
    }

    #[test]
    fn desired_cpu_limit_requests_burst_when_site_hits_cap() {
        let row = candidate();
        let demanders = HashMap::from([(row.server_id, 1_usize)]);
        let desired = desired_cpu_limit(&row, &demanders);
        assert_eq!(desired, Some(CpuLimitTarget::Limited(6.0)));
    }

    #[test]
    fn baseline_policy_restores_baseline_from_unlimited_runtime() {
        let mut row = candidate();
        row.current_site_cpu_limit_cores = None;
        row.deployment_cpu_percent = Some(10.0);
        let desired = desired_cpu_limit(&row, &HashMap::new());
        assert_eq!(desired, Some(CpuLimitTarget::Limited(0.5)));
    }

    #[test]
    fn contention_policy_throttles_only_under_high_pressure() {
        let mut row = candidate();
        row.cpu_scaling_policy = CPU_SCALING_POLICY_CONTENTION_THROTTLE.to_string();
        row.current_site_cpu_limit_cores = None;
        row.server_cpu_percent = Some(85.0);
        row.deployment_cpu_percent = Some(75.0);
        let demanders = HashMap::from([(row.server_id, 1_usize)]);
        let desired = desired_cpu_limit(&row, &demanders);
        assert_eq!(desired, Some(CpuLimitTarget::Limited(6.0)));
    }

    #[test]
    fn contention_policy_releases_limit_when_host_recovers() {
        let mut row = candidate();
        row.cpu_scaling_policy = CPU_SCALING_POLICY_CONTENTION_THROTTLE.to_string();
        row.current_site_cpu_limit_cores = Some(2.0);
        row.server_cpu_percent = Some(20.0);
        row.deployment_cpu_percent = Some(40.0);
        let desired = desired_cpu_limit(&row, &HashMap::new());
        assert_eq!(desired, Some(CpuLimitTarget::Unlimited));
    }

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
