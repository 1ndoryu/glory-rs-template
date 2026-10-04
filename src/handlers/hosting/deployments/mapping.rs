/* [01AA-4-f3o] Inventario mínimo de despliegues (extraido de deployments.rs).
 * Mapea resúmenes de runtime a CoolifyDeploymentResponse, reconstruye el
 * inventario desde suscripciones cuando un runtime falla ([255A-2]) y
 * enriquece con snapshots del sampler en vez de SSH en render ([225A-4]). */

use std::collections::{HashMap, HashSet};

use super::super::deployment_helpers::{
    resolve_server_label, runtime_link_key, FailedRuntimeLookup, PendingRuntimeDeployments,
};
use crate::models::{CoolifyDeploymentResponse, HostingSubscription};
use crate::repositories::InfrastructureRepository;
use crate::services::{HostingRuntimeDeploymentSummary, HostingRuntimeKind};
use crate::AppState;

pub(super) fn map_runtime_deployments(
    services: Vec<HostingRuntimeDeploymentSummary>,
    fallback_label: &str,
    duplicate_name_keys: &HashSet<String>,
    subscriptions_by_uuid: &HashMap<String, &crate::models::HostingSubscription>,
    subscriptions_by_name: &HashMap<String, &crate::models::HostingSubscription>,
    plan_configs_by_name: &HashMap<String, crate::models::HostingPlanConfig>,
) -> Vec<CoolifyDeploymentResponse> {
    services
        .into_iter()
        .map(|service| {
            let deployment_key = runtime_link_key(service.runtime_kind, &service.deployment_id);
            let name_key = runtime_link_key(service.runtime_kind, &service.name);
            let linked_subscription =
                subscriptions_by_uuid
                    .get(&deployment_key)
                    .copied()
                    .or_else(|| {
                        (!duplicate_name_keys.contains(&name_key))
                            .then(|| subscriptions_by_name.get(&name_key).copied())
                            .flatten()
                    });

            let server_label = resolve_server_label(&service, fallback_label);
            let plan_config =
                linked_subscription.and_then(|sub| plan_configs_by_name.get(&sub.plan));

            CoolifyDeploymentResponse {
                uuid: service.deployment_id.clone(),
                runtime_kind: service.runtime_kind.as_str().to_string(),
                deployment_id: service.deployment_id,
                name: service.name,
                status: service.status,
                fqdn: service.fqdn,
                server_uuid: service.target_id,
                server_name: Some(server_label.clone()),
                project_uuid: service.project_id,
                environment_name: service.environment_name,
                linked_subscription_id: linked_subscription.map(|subscription| subscription.id),
                linked_subscription_domain: linked_subscription
                    .and_then(|subscription| subscription.domain.clone()),
                linked_subscription_status: linked_subscription
                    .map(|subscription| subscription.status.clone()),
                linked_subscription_plan: linked_subscription
                    .map(|subscription| subscription.plan.clone()),
                linked_subscription_client: linked_subscription
                    .map(|subscription| subscription.client_name.clone()),
                runtime_sampled_at: None,
                storage_limit_mb: linked_subscription
                    .map(|subscription| subscription.storage_limit_mb),
                runtime_site_cpu_limit_cores: None,
                runtime_site_ram_limit_mb: None,
                runtime_db_cpu_limit_cores: None,
                runtime_db_ram_limit_mb: None,
                runtime_ssh_cpu_limit_cores: None,
                runtime_ssh_ram_limit_mb: None,
                plan_wp_cpu_millicores: plan_config.map(|c| c.wp_cpu_millicores),
                plan_db_cpu_millicores: plan_config.map(|c| c.db_cpu_millicores),
                plan_ssh_cpu_millicores: plan_config.map(|c| c.ssh_cpu_millicores),
                plan_wp_memory_mb: plan_config.map(|c| c.wp_memory_mb),
                plan_db_memory_mb: plan_config.map(|c| c.db_memory_mb),
                plan_ssh_memory_mb: plan_config.map(|c| c.ssh_memory_mb),
                cpu_percent: None,
                ram_used_mb: None,
                ram_limit_mb: None,
                storage_used_mb: None,
                server_label,
            }
        })
        .collect()
}

pub(super) fn f64_to_i64_rounded(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    format!("{value:.0}").parse::<i64>().ok()
}

