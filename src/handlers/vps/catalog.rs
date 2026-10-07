/* [07AA-14] Catálogo público VPS extraído de `vps.rs`: features derivadas de la
 * config del plan y handler de planes públicos. Sin auth. */

use axum::extract::State;
use axum::Json;

use crate::errors::AppError;
use crate::models::{PublicVpsPlan, VpsPlanConfig};
use crate::repositories::VpsRepository;
use crate::AppState;

fn vps_plan_features(config: &VpsPlanConfig) -> Vec<String> {
    let ram_gb = config.ram_mb / 1024;
    let storage_summary = if config.storage_options.is_empty() {
        format!("{} GB {}", config.disk_mb / 1024, config.storage_type)
    } else {
        config.storage_options.join(" o ")
    };
    vec![
        format!("{} vCPU dedicados", config.cpu_cores),
        format!("{} GB RAM", ram_gb),
        storage_summary,
        format!("Puerto de {} Mbit/s", config.port_speed_mbps),
        config.bandwidth_label.clone(),
        format!(
            "{} snapshot{} incluido{}",
            config.snapshot_count,
            if config.snapshot_count == 1 { "" } else { "s" },
            if config.snapshot_count == 1 { "" } else { "s" }
        ),
        "Acceso root y SSH".to_string(),
        "Hostname y MOTD white-label".to_string(),
        "Entrega verificada antes del provisioning".to_string(),
    ]
}

fn public_plan_from_config(config: VpsPlanConfig) -> PublicVpsPlan {
    let features = vps_plan_features(&config);
    PublicVpsPlan {
        tier_name: config.tier_name,
        display_name: config.display_name,
        description: config.description,
        monthly_price_cents: config.monthly_price_cents,
        setup_fee_cents: config.setup_fee_cents,
        cpu_cores: config.cpu_cores,
        ram_mb: config.ram_mb,
        disk_mb: config.disk_mb,
        storage_type: config.storage_type,
        storage_options: config.storage_options,
        port_speed_mbps: config.port_speed_mbps,
        bandwidth_label: config.bandwidth_label,
        snapshot_count: config.snapshot_count,
        region: config.region,
        region_extra_cents: config.region_extra_cents,
        storage_extra_cents: config.storage_extra_cents,
        features,
        approval_required: config.approval_required,
        recommended: false,
    }
}

#[utoipa::path(
    get,
    path = "/api/vps/public-plans",
    responses(
        (status = 200, description = "Catálogo público de VPS", body = Vec<PublicVpsPlan>),
    ),
    tag = "vps"
)]
pub async fn list_public_plans(
    State(state): State<AppState>,
) -> Result<Json<Vec<PublicVpsPlan>>, AppError> {
    let configs = VpsRepository::list_plan_configs(&state.pool).await?;
    Ok(Json(
        configs.into_iter().map(public_plan_from_config).collect(),
    ))
}
