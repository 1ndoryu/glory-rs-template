use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::{HostingPlanConfig, UpdatePlanConfigRequest};

use super::types::validated_cpu_scaling_policy;
use super::HostingRepository;

impl HostingRepository {
    /* [114A-3] Obtener configuración de recursos para un plan específico */
    pub async fn get_plan_config(
        pool: &PgPool,
        plan_name: &str,
    ) -> Result<Option<HostingPlanConfig>, AppError> {
        let row = sqlx::query_as!(
            HostingPlanConfig,
            r"SELECT id, plan_name, monthly_price_cents,
                      wp_cpu_millicores, wp_memory_mb, db_cpu_millicores, db_memory_mb,
                      ssh_cpu_millicores, ssh_memory_mb, storage_limit_mb, bandwidth_limit_gb,
                      cpu_scaling_policy, created_at, updated_at
               FROM hosting_plan_configs
               WHERE plan_name = $1",
            plan_name
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /* [114A-3] Listar todas las configuraciones de planes */
    pub async fn list_plan_configs(pool: &PgPool) -> Result<Vec<HostingPlanConfig>, AppError> {
        let rows = sqlx::query_as!(
            HostingPlanConfig,
            r"SELECT id, plan_name, monthly_price_cents,
                      wp_cpu_millicores, wp_memory_mb, db_cpu_millicores, db_memory_mb,
                      ssh_cpu_millicores, ssh_memory_mb, storage_limit_mb, bandwidth_limit_gb,
                      cpu_scaling_policy, created_at, updated_at
               FROM hosting_plan_configs
               ORDER BY monthly_price_cents ASC",
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [114A-3] Actualizar configuración de un plan (PATCH semántico: solo campos presentes).
     * Usa COALESCE para mantener valores existentes cuando el campo es NULL en el request. */
    pub async fn update_plan_config(
        pool: &PgPool,
        plan_name: &str,
        req: &UpdatePlanConfigRequest,
    ) -> Result<HostingPlanConfig, AppError> {
        let cpu_scaling_policy = validated_cpu_scaling_policy(req.cpu_scaling_policy.as_deref())?;
        let row = sqlx::query_as!(
            HostingPlanConfig,
            r"UPDATE hosting_plan_configs SET
                  monthly_price_cents = COALESCE($1, monthly_price_cents),
                  wp_cpu_millicores = COALESCE($2, wp_cpu_millicores),
                  wp_memory_mb = COALESCE($3, wp_memory_mb),
                  db_cpu_millicores = COALESCE($4, db_cpu_millicores),
                  db_memory_mb = COALESCE($5, db_memory_mb),
                  ssh_cpu_millicores = COALESCE($6, ssh_cpu_millicores),
                  ssh_memory_mb = COALESCE($7, ssh_memory_mb),
                  storage_limit_mb = COALESCE($8, storage_limit_mb),
                  bandwidth_limit_gb = COALESCE($9, bandwidth_limit_gb),
                  cpu_scaling_policy = COALESCE($10, cpu_scaling_policy),
                  updated_at = NOW()
               WHERE plan_name = $11
               RETURNING id, plan_name, monthly_price_cents,
                         wp_cpu_millicores, wp_memory_mb, db_cpu_millicores, db_memory_mb,
                         ssh_cpu_millicores, ssh_memory_mb, storage_limit_mb, bandwidth_limit_gb,
                         cpu_scaling_policy, created_at, updated_at",
            req.monthly_price_cents,
            req.wp_cpu_millicores,
            req.wp_memory_mb,
            req.db_cpu_millicores,
            req.db_memory_mb,
            req.ssh_cpu_millicores,
            req.ssh_memory_mb,
            req.storage_limit_mb,
            req.bandwidth_limit_gb,
            cpu_scaling_policy,
            plan_name
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                AppError::NotFound(format!("Plan '{plan_name}' no encontrado"))
            }
            other => AppError::from(other),
        })?;
        Ok(row)
    }
}
