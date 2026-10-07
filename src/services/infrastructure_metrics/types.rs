/* [01AA-4-F3] Tipos del sampler de infraestructura (era parte de
 * services/infrastructure_metrics.rs, god-object 792). */

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::RwLock;

pub(crate) const SAMPLER_INTERVAL: Duration = Duration::from_mins(10);
pub(crate) const SAMPLER_STARTUP_RETRY_INTERVAL: Duration = Duration::from_secs(45);
/* [235A-4] En VPS1 el snapshot base (docker stats + du de volúmenes) roza los 20s
 * cuando el host está cargado. Con 20s exactos el sampler cae en timeout y deja
 * `disk_used_mb = NULL` aunque la lógica de storage sea correcta. */
pub(crate) const SSH_TIMEOUT: Duration = Duration::from_secs(40);
pub(crate) const STORAGE_PROBE_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Copy)]
pub(crate) struct CpuCounters {
    pub(crate) total: f64,
    pub(crate) idle: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ContainerRuntimeLimits {
    pub(crate) cpu_limit_cores: Option<f64>,
    pub(crate) mem_limit_mb: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DeploymentRuntimeLimits {
    pub(crate) site_cpu_limit_cores: Option<f64>,
    pub(crate) site_ram_limit_mb: Option<f64>,
    pub(crate) db_cpu_limit_cores: Option<f64>,
    pub(crate) db_ram_limit_mb: Option<f64>,
    pub(crate) ssh_cpu_limit_cores: Option<f64>,
    pub(crate) ssh_ram_limit_mb: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DeploymentContainerRole {
    Site,
    Db,
    Ssh,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ServerSshSnapshot {
    pub(crate) cpu_counters: Option<CpuCounters>,
    pub(crate) cpu_cores: Option<f64>,
    pub(crate) ram_used_mb: Option<f64>,
    pub(crate) ram_limit_mb: Option<f64>,
    pub(crate) disk_used_mb: Option<f64>,
    pub(crate) disk_limit_mb: Option<f64>,
    pub(crate) containers: Vec<crate::services::docker_stats::ContainerStats>,
    pub(crate) container_runtime_limits: HashMap<String, ContainerRuntimeLimits>,
    pub(crate) storage_by_uuid: HashMap<String, i64>,
}

#[derive(Debug, Clone)]
pub(crate) struct ServiceStorageProbe {
    pub(crate) deployment_uuid: String,
    pub(crate) container_name: String,
    pub(crate) path: &'static str,
}

pub(crate) static CPU_HISTORY: OnceLock<RwLock<HashMap<String, CpuCounters>>> = OnceLock::new();
pub(crate) static STORAGE_HISTORY: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

pub(crate) fn f64_to_i64_rounded(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    format!("{value:.0}").parse::<i64>().unwrap_or(i64::MAX)
}

pub(crate) fn f64_to_i32_rounded(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    format!("{value:.0}").parse::<i32>().unwrap_or(i32::MAX)
}

pub(crate) fn i64_to_f64(value: i64) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0)
}
