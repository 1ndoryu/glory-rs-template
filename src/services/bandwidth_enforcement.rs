use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::errors::AppError;
use crate::repositories::{HostingRepository, InfrastructureRepository};
use crate::services::coolify::CoolifyConfig;
use crate::services::infrastructure::coolify_server_targets;
use crate::services::tc_throttle;

const THROTTLE_INTERVAL: Duration = Duration::from_mins(5);
const SUSTAINED_WINDOW: Duration = Duration::from_mins(10);
const RESTORE_WINDOW: Duration = Duration::from_mins(5);
const ACTIVATION_RATIO: f64 = 0.50;
const DEACTIVATION_RATIO: f64 = 0.30;
const THROTTLE_RATE_MODERATE: u32 = 10;
const THROTTLE_RATE_AGGRESSIVE: u32 = 5;
#[allow(dead_code)]
/// 20 Mbps preventivo cuando proyeccion mensual excede 25 TB
const THROTTLE_RATE_PREVENTIVE: u32 = 20;

struct SnapshotState {
    input_mb: f64,
    output_mb: f64,
    sampled_at: Instant,
}

struct PerSubState {
    snap: Option<SnapshotState>,
    is_throttled: bool,
    current_rate: u32,
    exceeding_since: Option<Instant>,
    restored_since: Option<Instant>,
}

static STATE_MAP: OnceLock<RwLock<HashMap<Uuid, PerSubState>>> = OnceLock::new();

fn state_map() -> &'static RwLock<HashMap<Uuid, PerSubState>> {
    STATE_MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn ssh_key_for(
    server_ip: &str,
    vps1: Option<&CoolifyConfig>,
    default: Option<&CoolifyConfig>,
) -> Option<String> {
    coolify_server_targets(vps1, default)
        .into_iter()
        .find(|t| server_ip == t.config.server_ip)
        .and_then(|t| t.config.ssh_key_path.clone())
}

async fn ensure_throttle(
    pool: &PgPool,
    server_ip: &str,
    ssh_key: &str,
    sub_id: Uuid,
    site_name: Option<&str>,
    rate_mbps: u32,
) {
    let Some(site) = site_name else { return };
    let veth = match tc_throttle::find_veth(server_ip, ssh_key, site).await {
        Ok(v) if !v.is_empty() => v,
        Ok(_) => {
            tracing::warn!("[throttle] Sin veth para {site}");
            return;
        }
        Err(e) => {
            tracing::warn!("[throttle] find_veth {site}: {e}");
            return;
        }
    };
    if let Err(e) = tc_throttle::set_rate_limit(server_ip, ssh_key, &veth, rate_mbps).await {
        tracing::warn!("[throttle] set_rate_limit {site}: {e}");
        return;
    }
    let _ = HostingRepository::add_event(
        pool,
        sub_id,
        "bandwidth_throttled",
        Some(serde_json::json!({
            "rate_mbps": rate_mbps,
            "action": "tc_tbf"
        })),
    )
    .await;
    tracing::info!("[throttle] {sub_id} -> {rate_mbps} Mbps");
}

async fn ensure_unthrottled(
    pool: &PgPool,
    server_ip: &str,
    ssh_key: &str,
    sub_id: Uuid,
    site_name: Option<&str>,
) {
    let Some(site) = site_name else { return };
    let veth = match tc_throttle::find_veth(server_ip, ssh_key, site).await {
        Ok(v) if !v.is_empty() => v,
        Ok(_) => {
            tracing::warn!("[throttle] Sin veth para {site} al restaurar");
            return;
        }
        Err(e) => {
            tracing::warn!("[throttle] find_veth {site}: {e}");
            return;
        }
    };
    if let Err(e) = tc_throttle::remove_rate_limit(server_ip, ssh_key, &veth).await {
        tracing::warn!("[throttle] remove_rate_limit {site}: {e}");
        return;
    }
    let _ = HostingRepository::add_event(
        pool,
        sub_id,
        "bandwidth_restored",
        Some(serde_json::json!({ "action": "tc_tbf_removed" })),
    )
    .await;
    tracing::info!("[throttle] {sub_id} restaurado");
}

fn calc_mbps(
    prev_input_mb: f64,
    prev_output_mb: f64,
    elapsed_secs: f64,
    curr_input_mb: f64,
    curr_output_mb: f64,
) -> Option<f64> {
    if elapsed_secs <= 0.0 {
        return None;
    }
    let drx = curr_input_mb - prev_input_mb;
    let dtx = curr_output_mb - prev_output_mb;
    if drx < 0.0 && dtx < 0.0 {
        return None;
    }
    let total = drx.max(0.0) + dtx.max(0.0);
    if total <= 0.0 {
        None
    } else {
        Some(total * 8.0 / elapsed_secs)
    }
}