pub(super) fn should_include_subscription_fallback(subscription: &HostingSubscription) -> bool {
    subscription.deployment_id_or_legacy().is_some()
        && !subscription.status.trim().eq_ignore_ascii_case("cancelled")
}

pub(super) fn subscription_fallback_name(
    subscription: &HostingSubscription,
    deployment_id: &str,
) -> String {
    subscription
        .coolify_site_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            subscription
                .domain
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| deployment_id.to_string())
}

fn subscription_matches_failed_lookup(
    subscription: &HostingSubscription,
    failed_lookup: &FailedRuntimeLookup,
    failed_lookup_count: usize,
) -> bool {
    if HostingRuntimeKind::from_persisted(&subscription.runtime_kind) != failed_lookup.runtime_kind
        || !should_include_subscription_fallback(subscription)
    {
        return false;
    }

    match (
        failed_lookup.target_server_ip.as_deref(),
        subscription.server_ip.as_deref(),
    ) {
        (Some(expected_ip), Some(actual_ip)) => actual_ip == expected_ip,
        (Some(_), None) => failed_lookup_count == 1,
        (None, _) => true,
    }
}

/* [255A-2] Cuando Coolify responde 500, el panel no debe degradar a "cero despliegues".
 * Si un runtime concreto falla, reconstruimos un inventario mínimo desde suscripciones
 * persistidas para conservar la tabla utilizable hasta que el proveedor vuelva. */
pub(super) fn build_failed_runtime_fallback_batches(
    subscriptions: &[HostingSubscription],
    failed_lookups: &[FailedRuntimeLookup],
) -> Vec<PendingRuntimeDeployments> {
    failed_lookups
        .iter()
        .filter_map(|failed_lookup| {
            let failed_lookup_count = failed_lookups
                .iter()
                .filter(|lookup| lookup.runtime_kind == failed_lookup.runtime_kind)
                .count();

            let services: Vec<_> = subscriptions
                .iter()
                .filter(|subscription| {
                    subscription_matches_failed_lookup(
                        subscription,
                        failed_lookup,
                        failed_lookup_count,
                    )
                })
                .filter_map(|subscription| {
                    let deployment_id = subscription.deployment_id_or_legacy()?;
                    Some(HostingRuntimeDeploymentSummary {
                        runtime_kind: HostingRuntimeKind::from_persisted(
                            &subscription.runtime_kind,
                        ),
                        deployment_id: deployment_id.to_string(),
                        name: subscription_fallback_name(subscription, deployment_id),
                        status: subscription.status.clone(),
                        fqdn: subscription
                            .domain
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned),
                        target_id: None,
                        target_name: Some(failed_lookup.fallback_label.clone()),
                        project_id: None,
                        environment_name: None,
                    })
                })
                .collect();

            (!services.is_empty()).then_some(PendingRuntimeDeployments {
                fallback_label: failed_lookup.fallback_label.clone(),
                services,
            })
        })
        .collect()
}

pub(super) fn dedupe_deployments(
    deployments: Vec<CoolifyDeploymentResponse>,
) -> Vec<CoolifyDeploymentResponse> {
    let mut seen = HashSet::new();

    deployments
        .into_iter()
        .filter(|deployment| {
            seen.insert(format!(
                "{}::{}",
                deployment.runtime_kind, deployment.deployment_id
            ))
        })
        .collect()
}

