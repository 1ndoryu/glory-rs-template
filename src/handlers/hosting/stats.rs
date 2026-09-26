use axum::extract::{Path, State};
use axum::Json;
use chrono::{Datelike, TimeZone};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{HostingEvent, HostingStatsResponse, UserRole};
use crate::repositories::{HostingRepository, InfrastructureRepository};
use crate::AppState;

/* [265A-6] Cambiado a pub(crate) para permitir acceso desde backups handler. */
pub(crate) fn resolve_ssh_key<'a>(state: &'a AppState, server_ip: &str) -> Option<&'a str> {
    for target in crate::services::infrastructure::coolify_server_targets(
        state.coolify_config_vps1.as_ref(),
        state.coolify_config.as_ref(),
    ) {
        if target.config.server_ip == server_ip {
            return target.config.ssh_key_path.as_deref();
        }
    }

    state
        .coolify_config
        .as_ref()
        .and_then(|c| c.ssh_key_path.as_deref())
        .or_else(|| {
            state
                .coolify_config_vps1
                .as_ref()
                .and_then(|c| c.ssh_key_path.as_deref())
        })
}

/* [114A-15+] Obtiene estadísticas de contenedores Docker via SSH.
 * Usa cache de 30s para evitar SSH en cada request.
 * Retorna (cpu_percent, ram_used, ram_limit, containers) — todo None si SSH no disponible.
 * [215A-14] Ahora resuelve la SSH key correcta según el server_ip de la suscripción. */
pub(super) async fn fetch_container_resources(
    state: &AppState,
    sub: &crate::models::HostingSubscription,
) -> (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<Vec<crate::services::docker_stats::ContainerStats>>,
) {
    let coolify_name = match &sub.coolify_site_name {
        Some(name) if !name.is_empty() => name,
        _ => return (None, None, None, None),
    };
    let server_ip = match &sub.server_ip {
        Some(ip) if !ip.is_empty() => ip,
        _ => return (None, None, None, None),
    };
    let Some(ssh_key) = resolve_ssh_key(state, server_ip) else {
        return (None, None, None, None);
    };

    let cache_key = format!("{server_ip}:{coolify_name}");
    if let Some(cached) = state.docker_stats_cache.get(&cache_key).await {
        return (
            Some(cached.total_cpu_percent),
            Some(cached.total_ram_used_mb),
            Some(cached.total_ram_limit_mb),
            Some(cached.containers),
        );
    }

    match crate::services::docker_stats::fetch_docker_stats(server_ip, ssh_key, coolify_name).await
    {
        Ok(resource_stats) => {
            let result = (
                Some(resource_stats.total_cpu_percent),
                Some(resource_stats.total_ram_used_mb),
                Some(resource_stats.total_ram_limit_mb),
                Some(resource_stats.containers.clone()),
            );
            state
                .docker_stats_cache
                .set(cache_key, resource_stats)
                .await;
            result
        }
        Err(error) => {
            tracing::warn!("Docker stats fallo para {coolify_name}@{server_ip}: {error}");
            (None, None, None, None)
        }
    }
}

/* [154A-3] Obtiene el uso de disco real del contenedor WordPress via SSH.
 * Sigue el mismo patrón que fetch_container_resources: requiere SSH key + server_ip.
 * [215A-14] Ahora resuelve SSH key via resolve_ssh_key para soportar VPS1 y VPS2. */
pub(super) async fn fetch_storage_used(
    state: &AppState,
    sub: &crate::models::HostingSubscription,
) -> Option<i64> {
    let coolify_name = sub.coolify_site_name.as_deref().filter(|n| !n.is_empty())?;
    let server_ip = sub.server_ip.as_deref().filter(|ip| !ip.is_empty())?;
    let ssh_key = resolve_ssh_key(state, server_ip)?;

    match crate::services::docker_stats::fetch_storage_usage(
        server_ip,
        ssh_key,
        coolify_name,
        sub.deployment_id_or_legacy(),
        &sub.plan,
    )
    .await
    {
        Ok(mb) => Some(mb),
        Err(error) => {
            tracing::warn!("Storage check fallo para {coolify_name}@{server_ip}: {error}");
            None
        }
    }
}

fn f64_to_i64_rounded(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    format!("{value:.0}").parse::<i64>().ok()
}

/* [094A-8] Calcula el porcentaje de uptime analizando transiciones de status en eventos.
 * Recorre los eventos cronológicamente, contando el tiempo total en estado "active".
 * Si la suscripción actualmente está activa, el período abierto se extiende hasta ahora. */
