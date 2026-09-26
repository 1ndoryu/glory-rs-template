/* [259A-4] Bandwidth, cuotas y monitoreo (extraído de infrastructure.rs).
 * Tablas: hosting_subscriptions, bandwidth_snapshots, bandwidth_usage,
 * vps_monitor_state, user_profiles. Sin estado: métodos asociados sobre
 * `InfrastructureRepository` (impl inherente cross-módulo, misma crate). */

use chrono::{DateTime, Datelike, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ResourceUsageReportItem;

use super::infrastructure::{i64_to_f64, InfrastructureRepository};

pub struct BandwidthSnapshotInput<'a> {
    pub subscription_id: Uuid,
    pub deployment_uuid: &'a str,
    pub server_id: Uuid,
    pub net_input_mb: f64,
    pub net_output_mb: f64,
    pub sampled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BandwidthSnapshotRow {
    pub net_input_mb: f64,
    pub net_output_mb: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BandwidthEnforcementCandidate {
    pub subscription_id: Uuid,
    pub deployment_uuid: Option<String>,
    pub server_ip: Option<String>,
    pub status: String,
    pub bandwidth_limit_gb: i32,
    pub bandwidth_used_gb: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BandwidthThrottleCandidate {
    pub subscription_id: Uuid,
    pub deployment_uuid: Option<String>,
    pub coolify_site_name: Option<String>,
    pub server_ip: Option<String>,
    pub server_id: Uuid,
    pub port_speed_mbps: i32,
    pub net_input_mb: f64,
    pub net_output_mb: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CpuBurstCandidate {
    pub subscription_id: Uuid,
    pub deployment_uuid: String,
    pub coolify_site_name: String,
    pub server_ip: String,
    pub server_id: Uuid,
    pub server_cpu_cores: Option<f64>,
    pub server_cpu_percent: Option<f64>,
    pub server_sampled_at: Option<DateTime<Utc>>,
    pub baseline_site_cpu_cores: f64,
    pub cpu_scaling_policy: String,
    pub current_site_cpu_limit_cores: Option<f64>,
    pub deployment_cpu_percent: Option<f64>,
    pub deployment_sampled_at: Option<DateTime<Utc>>,
}

impl InfrastructureRepository {
    pub async fn bandwidth_limit_gb(pool: &PgPool, subscription_id: Uuid) -> Result<i32, AppError> {
        sqlx::query_scalar::<_, i32>(
            "SELECT bandwidth_limit_gb FROM hosting_subscriptions WHERE id = $1",
        )
        .bind(subscription_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn set_subscription_bandwidth_limit(
        pool: &PgPool,
        subscription_id: Uuid,
        bandwidth_limit_gb: i32,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE hosting_subscriptions SET bandwidth_limit_gb = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(bandwidth_limit_gb)
        .bind(subscription_id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn current_bandwidth_gb(
        pool: &PgPool,
        subscription_id: Uuid,
    ) -> Result<Option<f64>, AppError> {
        let month_start = chrono::Utc::now().date_naive().with_day(1).ok_or_else(|| {
            AppError::Internal("No se pudo calcular inicio de mes bandwidth".into())
        })?;
        let bytes = sqlx::query_scalar::<_, Option<i64>>(
            r"SELECT bytes_rx + bytes_tx
               FROM bandwidth_usage
               WHERE subscription_id = $1 AND month_start = $2",
        )
        .bind(subscription_id)
        .bind(month_start)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?
        .flatten();
        Ok(bytes.map(|value| i64_to_f64(value) / 1_000_000_000.0))
    }

    pub async fn find_bandwidth_snapshot(
        pool: &PgPool,
        subscription_id: Uuid,
        deployment_uuid: &str,
    ) -> Result<Option<BandwidthSnapshotRow>, AppError> {
        sqlx::query_as::<_, BandwidthSnapshotRow>(
            r"SELECT net_input_mb, net_output_mb
               FROM bandwidth_snapshots
               WHERE subscription_id = $1 AND deployment_uuid = $2",
        )
        .bind(subscription_id)
        .bind(deployment_uuid)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn upsert_bandwidth_snapshot(
        pool: &PgPool,
        input: BandwidthSnapshotInput<'_>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"INSERT INTO bandwidth_snapshots
                    (subscription_id, deployment_uuid, server_id, net_input_mb, net_output_mb, sampled_at)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (subscription_id, deployment_uuid)
               DO UPDATE SET server_id = EXCLUDED.server_id,
                             net_input_mb = EXCLUDED.net_input_mb,
                             net_output_mb = EXCLUDED.net_output_mb,
                             sampled_at = EXCLUDED.sampled_at",
        )
        .bind(input.subscription_id)
        .bind(input.deployment_uuid)
        .bind(input.server_id)
        .bind(input.net_input_mb)
        .bind(input.net_output_mb)
        .bind(input.sampled_at)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn add_bandwidth_delta(
        pool: &PgPool,
        subscription_id: Uuid,
        rx_bytes: i64,
        tx_bytes: i64,
    ) -> Result<(), AppError> {
        let month_start = chrono::Utc::now().date_naive().with_day(1).ok_or_else(|| {
            AppError::Internal("No se pudo calcular inicio de mes bandwidth".into())
        })?;
        sqlx::query(
            r"INSERT INTO bandwidth_usage (subscription_id, month_start, bytes_rx, bytes_tx, updated_at)
               VALUES ($1, $2, $3, $4, NOW())
               ON CONFLICT (subscription_id, month_start)
               DO UPDATE SET bytes_rx = bandwidth_usage.bytes_rx + EXCLUDED.bytes_rx,
                             bytes_tx = bandwidth_usage.bytes_tx + EXCLUDED.bytes_tx,
                             updated_at = NOW()",
        )
        .bind(subscription_id)
        .bind(month_start)
        .bind(rx_bytes.max(0))
        .bind(tx_bytes.max(0))
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn user_subscription_limit(pool: &PgPool, user_id: Uuid) -> Result<i32, AppError> {
        let limit = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT max_active_subscriptions FROM user_profiles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?
        .flatten()
        .unwrap_or(5);
        Ok(limit)
    }

    pub async fn active_subscription_count(pool: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
        sqlx::query_scalar::<_, i64>(
            r"SELECT COUNT(*)
               FROM hosting_subscriptions
               WHERE user_id = $1 AND status IN ('active', 'provisioning', 'pending')",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn resource_usage_report(
        pool: &PgPool,
    ) -> Result<Vec<ResourceUsageReportItem>, AppError> {
        sqlx::query_as::<_, ResourceUsageReportItem>(
            r"SELECT hs.id AS subscription_id,
                      hs.client_name,
                      hs.domain,
                      hs.plan,
                      hs.status,
                      hs.storage_limit_mb,
                      latest_storage.disk_used_mb AS storage_used_mb,
                      hs.bandwidth_limit_gb,
                      ((bu.bytes_rx + bu.bytes_tx)::float8 / 1000000000.0) AS bandwidth_used_gb,
                      CASE WHEN hs.storage_limit_mb > 0 AND latest_storage.disk_used_mb IS NOT NULL
                           THEN latest_storage.disk_used_mb / hs.storage_limit_mb * 100.0 END AS storage_usage_pct,
                      CASE WHEN hs.bandwidth_limit_gb > 0 AND bu.bytes_rx IS NOT NULL
                           THEN ((bu.bytes_rx + bu.bytes_tx)::float8 / 1000000000.0) / hs.bandwidth_limit_gb * 100.0 END AS bandwidth_usage_pct
               FROM hosting_subscriptions hs
               LEFT JOIN LATERAL (
                   SELECT disk_used_mb
                   FROM infrastructure_resource_samples sample
                   WHERE sample.entity_kind = 'deployment'
                     AND sample.deployment_uuid = hs.server_uuid
                     AND sample.disk_used_mb IS NOT NULL
                   ORDER BY sampled_at DESC
                   LIMIT 1
               ) latest_storage ON TRUE
               LEFT JOIN bandwidth_usage bu
                      ON bu.subscription_id = hs.id
                     AND bu.month_start = date_trunc('month', NOW())::date
               ORDER BY GREATEST(
                   COALESCE(CASE WHEN hs.storage_limit_mb > 0 THEN latest_storage.disk_used_mb / hs.storage_limit_mb * 100.0 ELSE 0 END, 0),
                   COALESCE(CASE WHEN hs.bandwidth_limit_gb > 0 THEN ((bu.bytes_rx + bu.bytes_tx)::float8 / 1000000000.0) / hs.bandwidth_limit_gb * 100.0 ELSE 0 END, 0)
               ) DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn bandwidth_enforcement_candidates(
        pool: &PgPool,
    ) -> Result<Vec<BandwidthEnforcementCandidate>, AppError> {
        sqlx::query_as::<_, BandwidthEnforcementCandidate>(
            r"SELECT hs.id AS subscription_id,
                      hs.server_uuid AS deployment_uuid,
                      hs.server_ip,
                      hs.status,
                      hs.bandwidth_limit_gb,
                      COALESCE((bu.bytes_rx + bu.bytes_tx)::float8 / 1000000000.0, 0.0) AS bandwidth_used_gb
               FROM hosting_subscriptions hs
               LEFT JOIN bandwidth_usage bu
                      ON bu.subscription_id = hs.id
                     AND bu.month_start = date_trunc('month', NOW())::date
               WHERE hs.status IN ('active', 'suspended_bandwidth')
                 AND hs.server_uuid IS NOT NULL
               ORDER BY bandwidth_used_gb DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn bandwidth_throttle_candidates(
        pool: &PgPool,
    ) -> Result<Vec<BandwidthThrottleCandidate>, AppError> {
        sqlx::query_as::<_, BandwidthThrottleCandidate>(
            r"SELECT hs.id AS subscription_id,
                      hs.server_uuid AS deployment_uuid,
                      hs.coolify_site_name,
                      hs.server_ip,
                      s.id AS server_id,
                      s.port_speed_mbps,
                      COALESCE(bs.net_input_mb, 0.0) AS net_input_mb,
                      COALESCE(bs.net_output_mb, 0.0) AS net_output_mb
               FROM hosting_subscriptions hs
               JOIN infrastructure_servers s ON s.server_ip = hs.server_ip AND s.is_active = TRUE
               LEFT JOIN bandwidth_snapshots bs
                      ON bs.subscription_id = hs.id
                     AND bs.deployment_uuid = hs.server_uuid
               WHERE hs.status = 'active'
                 AND hs.server_uuid IS NOT NULL
                 AND hs.server_ip IS NOT NULL",
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn cpu_burst_candidates(pool: &PgPool) -> Result<Vec<CpuBurstCandidate>, AppError> {
        sqlx::query_as::<_, CpuBurstCandidate>(
                        r"SELECT hs.id AS subscription_id,
                                            COALESCE(hs.deployment_id, hs.server_uuid) AS deployment_uuid,
                                            hs.coolify_site_name,
                                            hs.server_ip,
                                            s.id AS server_id,
                                            cap.cpu_cores AS server_cpu_cores,
                                            server_sample.cpu_percent AS server_cpu_percent,
                                            server_sample.sampled_at AS server_sampled_at,
                                            plan.wp_cpu_millicores::float8 / 1000.0 AS baseline_site_cpu_cores,
                                            plan.cpu_scaling_policy,
                                            deploy_sample.site_cpu_limit_cores AS current_site_cpu_limit_cores,
                                            deploy_sample.cpu_percent AS deployment_cpu_percent,
                                            deploy_sample.sampled_at AS deployment_sampled_at
                             FROM hosting_subscriptions hs
                             JOIN infrastructure_servers s
                                 ON s.server_ip = hs.server_ip
                                AND s.is_active = TRUE
                             JOIN hosting_plan_configs plan
                                 ON plan.plan_name = hs.plan
                             LEFT JOIN LATERAL (
                                     SELECT cpu_cores::float8 AS cpu_cores
                                     FROM server_capacity
                                     WHERE server_id = s.id OR server_uuid = s.coolify_server_uuid
                                     ORDER BY updated_at DESC
                                     LIMIT 1
                             ) cap ON TRUE
                             LEFT JOIN LATERAL (
                                     SELECT cpu_percent, sampled_at
                                     FROM infrastructure_resource_samples
                                     WHERE entity_kind = 'server'
                                         AND server_id = s.id
                                     ORDER BY sampled_at DESC
                                     LIMIT 1
                             ) server_sample ON TRUE
                             LEFT JOIN LATERAL (
                                     SELECT cpu_percent, site_cpu_limit_cores, sampled_at
                                     FROM infrastructure_resource_samples
                                     WHERE entity_kind = 'deployment'
                                         AND deployment_uuid = COALESCE(hs.deployment_id, hs.server_uuid)
                                     ORDER BY sampled_at DESC
                                     LIMIT 1
                             ) deploy_sample ON TRUE
                             WHERE hs.status = 'active'
                                 AND hs.coolify_site_name IS NOT NULL
                                 AND hs.server_ip IS NOT NULL
                                 AND COALESCE(hs.deployment_id, hs.server_uuid) IS NOT NULL
                                 AND COALESCE(NULLIF(lower(hs.runtime_kind), ''), 'coolify') = 'coolify'
                             ORDER BY s.id ASC, hs.created_at ASC",
                )
                .fetch_all(pool)
                .await
                .map_err(AppError::from)
    }

    pub async fn upsert_vps_monitor_state(
        pool: &PgPool,
        subscription_id: Uuid,
        provider_status: Option<&str>,
        failure_count: i32,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"INSERT INTO vps_monitor_state
                    (subscription_id, provider_status, failure_count, checked_at, updated_at)
               VALUES ($1, $2, $3, NOW(), NOW())
               ON CONFLICT (subscription_id)
               DO UPDATE SET provider_status = EXCLUDED.provider_status,
                             failure_count = EXCLUDED.failure_count,
                             checked_at = NOW(),
                             updated_at = NOW()",
        )
        .bind(subscription_id)
        .bind(provider_status)
        .bind(failure_count)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn vps_monitor_failure_count(
        pool: &PgPool,
        subscription_id: Uuid,
    ) -> Result<i32, AppError> {
        let count = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT failure_count FROM vps_monitor_state WHERE subscription_id = $1",
        )
        .bind(subscription_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?
        .flatten()
        .unwrap_or(0);
        Ok(count)
    }
}