pub(super) fn failed_runtime_labels(failed_lookups: &[FailedRuntimeLookup]) -> String {
    let mut seen = HashSet::new();

    failed_lookups
        .iter()
        .filter_map(|failed_lookup| {
            let label = format!(
                "{} ({})",
                failed_lookup.fallback_label,
                failed_lookup.runtime_kind.as_str()
            );
            seen.insert(label.clone()).then_some(label)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/* [225A-4] Enriquece despliegues desde snapshots del sampler, no desde SSH en render.
 * Si aún no hay muestras, el panel muestra guiones hasta que el loop background
 * capture el primer promedio. */
pub(super) async fn enrich_deployment_resources(
    state: &AppState,
    deployments: &mut [CoolifyDeploymentResponse],
) {
    for deployment in deployments {
        match InfrastructureRepository::latest_deployment_sample(&state.pool, &deployment.uuid)
            .await
        {
            Ok(Some(sample)) => {
                deployment.runtime_sampled_at = Some(sample.sampled_at);
                deployment.cpu_percent = sample.cpu_percent;
                deployment.ram_used_mb = sample.ram_used_mb;
                deployment.ram_limit_mb = sample.ram_limit_mb;
                deployment.storage_used_mb = sample.disk_used_mb.and_then(f64_to_i64_rounded);
                deployment.runtime_site_cpu_limit_cores = sample.site_cpu_limit_cores;
                deployment.runtime_site_ram_limit_mb = sample.site_ram_limit_mb;
                deployment.runtime_db_cpu_limit_cores = sample.db_cpu_limit_cores;
                deployment.runtime_db_ram_limit_mb = sample.db_ram_limit_mb;
                deployment.runtime_ssh_cpu_limit_cores = sample.ssh_cpu_limit_cores;
                deployment.runtime_ssh_ram_limit_mb = sample.ssh_ram_limit_mb;
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(
                "[deployments] No se pudo leer snapshot para {}: {error}",
                deployment.uuid
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn sample_subscription(
        runtime_kind: &str,
        status: &str,
        deployment_id: Option<&str>,
        coolify_site_name: Option<&str>,
        server_ip: Option<&str>,
    ) -> HostingSubscription {
        let now = Utc::now();

        HostingSubscription {
            id: Uuid::new_v4(),
            user_id: None,
            client_name: "Cliente Test".to_string(),
            client_email: "test@example.com".to_string(),
            plan: "normal-mini".to_string(),
            domain: Some("example.test".to_string()),
            domain_verification_status: "pending".to_string(),
            domain_verification_token: None,
            domain_verified_at: None,
            runtime_kind: runtime_kind.to_string(),
            deployment_id: deployment_id.map(str::to_string),
            coolify_site_name: coolify_site_name.map(str::to_string),
            status: status.to_string(),
            stripe_subscription_id: None,
            monthly_price_cents: 1000,
            storage_limit_mb: 1024,
            server_uuid: None,
            server_ip: server_ip.map(str::to_string),
            sftp_user: None,
            sftp_password: None,
            sftp_port: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn build_failed_runtime_fallback_batches_filters_runtime_status_and_target() {
        let failed_lookups = vec![FailedRuntimeLookup {
            fallback_label: "VPS2".to_string(),
            runtime_kind: HostingRuntimeKind::Coolify,
            target_server_ip: Some("173.249.50.44".to_string()),
        }];
        let subscriptions = vec![
            sample_subscription(
                "coolify",
                "active",
                Some("dep-ok"),
                Some("hosting-ok"),
                Some("173.249.50.44"),
            ),
            sample_subscription(
                "coolify",
                "cancelled",
                Some("dep-cancelled"),
                Some("hosting-cancelled"),
                Some("173.249.50.44"),
            ),
            sample_subscription(
                "coolify",
                "active",
                Some("dep-other-ip"),
                Some("hosting-other-ip"),
                Some("66.94.100.241"),
            ),
            sample_subscription(
                "lightweight",
                "active",
                Some("dep-lightweight"),
                Some("hosting-lightweight"),
                Some("173.249.50.44"),
            ),
            sample_subscription(
                "coolify",
                "active",
                None,
                Some("hosting-without-id"),
                Some("173.249.50.44"),
            ),
        ];

        let batches = build_failed_runtime_fallback_batches(&subscriptions, &failed_lookups);

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].fallback_label, "VPS2");
        assert_eq!(batches[0].services.len(), 1);
        assert_eq!(batches[0].services[0].deployment_id, "dep-ok");
        assert_eq!(batches[0].services[0].name, "hosting-ok");
    }

    #[test]
    fn build_failed_runtime_fallback_batches_accepts_missing_server_ip_if_runtime_failure_is_unique(
    ) {
        let failed_lookups = vec![FailedRuntimeLookup {
            fallback_label: "VPS2".to_string(),
            runtime_kind: HostingRuntimeKind::Coolify,
            target_server_ip: Some("173.249.50.44".to_string()),
        }];
        let subscriptions = vec![sample_subscription(
            "coolify",
            "active",
            Some("dep-without-ip"),
            Some("hosting-without-ip"),
            None,
        )];

        let batches = build_failed_runtime_fallback_batches(&subscriptions, &failed_lookups);

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].services.len(), 1);
        assert_eq!(batches[0].services[0].deployment_id, "dep-without-ip");
    }
}
