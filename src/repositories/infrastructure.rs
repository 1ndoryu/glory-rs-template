/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro limite-lineas:
 * repositorio incremental de métricas con queries runtime preparadas para evitar
 * regenerar todo el cache SQLx del CRUD legacy durante la migración de infraestructura.
 * limite-lineas: 709 líneas, supera 550 — dividir requiere refactor mayor (mover
 * CapacityRepository, BandwidthRepository y MetricsRepository) que se aborda en tarea
 * separada para no mezclar dominios en este bloque. */

use chrono::{DateTime, Datelike, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    InfrastructureServerMetricsResponse, ResourceMetricPoint, ResourceUsageReportItem,
};
use crate::services::coolify::CoolifyConfig;

fn i64_to_f64(value: i64) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0)
}

fn millicores_to_cores(value: i64) -> f64 {
    i64_to_f64(value) / 1000.0
}

pub struct InfrastructureRepository;

pub struct ConfiguredServerInput<'a> {
    pub label: &'a str,
    pub config: &'a CoolifyConfig,
    pub secret_ref: &'a str,
    pub ssh_secret_ref: Option<&'a str>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InfrastructureServerRecord {
    pub id: Uuid,
    pub label: String,
    pub server_ip: String,
    pub coolify_server_uuid: Option<String>,
}