#[allow(clippy::cast_precision_loss)]
pub(super) fn calculate_uptime(
    created_at: chrono::DateTime<chrono::Utc>,
    current_status: &str,
    events: &[HostingEvent],
) -> (f64, Option<chrono::DateTime<chrono::Utc>>) {
    let now = chrono::Utc::now();
    let total_duration = (now - created_at).num_seconds().max(1) as f64;
    let mut active_seconds: f64 = 0.0;
    let mut last_active_start: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut first_active: Option<chrono::DateTime<chrono::Utc>> = None;

    let mut sorted_events: Vec<&HostingEvent> = events.iter().collect();
    sorted_events.sort_by_key(|event| event.created_at);

    for event in &sorted_events {
        if event.event_type != "status_change" {
            continue;
        }
        let new_status = event
            .details
            .as_ref()
            .and_then(|details| details.get("new_status"))
            .and_then(|value| value.as_str())
            .unwrap_or("");

        match new_status {
            "active" => {
                last_active_start = Some(event.created_at);
                if first_active.is_none() {
                    first_active = Some(event.created_at);
                }
            }
            _ => {
                if let Some(start) = last_active_start.take() {
                    active_seconds += (event.created_at - start).num_seconds().max(0) as f64;
                }
            }
        }
    }

    if current_status == "active" {
        if let Some(start) = last_active_start {
            active_seconds += (now - start).num_seconds().max(0) as f64;
        } else if first_active.is_none() {
            first_active = Some(created_at);
            active_seconds = total_duration;
        }
    }

    let uptime = if total_duration > 0.0 {
        (active_seconds / total_duration * 100.0).min(100.0)
    } else {
        0.0
    };

    (uptime, first_active)
}

/// Estadísticas de uso de una suscripción de hosting
#[utoipa::path(
    get,
    path = "/api/hosting/subscriptions/{id}/stats",
    params(("id" = Uuid, Path, description = "ID de la suscripción")),
    responses(
        (status = 200, description = "Estadísticas de la suscripción", body = HostingStatsResponse),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripción no encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn get_hosting_stats(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<HostingStatsResponse>, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if auth.effective_role == UserRole::Client && sub.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden("Sin permisos".into()));
    }

    let events = HostingRepository::list_events(&state.pool, id, 1000).await?;
    let (uptime_percent, active_since) = calculate_uptime(sub.created_at, &sub.status, &events);
    let total_events = i64::try_from(events.len()).unwrap_or(i64::MAX);
    let last_event_at = events.first().map(|event| event.created_at);
    let monitoring_available = sub.coolify_site_name.is_some();
    let bandwidth = InfrastructureRepository::bandwidth_limit_gb(&state.pool, sub.id).await?;
    let bandwidth_used_gb = InfrastructureRepository::current_bandwidth_gb(&state.pool, sub.id)
        .await?
        .or(Some(0.0));
    let bandwidth_remaining_gb = if bandwidth < 0 {
        None
    } else {
        Some((f64::from(bandwidth) - bandwidth_used_gb.unwrap_or(0.0)).max(0.0))
    };
    let now = chrono::Utc::now();
    let next_month_year = if now.month() == 12 {
        now.year() + 1
    } else {
        now.year()
    };
    let next_month = if now.month() == 12 {
        1
    } else {
        now.month() + 1
    };
    let bandwidth_reset_at = chrono::Utc
        .with_ymd_and_hms(next_month_year, next_month, 1, 0, 0, 0)
        .single()
        .unwrap_or(now);
    let latest_sample = if let Some(deployment_uuid) = sub.deployment_id_or_legacy() {
        InfrastructureRepository::latest_deployment_sample(&state.pool, deployment_uuid).await?
    } else {
        None
    };
    let storage_from_sample = latest_sample
        .as_ref()
        .and_then(|sample| sample.disk_used_mb.and_then(f64_to_i64_rounded));
    let (cpu_percent, ram_used_mb, ram_limit_mb, containers) = if let Some(sample) = &latest_sample
    {
        (
            sample.cpu_percent,
            sample.ram_used_mb,
            sample.ram_limit_mb,
            None,
        )
    } else {
        fetch_container_resources(&state, &sub).await
    };
    let storage_used_mb = if storage_from_sample.is_some() {
        storage_from_sample
    } else {
        fetch_storage_used(&state, &sub).await
    };

    Ok(Json(HostingStatsResponse {
        storage_limit_mb: sub.storage_limit_mb,
        storage_used_mb,
        bandwidth_limit_gb: bandwidth,
        bandwidth_used_gb,
        bandwidth_remaining_gb,
        bandwidth_reset_at,
        uptime_percent,
        active_since,
        total_events,
        last_event_at,
        monitoring_available,
        cpu_percent,
        ram_used_mb,
        ram_limit_mb,
        containers,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_event(
        event_type: &str,
        status: &str,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> HostingEvent {
        HostingEvent {
            id: Uuid::new_v4(),
            subscription_id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            details: Some(serde_json::json!({"new_status": status})),
            created_at,
        }
    }

    #[test]
    fn calculate_uptime_no_events_active_status() {
        let created = chrono::Utc::now() - chrono::Duration::hours(24);
        let events = vec![];

        let (uptime, active_since) = calculate_uptime(created, "active", &events);

        assert!(uptime > 99.0);
        assert_eq!(active_since, Some(created));
    }

    #[test]
    fn calculate_uptime_ignores_non_status_events() {
        let created = chrono::Utc::now() - chrono::Duration::hours(2);
        let events = vec![status_event(
            "created",
            "active",
            chrono::Utc::now() - chrono::Duration::hours(1),
        )];

        let (uptime, active_since) = calculate_uptime(created, "pending", &events);

        /* [259A-1] Comparacion exacta de f32 prohibida por clippy (float_cmp). */
        assert!(uptime.abs() < f64::EPSILON);
        assert!(active_since.is_none());
    }
}
