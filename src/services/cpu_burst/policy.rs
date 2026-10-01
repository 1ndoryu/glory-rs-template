use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::{
    CPU_SCALING_POLICY_BASELINE_BURST, CPU_SCALING_POLICY_CONTENTION_THROTTLE,
};
use crate::repositories::CpuBurstCandidate;

use super::state::{
    CpuLimitTarget, CpuScalingPolicy, CPU_ACTIVATION_UTILIZATION,
    CPU_BURST_MAX_SAMPLE_AGE_MINUTES, CPU_DEACTIVATION_UTILIZATION,
    SERVER_HIGH_PRESSURE_PCT, SERVER_LOW_PRESSURE_PCT, SERVER_RESERVE_MIN_CORES,
    SERVER_RESERVE_RATIO,
};

pub(crate) fn usize_to_f64(value: usize) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(1.0)
}

pub(crate) fn reserve_cores(total_cores: Option<f64>) -> f64 {
    let Some(total_cores) = total_cores.filter(|value| *value > 0.0) else {
        return 0.0;
    };

    (total_cores * SERVER_RESERVE_RATIO).max(SERVER_RESERVE_MIN_CORES)
}

pub(crate) fn available_cores(total_cores: Option<f64>) -> Option<f64> {
    total_cores
        .filter(|value| *value > 0.0)
        .map(|value| (value - reserve_cores(Some(value))).max(0.0))
}

pub(crate) fn normalized_cpu_utilization(
    cpu_percent: Option<f64>,
    limit_cores: f64,
) -> Option<f64> {
    if limit_cores <= 0.0 {
        return None;
    }

    cpu_percent
        .filter(|value| *value >= 0.0)
        .map(|value| value / (limit_cores * 100.0))
}

pub(crate) fn sample_is_fresh(sampled_at: Option<DateTime<Utc>>) -> bool {
    sampled_at.is_some_and(|value| {
        value >= Utc::now() - chrono::Duration::minutes(CPU_BURST_MAX_SAMPLE_AGE_MINUTES)
    })
}

pub(crate) fn server_low_pressure(candidate: &CpuBurstCandidate) -> bool {
    sample_is_fresh(candidate.server_sampled_at)
        && candidate
            .server_cpu_percent
            .is_some_and(|value| value < SERVER_LOW_PRESSURE_PCT)
}

pub(crate) fn server_high_pressure(candidate: &CpuBurstCandidate) -> bool {
    !sample_is_fresh(candidate.server_sampled_at)
        || candidate
            .server_cpu_percent
            .is_some_and(|value| value >= SERVER_HIGH_PRESSURE_PCT)
}

pub(crate) fn current_site_limit(candidate: &CpuBurstCandidate) -> Option<f64> {
    use super::state::round_cpu_target;
    candidate
        .current_site_cpu_limit_cores
        .filter(|value| *value > 0.0)
        .map(round_cpu_target)
}

pub(crate) fn baseline_site_limit(candidate: &CpuBurstCandidate) -> f64 {
    use super::state::{CPU_STEP_CORES, round_cpu_target};
    round_cpu_target(candidate.baseline_site_cpu_cores.max(CPU_STEP_CORES))
}

pub(crate) fn observed_limit_target(candidate: &CpuBurstCandidate) -> CpuLimitTarget {
    current_site_limit(candidate).map_or(CpuLimitTarget::Unlimited, CpuLimitTarget::limited)
}

pub(crate) fn utilization_reference_limit(observed: CpuLimitTarget, baseline: f64) -> f64 {
    observed.as_limit_cores().unwrap_or(baseline)
}

pub(crate) fn is_cpu_demander(candidate: &CpuBurstCandidate) -> bool {
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

pub(crate) fn demanders_by_server(candidates: &[CpuBurstCandidate]) -> HashMap<Uuid, usize> {
    let mut counts = HashMap::new();
    for candidate in candidates {
        if is_cpu_demander(candidate) {
            *counts.entry(candidate.server_id).or_insert(0) += 1;
        }
    }
    counts
}

pub(crate) fn compute_burst_target(
    baseline_cores: f64,
    server_cpu_cores: Option<f64>,
    demander_count: usize,
) -> f64 {
    use super::state::round_cpu_target;
    let demander_count = demander_count.max(1);

    if let Some(available) = available_cores(server_cpu_cores) {
        let fair_share = (available / usize_to_f64(demander_count)).max(baseline_cores);
        return round_cpu_target(fair_share.min(available.max(baseline_cores)));
    }

    round_cpu_target((baseline_cores * 2.0).max(baseline_cores))
}

pub(crate) fn desired_cpu_limit(
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
}