fn select_rate(mbps: f64, port_speed: i32, fair_share: f64) -> u32 {
    if mbps > f64::from(port_speed) * 0.70 {
        THROTTLE_RATE_MODERATE
    } else if mbps > fair_share {
        THROTTLE_RATE_AGGRESSIVE
    } else {
        THROTTLE_RATE_MODERATE
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
async fn evaluate_and_throttle(
    pool: &PgPool,
    vps1_config: Option<&CoolifyConfig>,
    default_config: Option<&CoolifyConfig>,
) -> Result<(), AppError> {
    let candidates = InfrastructureRepository::bandwidth_throttle_candidates(pool).await?;
    if candidates.is_empty() {
        return Ok(());
    }

    let now = Instant::now();
    let mut map = state_map().write().await;

    let active_count: HashMap<Uuid, usize> = {
        let mut c = HashMap::new();
        for row in &candidates {
            *c.entry(row.server_id).or_insert(0) += 1;
        }
        c
    };

    let mut ssh_cache: HashMap<String, String> = HashMap::new();

    for row in &candidates {
        let sub_id = row.subscription_id;
        let Some(server_ip) = row.server_ip.as_deref() else {
            continue;
        };
        let site = row.coolify_site_name.as_deref();

        let ssh_key = if let Some(k) = ssh_cache.get(server_ip) {
            k.clone()
        } else {
            let Some(k) = ssh_key_for(server_ip, vps1_config, default_config) else {
                tracing::warn!("[throttle] Sin SSH key para {server_ip}");
                continue;
            };
            ssh_cache.insert(server_ip.to_owned(), k.clone());
            k
        };

        let fair_share = {
            let count = active_count
                .get(&row.server_id)
                .copied()
                .unwrap_or(1)
                .max(1);
            f64::from(row.port_speed_mbps) / count as f64
        };

        let state = map.entry(sub_id).or_insert(PerSubState {
            snap: None,
            is_throttled: false,
            current_rate: 0,
            exceeding_since: None,
            restored_since: None,
        });

        let mbps = state.snap.as_ref().and_then(|prev| {
            let elapsed = prev.sampled_at.elapsed().as_secs_f64();
            calc_mbps(
                prev.input_mb,
                prev.output_mb,
                elapsed,
                row.net_input_mb,
                row.net_output_mb,
            )
        });

        state.snap = Some(SnapshotState {
            input_mb: row.net_input_mb,
            output_mb: row.net_output_mb,
            sampled_at: now,
        });

        let Some(mbps) = mbps else { continue };

        let should_throttle = mbps > fair_share * ACTIVATION_RATIO;
        let should_restore = state.is_throttled && mbps < fair_share * DEACTIVATION_RATIO;

        if should_throttle {
            let start = state.exceeding_since.get_or_insert(now);
            let elapsed = now.duration_since(*start);
            if !state.is_throttled && elapsed >= SUSTAINED_WINDOW {
                let rate = select_rate(mbps, row.port_speed_mbps, fair_share);
                ensure_throttle(pool, server_ip, &ssh_key, sub_id, site, rate).await;
                state.is_throttled = true;
                state.current_rate = rate;
                state.exceeding_since = None;
                state.restored_since = None;
            }
        } else if should_restore {
            let start = state.restored_since.get_or_insert(now);
            let elapsed = now.duration_since(*start);
            if elapsed >= RESTORE_WINDOW {
                ensure_unthrottled(pool, server_ip, &ssh_key, sub_id, site).await;
                state.is_throttled = false;
                state.current_rate = 0;
                state.exceeding_since = None;
                state.restored_since = None;
            }
        } else {
            state.exceeding_since = None;
            state.restored_since = None;
        }
    }

    Ok(())
}

pub async fn bandwidth_throttle_loop(
    pool: PgPool,
    vps1_config: Option<CoolifyConfig>,
    default_config: Option<CoolifyConfig>,
) {
    loop {
        if let Err(e) =
            evaluate_and_throttle(&pool, vps1_config.as_ref(), default_config.as_ref()).await
        {
            tracing::warn!("[throttle] ciclo: {e}");
        }
        tokio::time::sleep(THROTTLE_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORT_200: i32 = 200;
    const PORT_600: i32 = 600;

    #[test]
    fn select_rate_dominates_entire_vps_uses_moderate() {
        let r = select_rate(150.0, PORT_200, 100.0);
        assert_eq!(r, THROTTLE_RATE_MODERATE);
    }

    #[test]
    fn select_rate_exceeds_fair_share_uses_aggressive() {
        let r = select_rate(80.0, PORT_200, 50.0);
        assert_eq!(r, THROTTLE_RATE_AGGRESSIVE);
    }

    #[test]
    fn select_rate_moderate_above_half_fair_share() {
        let r = select_rate(30.0, PORT_200, 50.0);
        assert_eq!(r, THROTTLE_RATE_MODERATE);
    }

    #[test]
    fn select_rate_exactly_at_port_speed_threshold_boundary() {
        let boundary = f64::from(PORT_200) * 0.70;
        let r = select_rate(boundary + 0.001, PORT_200, 50.0);
        assert_eq!(r, THROTTLE_RATE_MODERATE);
    }

    #[test]
    fn select_rate_barely_below_port_speed_threshold_uses_aggressive_if_above_fair_share() {
        let boundary = f64::from(PORT_200) * 0.70;
        let r = select_rate(boundary - 0.001, PORT_200, 10.0);
        assert_eq!(r, THROTTLE_RATE_AGGRESSIVE);
    }

    #[test]
    fn select_rate_exactly_at_fair_share_edge() {
        let r = select_rate(50.0, PORT_600, 100.0);
        assert_eq!(r, THROTTLE_RATE_MODERATE);
    }

    #[test]
    fn select_rate_handles_low_port_speed() {
        let low_port: i32 = 100;
        let r = select_rate(60.0, low_port, 30.0);
        assert_eq!(r, THROTTLE_RATE_AGGRESSIVE);
    }

    #[test]
    fn select_rate_handles_zero_mbps() {
        let r = select_rate(0.0, PORT_200, 100.0);
        assert_eq!(r, THROTTLE_RATE_MODERATE);
    }

    #[test]
    fn calc_mbps_positive_delta() {
        let result = calc_mbps(100.0, 50.0, 10.0, 110.0, 55.0);
        assert!(result.is_some());
        let mbps = result.unwrap();
        let expected = (10.0 + 5.0) * 8.0 / 10.0;
        assert!((mbps - expected).abs() < 0.001);
    }

    #[test]
    fn calc_mbps_zero_elapsed_returns_none() {
        let result = calc_mbps(100.0, 50.0, 0.0, 110.0, 55.0);
        assert!(result.is_none());
    }

    #[test]
    fn calc_mbps_negative_deltas_returns_none() {
        let result = calc_mbps(100.0, 50.0, 10.0, 90.0, 40.0);
        assert!(result.is_none());
    }

    #[test]
    fn calc_mbps_partial_negative_input_clamps_to_zero() {
        let result = calc_mbps(100.0, 50.0, 10.0, 90.0, 60.0);
        assert!(result.is_some());
        let mbps = result.unwrap();
        let expected = (0.0 + 10.0) * 8.0 / 10.0;
        assert!((mbps - expected).abs() < 0.001);
    }

    #[test]
    fn calc_mbps_high_rate() {
        let result = calc_mbps(0.0, 0.0, 60.0, 500.0, 300.0);
        assert!(result.is_some());
        let mbps = result.unwrap();
        let expected = (500.0 + 300.0) * 8.0 / 60.0;
        assert!((mbps - expected).abs() < 0.001);
    }

    #[test]
    fn activation_ratio_greater_than_deactivation() {
        /* [259A-1] assert sobre constantes en bloque const (clippy assertions_on_constants). */
        const {
            assert!(
                ACTIVATION_RATIO > DEACTIVATION_RATIO,
                "histeresis: activacion debe ser mayor que desactivacion"
            );
        }
    }

    #[test]
    fn sustained_window_longer_than_restore() {
        assert!(
            SUSTAINED_WINDOW > RESTORE_WINDOW,
            "ventana sostenida debe ser mayor que restauracion"
        );
    }

    #[test]
    fn throttle_rates_positive() {
        /* [259A-1] assert sobre constantes en bloque const (clippy assertions_on_constants). */
        const {
            assert!(THROTTLE_RATE_MODERATE > 0);
            assert!(THROTTLE_RATE_AGGRESSIVE > 0);
            assert!(THROTTLE_RATE_PREVENTIVE > 0);
        }
    }

    #[test]
    fn aggressive_rate_less_than_moderate() {
        /* [259A-1] assert sobre constantes en bloque const (clippy assertions_on_constants). */
        const {
            assert!(
                THROTTLE_RATE_AGGRESSIVE < THROTTLE_RATE_MODERATE,
                "agresivo debe ser menor que moderado"
            );
        }
    }
}