pub struct ResourceSampleInput<'a> {
    pub entity_kind: &'a str,
    pub server_id: Uuid,
    pub deployment_uuid: Option<&'a str>,
    pub sampled_at: DateTime<Utc>,
    pub cpu_percent: Option<f64>,
    pub ram_used_mb: Option<f64>,
    pub ram_limit_mb: Option<f64>,
    pub disk_used_mb: Option<f64>,
    pub disk_limit_mb: Option<f64>,
    pub site_cpu_limit_cores: Option<f64>,
    pub site_ram_limit_mb: Option<f64>,
    pub db_cpu_limit_cores: Option<f64>,
    pub db_ram_limit_mb: Option<f64>,
    pub ssh_cpu_limit_cores: Option<f64>,
    pub ssh_ram_limit_mb: Option<f64>,
}

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
pub struct HostingResourceAllocation {
    pub cpu_millicores: i64,
    pub ram_mb: i64,
    pub disk_mb: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ServerCapacityRow {
    pub server_uuid: String,
    pub cpu_cores: f64,
    pub ram_mb: i32,
    pub disk_mb: i32,
    pub cpu_allocated: f64,
    pub ram_allocated: i32,
    pub disk_allocated: i32,
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
    pub async fn upsert_configured_server(
        pool: &PgPool,
        input: ConfiguredServerInput<'_>,
    ) -> Result<InfrastructureServerRecord, AppError> {
        /* [225A-4] Dos-step: primero buscar por coolify_server_uuid (para cubrir cambios
         * de server_ip/coolify_base_url), luego upsert por (server_ip, coolify_base_url).
         * Sin esto, cambiar coolify_base_url (ej: 66.94.100.241:8000 → coolify:8080) causa
         * que ON CONFLICT no detecte el match y se inserte un duplicado de coolify_server_uuid. */
        if !input.config.server_uuid.is_empty() {
            let existing = sqlx::query_as::<_, InfrastructureServerRecord>(
                r"UPDATE infrastructure_servers
                     SET label = $2,
                         server_ip = $3,
                         coolify_base_url = $4,
                         coolify_project_uuid = $5,
                         secret_ref = $6,
                         ssh_secret_ref = $7,
                         is_active = TRUE,
                         updated_at = NOW()
                   WHERE coolify_server_uuid = $1
                  RETURNING id, label, server_ip, coolify_server_uuid",
            )
            .bind(&input.config.server_uuid)
            .bind(input.label)
            .bind(&input.config.server_ip)
            .bind(&input.config.base_url)
            .bind(&input.config.project_uuid)
            .bind(input.secret_ref)
            .bind(input.ssh_secret_ref)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)?;

            if let Some(record) = existing {
                return Ok(record);
            }
        }

        sqlx::query_as::<_, InfrastructureServerRecord>(
            r"INSERT INTO infrastructure_servers
                    (label, provider, server_ip, coolify_base_url, coolify_server_uuid,
                     coolify_project_uuid, secret_ref, ssh_secret_ref, is_active, updated_at)
               VALUES ($1, 'coolify', $2, $3, $4, $5, $6, $7, TRUE, NOW())
               ON CONFLICT (server_ip, coolify_base_url)
               DO UPDATE SET label = EXCLUDED.label,
                             coolify_server_uuid = EXCLUDED.coolify_server_uuid,
                             coolify_project_uuid = EXCLUDED.coolify_project_uuid,
                             secret_ref = EXCLUDED.secret_ref,
                             ssh_secret_ref = EXCLUDED.ssh_secret_ref,
                             is_active = TRUE,
                             updated_at = NOW()
               RETURNING id, label, server_ip, coolify_server_uuid",
        )
        .bind(input.label)
        .bind(&input.config.server_ip)
        .bind(&input.config.base_url)
        .bind(&input.config.server_uuid)
        .bind(&input.config.project_uuid)
        .bind(input.secret_ref)
        .bind(input.ssh_secret_ref)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn list_servers_with_metrics(
        pool: &PgPool,
    ) -> Result<Vec<InfrastructureServerMetricsResponse>, AppError> {
        sqlx::query_as::<_, InfrastructureServerMetricsResponse>(
            r"SELECT s.id,
                      s.label,
                      s.provider,
                      s.server_ip,
                      s.status,
                      s.is_active,
                      s.coolify_server_uuid,
                      c.cpu_cores::float8 AS cpu_cores,
                      c.ram_mb,
                      c.disk_mb,
                      m.cpu_avg_1h,
                      latest.ram_used_mb,
                      latest.ram_limit_mb,
                      latest.disk_used_mb,
                      latest.disk_limit_mb,
                      latest.sampled_at
               FROM infrastructure_servers s
               LEFT JOIN server_capacity c ON c.server_id = s.id OR c.server_uuid = s.coolify_server_uuid
               LEFT JOIN LATERAL (
                   SELECT AVG(cpu_percent) AS cpu_avg_1h
                   FROM infrastructure_resource_samples sample
                   WHERE sample.entity_kind = 'server'
                     AND sample.server_id = s.id
                     AND sample.sampled_at > NOW() - INTERVAL '1 hour'
               ) m ON TRUE
               LEFT JOIN LATERAL (
                   SELECT ram_used_mb, ram_limit_mb, disk_used_mb, disk_limit_mb, sampled_at
                   FROM infrastructure_resource_samples sample
                   WHERE sample.entity_kind = 'server'
                     AND sample.server_id = s.id
                   ORDER BY sampled_at DESC
                   LIMIT 1
               ) latest ON TRUE
               WHERE s.is_active = TRUE
               ORDER BY s.label ASC",
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn insert_sample(
        pool: &PgPool,
        input: ResourceSampleInput<'_>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"INSERT INTO infrastructure_resource_samples
                    (entity_kind, server_id, deployment_uuid, sampled_at, cpu_percent,
                     ram_used_mb, ram_limit_mb, disk_used_mb, disk_limit_mb,
                     site_cpu_limit_cores, site_ram_limit_mb, db_cpu_limit_cores,
                     db_ram_limit_mb, ssh_cpu_limit_cores, ssh_ram_limit_mb)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(input.entity_kind)
        .bind(input.server_id)
        .bind(input.deployment_uuid)
        .bind(input.sampled_at)
        .bind(input.cpu_percent)
        .bind(input.ram_used_mb)
        .bind(input.ram_limit_mb)
        .bind(input.disk_used_mb)
        .bind(input.disk_limit_mb)
        .bind(input.site_cpu_limit_cores)
        .bind(input.site_ram_limit_mb)
        .bind(input.db_cpu_limit_cores)
        .bind(input.db_ram_limit_mb)
        .bind(input.ssh_cpu_limit_cores)
        .bind(input.ssh_ram_limit_mb)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn deployment_metric_points(
        pool: &PgPool,
        deployment_uuid: &str,
        range_hours: i64,
    ) -> Result<Vec<ResourceMetricPoint>, AppError> {
        sqlx::query_as::<_, ResourceMetricPoint>(
            r"SELECT to_timestamp(floor(extract(epoch from sampled_at) / 600) * 600) AS sampled_at,
                      AVG(cpu_percent) AS cpu_percent,
                      AVG(ram_used_mb) AS ram_used_mb,
                      MAX(ram_limit_mb) AS ram_limit_mb,
                      MAX(disk_used_mb) AS disk_used_mb,
                                            MAX(disk_limit_mb) AS disk_limit_mb,
                                            MAX(site_cpu_limit_cores) AS site_cpu_limit_cores,
                                            MAX(site_ram_limit_mb) AS site_ram_limit_mb,
                                            MAX(db_cpu_limit_cores) AS db_cpu_limit_cores,
                                            MAX(db_ram_limit_mb) AS db_ram_limit_mb,
                                            MAX(ssh_cpu_limit_cores) AS ssh_cpu_limit_cores,
                                            MAX(ssh_ram_limit_mb) AS ssh_ram_limit_mb
               FROM infrastructure_resource_samples
               WHERE entity_kind = 'deployment'
                 AND deployment_uuid = $1
                 AND sampled_at > NOW() - ($2::text || ' hours')::interval
               GROUP BY 1
               ORDER BY 1 ASC",
        )
        .bind(deployment_uuid)
        .bind(range_hours.clamp(1, 168))
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn latest_deployment_sample(
        pool: &PgPool,
        deployment_uuid: &str,
    ) -> Result<Option<ResourceMetricPoint>, AppError> {
        sqlx::query_as::<_, ResourceMetricPoint>(
            r"SELECT latest.sampled_at,
                      latest.cpu_percent,
                      latest.ram_used_mb,
                      latest.ram_limit_mb,
                      COALESCE(latest.disk_used_mb, latest_storage_used.disk_used_mb) AS disk_used_mb,
                      COALESCE(latest.disk_limit_mb, latest_storage_limit.disk_limit_mb) AS disk_limit_mb,
                      latest.site_cpu_limit_cores,
                      latest.site_ram_limit_mb,
                      latest.db_cpu_limit_cores,
                      latest.db_ram_limit_mb,
                      latest.ssh_cpu_limit_cores,
                      latest.ssh_ram_limit_mb
               FROM LATERAL (
                   SELECT sampled_at, cpu_percent, ram_used_mb, ram_limit_mb, disk_used_mb, disk_limit_mb,
                          site_cpu_limit_cores, site_ram_limit_mb, db_cpu_limit_cores,
                          db_ram_limit_mb, ssh_cpu_limit_cores, ssh_ram_limit_mb
                   FROM infrastructure_resource_samples
                   WHERE entity_kind = 'deployment' AND deployment_uuid = $1
                   ORDER BY sampled_at DESC
                   LIMIT 1
               ) latest
               LEFT JOIN LATERAL (
                   SELECT disk_used_mb
                   FROM infrastructure_resource_samples
                   WHERE entity_kind = 'deployment'
                     AND deployment_uuid = $1
                     AND disk_used_mb IS NOT NULL
                   ORDER BY sampled_at DESC
                   LIMIT 1
               ) latest_storage_used ON TRUE
               LEFT JOIN LATERAL (
                   SELECT disk_limit_mb
                   FROM infrastructure_resource_samples
                   WHERE entity_kind = 'deployment'
                     AND deployment_uuid = $1
                     AND disk_limit_mb IS NOT NULL
                   ORDER BY sampled_at DESC
                   LIMIT 1
               ) latest_storage_limit ON TRUE",
        )
        .bind(deployment_uuid)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)
    }

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

    pub async fn hosting_allocation_for_plan(
        pool: &PgPool,
        plan: &str,
    ) -> Result<HostingResourceAllocation, AppError> {
        sqlx::query_as::<_, HostingResourceAllocation>(
            r"SELECT (wp_cpu_millicores + db_cpu_millicores + ssh_cpu_millicores)::bigint AS cpu_millicores,
                      (wp_memory_mb + db_memory_mb + ssh_memory_mb)::bigint AS ram_mb,
                      storage_limit_mb::bigint AS disk_mb
               FROM hosting_plan_configs
               WHERE plan_name = $1",
        )
        .bind(plan)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn ensure_capacity_row(
        pool: &PgPool,
        server: &InfrastructureServerRecord,
    ) -> Result<(), AppError> {
        let Some(server_uuid) = server.coolify_server_uuid.as_deref() else {
            return Ok(());
        };
        sqlx::query(
            r"INSERT INTO server_capacity (server_uuid, server_id)
               VALUES ($1, $2)
               ON CONFLICT (server_uuid)
               DO UPDATE SET server_id = EXCLUDED.server_id, updated_at = NOW()",
        )
        .bind(server_uuid)
        .bind(server.id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn update_capacity_specs(
        pool: &PgPool,
        server_uuid: &str,
        cpu_cores: Option<f64>,
        ram_mb: Option<i32>,
        disk_mb: Option<i32>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"UPDATE server_capacity
               SET cpu_cores = COALESCE($2, cpu_cores),
                   ram_mb = COALESCE($3, ram_mb),
                   disk_mb = COALESCE($4, disk_mb),
                   updated_at = NOW()
               WHERE server_uuid = $1",
        )
        .bind(server_uuid)
        .bind(cpu_cores)
        .bind(ram_mb)
        .bind(disk_mb)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn capacity_for_update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        server_uuid: &str,
    ) -> Result<Option<ServerCapacityRow>, AppError> {
        sqlx::query_as::<_, ServerCapacityRow>(
            r"SELECT server_uuid,
                      cpu_cores::float8 AS cpu_cores,
                      ram_mb,
                      disk_mb,
                      cpu_allocated::float8 AS cpu_allocated,
                      ram_allocated,
                      disk_allocated
               FROM server_capacity
               WHERE server_uuid = $1
               FOR UPDATE",
        )
        .bind(server_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(AppError::from)
    }

    pub async fn allocate_capacity(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        server_uuid: &str,
        allocation: &HostingResourceAllocation,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"UPDATE server_capacity
               SET cpu_allocated = cpu_allocated + $2,
                   ram_allocated = ram_allocated + $3,
                   disk_allocated = disk_allocated + $4,
                   updated_at = NOW()
               WHERE server_uuid = $1",
        )
        .bind(server_uuid)
        .bind(millicores_to_cores(allocation.cpu_millicores))
        .bind(i32::try_from(allocation.ram_mb).unwrap_or(i32::MAX))
        .bind(i32::try_from(allocation.disk_mb).unwrap_or(i32::MAX))
        .execute(&mut **tx)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn reserve_capacity_if_known(
        pool: &PgPool,
        server_uuid: &str,
        allocation: &HostingResourceAllocation,
    ) -> Result<bool, AppError> {
        let cpu_requested = millicores_to_cores(allocation.cpu_millicores);
        let ram_requested = i32::try_from(allocation.ram_mb).unwrap_or(i32::MAX);
        let disk_requested = i32::try_from(allocation.disk_mb).unwrap_or(i32::MAX);

        let updated = sqlx::query_scalar::<_, String>(
            r"UPDATE server_capacity
               SET cpu_allocated = cpu_allocated + $2,
                   ram_allocated = ram_allocated + $3,
                   disk_allocated = disk_allocated + $4,
                   updated_at = NOW()
               WHERE server_uuid = $1
                 AND (cpu_cores <= 0 OR cpu_allocated + $2 <= cpu_cores * 0.90)
                 AND (ram_mb <= 0 OR ram_allocated + $3 <= ram_mb * 90 / 100)
                 AND (disk_mb <= 0 OR disk_allocated + $4 <= disk_mb * 90 / 100)
               RETURNING server_uuid",
        )
        .bind(server_uuid)
        .bind(cpu_requested)
        .bind(ram_requested)
        .bind(disk_requested)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?;

        if updated.is_some() {
            return Ok(true);
        }

        let row_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM server_capacity WHERE server_uuid = $1)",
        )
        .bind(server_uuid)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;
        Ok(!row_exists)
    }

    pub async fn release_capacity(
        pool: &PgPool,
        server_uuid: &str,
        allocation: &HostingResourceAllocation,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"UPDATE server_capacity
               SET cpu_allocated = GREATEST(0, cpu_allocated - $2),
                   ram_allocated = GREATEST(0, ram_allocated - $3),
                   disk_allocated = GREATEST(0, disk_allocated - $4),
                   updated_at = NOW()
               WHERE server_uuid = $1",
        )
        .bind(server_uuid)
        .bind(millicores_to_cores(allocation.cpu_millicores))
        .bind(i32::try_from(allocation.ram_mb).unwrap_or(i32::MAX))
        .bind(i32::try_from(allocation.disk_mb).unwrap_or(i32::MAX))
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

    pub async fn purge_old_samples(pool: &PgPool) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM infrastructure_resource_samples WHERE sampled_at < NOW() - INTERVAL '7 days'",
        )
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }
}
