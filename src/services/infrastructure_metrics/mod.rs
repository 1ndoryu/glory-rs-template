/* [225A-4] Sampler de recursos de infraestructura.
 * Coolify sigue siendo el inventario autoritativo; este loop toma promedios
 * aproximados por SSH cada 10 minutos y guarda snapshots para que el panel no
 * dispare SSH en cada render.
 *
 * [01AA-4-F3] Partido por responsabilidad (era god-object 792 + limite 792):
 * types.rs (tipos/consts/estáticos), ssh.rs (comando SSH + parseo),
 * collect.rs (recolección y persistencia). Este mod conserva la API pública. */

mod collect;
mod ssh;
mod types;

use std::collections::HashMap;

use sqlx::PgPool;

use super::types::SAMPLER_INTERVAL;
use super::types::SAMPLER_STARTUP_RETRY_INTERVAL;
use crate::models::HostingSubscription;
use crate::repositories::HostingRepository;
use crate::services::coolify::CoolifyConfig;
use crate::services::infrastructure::coolify_server_targets;

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
        collect::sample_target(
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

    crate::repositories::InfrastructureRepository::purge_old_samples(&pool).await?;
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
