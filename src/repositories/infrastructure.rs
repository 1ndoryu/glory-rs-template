/* [259A-4] Hub delgado: solo el tipo `InfrastructureRepository`, helpers
 * numéricos compartidos y re-exports. Implementación movida a módulos
 * hermanos (`infrastructure_servers`, `infrastructure_bandwidth`) como impl
 * inherente cross-módulo (misma crate). Rutas externas sin cambios:
 * `crate::repositories::{InfrastructureRepository, ...}` sigue resolviendo.
 * Gotcha: `i64_to_f64`/`millicores_to_cores` son `pub(crate)` porque los
 * módulos hermanos los usan vía `super::infrastructure::`.
 * Pendiente 259A-4b/c/d/e: hosting_runtime, email, chat_timing, ai_tools. */

pub use super::infrastructure_bandwidth::{
    BandwidthEnforcementCandidate, BandwidthSnapshotInput, BandwidthThrottleCandidate,
    CpuBurstCandidate,
};
pub use super::infrastructure_servers::{
    ConfiguredServerInput, HostingResourceAllocation, InfrastructureServerRecord,
    ResourceSampleInput,
};

pub(crate) fn i64_to_f64(value: i64) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0)
}

pub(crate) fn millicores_to_cores(value: i64) -> f64 {
    i64_to_f64(value) / 1000.0
}

pub struct InfrastructureRepository;
