/* [259A-4b] Runner del binario lightweight-manager (extraído de hosting_runtime.rs).
 * Métodos `pub(crate)`: los consumen los módulos hermanos de lifecycle y
 * backups vía `Self::` (mismo tipo, misma crate). Sin cambios de API pública. */

use serde::de::DeserializeOwned;

use crate::errors::AppError;

use super::hosting_runtime::{
    HostingRuntimeDeploymentSummary, HostingRuntimeKind, HostingRuntimeService,
    LightweightManagerConfig,
};
use super::hosting_runtime_lightweight_types::LightweightManagerInventoryReport;

impl HostingRuntimeService {
    pub(crate) async fn list_lightweight_deployments(
    ) -> Result<Vec<HostingRuntimeDeploymentSummary>, AppError> {
        let report: LightweightManagerInventoryReport = Self::run_lightweight_manager_json(
            "listar despliegues",
            "inventory-light",
            &["--json".to_string()],
        )
        .await?;

        Ok(report
            .sites
            .into_iter()
            .map(|site| HostingRuntimeDeploymentSummary {
                runtime_kind: HostingRuntimeKind::Lightweight,
                deployment_id: site.deployment_id,
                name: site.name,
                status: site.status,
                fqdn: site.fqdn,
                target_id: Some(report.target_ip.clone()),
                target_name: Some(report.target.clone()),
                project_id: None,
                environment_name: None,
            })
            .collect())
    }

    pub(crate) async fn run_lightweight_site_action(
        operation: &str,
        deployment_id: &str,
        action: &str,
        delete_volumes: bool,
    ) -> Result<(), AppError> {
        let mut args = vec![
            "--site".to_string(),
            deployment_id.to_string(),
            "--action".to_string(),
            action.to_string(),
            "--json".to_string(),
        ];
        if delete_volumes {
            args.push("--delete-volumes".to_string());
        }
        let _: serde_json::Value =
            Self::run_lightweight_manager_json(operation, "light-site", &args).await?;
        Ok(())
    }

    pub(crate) async fn run_lightweight_manager_json<T: DeserializeOwned>(
        operation: &str,
        command_name: &str,
        extra_args: &[String],
    ) -> Result<T, AppError> {
        let stdout = Self::run_lightweight_manager(operation, command_name, extra_args).await?;
        serde_json::from_slice(&stdout).map_err(|error| {
            AppError::ServiceUnavailable(format!(
                "El runtime de hosting 'lightweight' devolvió JSON inválido para {operation}: {error}"
            ))
        })
    }

    pub(crate) async fn run_lightweight_manager(
        operation: &str,
        command_name: &str,
        extra_args: &[String],
    ) -> Result<Vec<u8>, AppError> {
        let config = LightweightManagerConfig::from_env()?;
        let mut command = tokio::process::Command::new(&config.bin_path);
        command.arg("--log-level").arg("error");
        if let Some(config_path) = &config.config_path {
            command.arg("--config").arg(config_path);
        }
        command
            .arg(command_name)
            .arg("--target")
            .arg(&config.target);
        for arg in extra_args {
            command.arg(arg);
        }

        let output = command.output().await.map_err(|error| {
            AppError::ServiceUnavailable(format!(
                "No se pudo ejecutar {} del runtime de hosting 'lightweight' con '{}': {}",
                operation, config.bin_path, error
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(AppError::ServiceUnavailable(format!(
                "El runtime de hosting 'lightweight' falló al {}: {}{}{}",
                operation,
                stderr.trim(),
                if stderr.trim().is_empty() || stdout.trim().is_empty() {
                    ""
                } else {
                    " | stdout: "
                },
                if stdout.trim().is_empty() {
                    ""
                } else {
                    stdout.trim()
                }
            )));
        }

        Ok(output.stdout)
    }
}
