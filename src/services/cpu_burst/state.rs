use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::{
    normalize_cpu_scaling_policy, CPU_SCALING_POLICY_BASELINE_BURST,
    CPU_SCALING_POLICY_CONTENTION_THROTTLE,
};
use crate::repositories::CpuBurstCandidate;

pub(crate) const CPU_BURST_INTERVAL: Duration = Duration::from_mins(5);
pub(crate) const CPU_BURST_RAISE_WINDOW: Duration = Duration::from_mins(10);
pub(crate) const CPU_BURST_LOWER_WINDOW: Duration = Duration::from_mins(10);
pub(crate) const CPU_BURST_MAX_SAMPLE_AGE_MINUTES: i64 = 20;
pub(crate) const CPU_ACTIVATION_UTILIZATION: f64 = 0.85;
pub(crate) const CPU_DEACTIVATION_UTILIZATION: f64 = 0.35;
pub(crate) const SERVER_LOW_PRESSURE_PCT: f64 = 45.0;
pub(crate) const SERVER_HIGH_PRESSURE_PCT: f64 = 70.0;
pub(crate) const SERVER_RESERVE_RATIO: f64 = 0.25;
pub(crate) const SERVER_RESERVE_MIN_CORES: f64 = 1.0;
pub(crate) const CPU_STEP_CORES: f64 = 0.25;
pub(crate) const CPU_EPSILON: f64 = 0.01;
/* [265A-4] En VPS cargados, `docker compose ps` + `docker update` puede tardar
 * mas de 15s. Si el future de `output()` vence sin matar el proceso, el SSH
 * sigue vivo y aplica el cambio fuera del control del loop, dejando un falso
 * negativo en logs/eventos. */
pub(crate) const CPU_SSH_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Default)]
pub(crate) struct CpuBurstState {
    pub(crate) raise_since: Option<Instant>,
    pub(crate) lower_since: Option<Instant>,
    pub(crate) last_requested_target: Option<CpuLimitTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CpuScalingPolicy {
    BaselineBurst,
    ContentionThrottle,
}

impl CpuScalingPolicy {
    pub(crate) fn from_candidate(candidate: &CpuBurstCandidate) -> Self {
        match normalize_cpu_scaling_policy(&candidate.cpu_scaling_policy) {
            Some(CPU_SCALING_POLICY_BASELINE_BURST) => Self::BaselineBurst,
            _ => Self::ContentionThrottle,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::BaselineBurst => CPU_SCALING_POLICY_BASELINE_BURST,
            Self::ContentionThrottle => CPU_SCALING_POLICY_CONTENTION_THROTTLE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CpuLimitTarget {
    Limited(f64),
    Unlimited,
}

impl CpuLimitTarget {
    pub(crate) fn limited(value: f64) -> Self {
        Self::Limited(round_cpu_target(value.max(CPU_STEP_CORES)))
    }

    pub(crate) fn approx_eq(self, other: Self) -> bool {
        match (self, other) {
            (Self::Unlimited, Self::Unlimited) => true,
            (Self::Limited(left), Self::Limited(right)) => (left - right).abs() <= CPU_EPSILON,
            _ => false,
        }
    }

    pub(crate) fn is_more_permissive_than(self, current: Self) -> bool {
        target_rank(self) > target_rank(current) + CPU_EPSILON
    }

    pub(crate) fn as_limit_cores(self) -> Option<f64> {
        match self {
            Self::Limited(value) => Some(value),
            Self::Unlimited => None,
        }
    }

    pub(crate) fn as_state_label(self) -> &'static str {
        match self {
            Self::Limited(_) => "limited",
            Self::Unlimited => "unlimited",
        }
    }
}

static CPU_STATE_MAP: OnceLock<RwLock<HashMap<Uuid, CpuBurstState>>> = OnceLock::new();

pub(crate) fn state_map() -> &'static RwLock<HashMap<Uuid, CpuBurstState>> {
    CPU_STATE_MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(crate) fn target_rank(target: CpuLimitTarget) -> f64 {
    match target {
        CpuLimitTarget::Limited(value) => value,
        CpuLimitTarget::Unlimited => f64::INFINITY,
    }
}

pub(crate) fn round_cpu_target(value: f64) -> f64 {
    ((value / CPU_STEP_CORES).floor() * CPU_STEP_CORES).max(CPU_STEP_CORES)
}
