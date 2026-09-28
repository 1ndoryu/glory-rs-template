/* [259A-4] Servidores, muestras y capacidad (extraído de infrastructure.rs).
 * Tablas: infrastructure_servers, infrastructure_resource_samples,
 * server_capacity, hosting_plan_configs. Sin estado: métodos asociados sobre
 * `InfrastructureRepository` (impl inherente cross-módulo, misma crate). */

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{InfrastructureServerMetricsResponse, ResourceMetricPoint};
use crate::services::coolify::CoolifyConfig;

use super::infrastructure::{millicores_to_cores, InfrastructureRepository};

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
            let existing = sqlx::query_as!(
                InfrastructureServerRecord,
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
                &input.config.server_uuid,
                input.label,
                &input.config.server_ip,
                &input.config.base_url,
                &input.config.project_uuid,
                input.secret_ref,
                input.ssh_secret_ref
            )
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)?;

            if let Some(record) = existing {
                return Ok(record);
            }
        }

        sqlx::query_as!(
            InfrastructureServerRecord,
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
            input.label,
            &input.config.server_ip,
            &input.config.base_url,
            &input.config.server_uuid,
            &input.config.project_uuid,
            input.secret_ref,
            input.ssh_secret_ref
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn list_servers_with_metrics(
        pool: &PgPool,
    ) -> Result<Vec<InfrastructureServerMetricsResponse>, AppError> {
        sqlx::query_as!(
            InfrastructureServerMetricsResponse,
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
        sqlx::query!(
            r"INSERT INTO infrastructure_resource_samples
                    (entity_kind, server_id, deployment_uuid, sampled_at, cpu_percent,
                     ram_used_mb, ram_limit_mb, disk_used_mb, disk_limit_mb,
                     site_cpu_limit_cores, site_ram_limit_mb, db_cpu_limit_cores,
                     db_ram_limit_mb, ssh_cpu_limit_cores, ssh_ram_limit_mb)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            input.entity_kind,
            input.server_id,
            input.deployment_uuid,
            input.sampled_at,
            input.cpu_percent,
            input.ram_used_mb,
            input.ram_limit_mb,
            input.disk_used_mb,
            input.disk_limit_mb,
            input.site_cpu_limit_cores,
            input.site_ram_limit_mb,
            input.db_cpu_limit_cores,
            input.db_ram_limit_mb,
            input.ssh_cpu_limit_cores,
            input.ssh_ram_limit_mb
        )
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
        /* $2::text exige String en el macro: to_string() del clamp i64.
         * El SQL generado es idéntico al anterior (mismo intervalo). */
        sqlx::query_as!(
            ResourceMetricPoint,
            r#"SELECT to_timestamp(floor(extract(epoch from sampled_at) / 600) * 600) AS "sampled_at!",
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
               ORDER BY 1 ASC"#,
            deployment_uuid,
            range_hours.clamp(1, 168).to_string()
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn latest_deployment_sample(
        pool: &PgPool,
        deployment_uuid: &str,
    ) -> Result<Option<ResourceMetricPoint>, AppError> {
        sqlx::query_as!(
            ResourceMetricPoint,
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
            deployment_uuid
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn hosting_allocation_for_plan(
        pool: &PgPool,
        plan: &str,
    ) -> Result<HostingResourceAllocation, AppError> {
        sqlx::query_as!(
            HostingResourceAllocation,
            r#"SELECT (wp_cpu_millicores + db_cpu_millicores + ssh_cpu_millicores)::bigint AS "cpu_millicores!",
                      (wp_memory_mb + db_memory_mb + ssh_memory_mb)::bigint AS "ram_mb!",
                      storage_limit_mb::bigint AS "disk_mb!"
               FROM hosting_plan_configs
               WHERE plan_name = $1"#,
            plan
        )
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
        sqlx::query!(
            r"INSERT INTO server_capacity (server_uuid, server_id)
               VALUES ($1, $2)
               ON CONFLICT (server_uuid)
               DO UPDATE SET server_id = EXCLUDED.server_id, updated_at = NOW()",
            server_uuid,
            server.id
        )
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
        /* $2::float8: cpu_cores es numeric en BD; el cast fija el parámetro a
         * float8 para el macro. En runtime el comportamiento es idéntico
         * (sqlx ya enviaba f64 como FLOAT8 y Postgres resolvía a numeric). */
        sqlx::query!(
            r"UPDATE server_capacity
               SET cpu_cores = COALESCE($2::float8, cpu_cores),
                   ram_mb = COALESCE($3, ram_mb),
                   disk_mb = COALESCE($4, disk_mb),
                   updated_at = NOW()
               WHERE server_uuid = $1",
            server_uuid,
            cpu_cores,
            ram_mb,
            disk_mb
        )
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn capacity_for_update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        server_uuid: &str,
    ) -> Result<Option<ServerCapacityRow>, AppError> {
        sqlx::query_as!(
            ServerCapacityRow,
            r#"SELECT server_uuid,
                      cpu_cores::float8 AS "cpu_cores!",
                      ram_mb,
                      disk_mb,
                      cpu_allocated::float8 AS "cpu_allocated!",
                      ram_allocated,
                      disk_allocated
               FROM server_capacity
               WHERE server_uuid = $1
               FOR UPDATE"#,
            server_uuid
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(AppError::from)
    }

    pub async fn allocate_capacity(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        server_uuid: &str,
        allocation: &HostingResourceAllocation,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r"UPDATE server_capacity
               SET cpu_allocated = cpu_allocated + $2::float8,
                   ram_allocated = ram_allocated + $3,
                   disk_allocated = disk_allocated + $4,
                   updated_at = NOW()
               WHERE server_uuid = $1",
            server_uuid,
            millicores_to_cores(allocation.cpu_millicores),
            i32::try_from(allocation.ram_mb).unwrap_or(i32::MAX),
            i32::try_from(allocation.disk_mb).unwrap_or(i32::MAX)
        )
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

        let updated: Option<String> = sqlx::query_scalar!(
            r"UPDATE server_capacity
               SET cpu_allocated = cpu_allocated + $2::float8,
                   ram_allocated = ram_allocated + $3,
                   disk_allocated = disk_allocated + $4,
                   updated_at = NOW()
               WHERE server_uuid = $1
                 AND (cpu_cores <= 0 OR cpu_allocated + $2::float8 <= cpu_cores * 0.90)
                 AND (ram_mb <= 0 OR ram_allocated + $3 <= ram_mb * 90 / 100)
                 AND (disk_mb <= 0 OR disk_allocated + $4 <= disk_mb * 90 / 100)
               RETURNING server_uuid",
            server_uuid,
            cpu_requested,
            ram_requested,
            disk_requested
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?;

        if updated.is_some() {
            return Ok(true);
        }

        let row_exists: bool = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM server_capacity WHERE server_uuid = $1) AS \"exists!\"",
            server_uuid
        )
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
        sqlx::query!(
            r"UPDATE server_capacity
               SET cpu_allocated = GREATEST(0, cpu_allocated - $2::float8),
                   ram_allocated = GREATEST(0, ram_allocated - $3),
                   disk_allocated = GREATEST(0, disk_allocated - $4),
                   updated_at = NOW()
               WHERE server_uuid = $1",
            server_uuid,
            millicores_to_cores(allocation.cpu_millicores),
            i32::try_from(allocation.ram_mb).unwrap_or(i32::MAX),
            i32::try_from(allocation.disk_mb).unwrap_or(i32::MAX)
        )
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn purge_old_samples(pool: &PgPool) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM infrastructure_resource_samples WHERE sampled_at < NOW() - INTERVAL '7 days'",
        )
        .execute(pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }
}
