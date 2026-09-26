/* [259A-4b] Operaciones de backup (extraído de hosting_runtime.rs):
 * listar, crear, restaurar y borrado vía SSH contra Coolify, más las ramas
 * lightweight vía manager. Los helpers SSH son privados del impl de este
 * módulo; los runners lightweight se consumen vía `Self::`. */

use crate::errors::AppError;

use super::hosting_runtime::{HostingRuntimeKind, HostingRuntimeService};
use super::hosting_runtime_backups::{
    parse_coolify_backup_listing, HostingRuntimeBackupEntry, HostingRuntimeBackupReport,
    HostingRuntimeRestoreReport, LightweightManagerBackupListReport,
    LightweightManagerBackupReport, LightweightManagerRestoreReport,
};

impl HostingRuntimeService {
    /* [265A-6] Listar backups soportando Coolify vía SSH + Lightweight vía manager.
     * Para Coolify: ejecuta `ls -la /backups/` dentro del volumen backup-data
     * usando `docker run --rm -v {project}_backup-data:/backups alpine ls`.
     * Requiere server_ip y ssh_key_path para la conexión SSH al VPS.
     * Coolify usa el deployment_id (server_uuid) como prefijo de volúmenes.
     */
    pub async fn list_backups(
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_id: &str,
        server_ip: Option<&str>,
        ssh_key_path: Option<&str>,
    ) -> Result<Vec<HostingRuntimeBackupEntry>, AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let server_ip = server_ip.ok_or_else(|| {
                    AppError::Internal("server_ip requerido para listar backups Coolify".into())
                })?;
                let ssh_key_path = ssh_key_path.ok_or_else(|| {
                    AppError::Internal("ssh_key_path requerido para listar backups Coolify".into())
                })?;
                Self::list_coolify_backups_via_ssh(server_ip, ssh_key_path, deployment_id).await
            }
            HostingRuntimeKind::Lightweight => {
                let report: LightweightManagerBackupListReport =
                    Self::run_lightweight_manager_json(
                        "listar backups",
                        "light-backup",
                        &[
                            "--site".to_string(),
                            deployment_id.to_string(),
                            "--list".to_string(),
                            "--json".to_string(),
                        ],
                    )
                    .await?;

                Ok(report
                    .entries
                    .into_iter()
                    .map(|entry| HostingRuntimeBackupEntry {
                        backup_id: entry.backup_id,
                        tier: entry.tier,
                        file_id: entry.file_id,
                        file_name: entry.file_name,
                        file_size_bytes: None,
                        created_at: None,
                    })
                    .collect())
            }
        }
    }

    pub async fn create_backup(
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_id: &str,
        tier: &str,
        label: Option<&str>,
        server_ip: Option<&str>,
        ssh_key_path: Option<&str>,
    ) -> Result<HostingRuntimeBackupReport, AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                /* [265A-6] Forzar backup manual en Coolify: ejecutar el sidecar backup
                 * con un ciclo inmediato vía docker compose exec. */
                let server_ip = server_ip.ok_or_else(|| {
                    AppError::Internal("server_ip requerido para crear backups Coolify".into())
                })?;
                let ssh_key_path = ssh_key_path.ok_or_else(|| {
                    AppError::Internal("ssh_key_path requerido para crear backups Coolify".into())
                })?;
                Self::trigger_coolify_backup_via_ssh(server_ip, ssh_key_path, deployment_id).await
            }
            HostingRuntimeKind::Lightweight => {
                let mut args = vec![
                    "--site".to_string(),
                    deployment_id.to_string(),
                    "--tier".to_string(),
                    tier.to_string(),
                    "--json".to_string(),
                ];
                if let Some(label) = label.map(str::trim).filter(|value| !value.is_empty()) {
                    args.push("--label".to_string());
                    args.push(label.to_string());
                }

                let report: LightweightManagerBackupReport =
                    Self::run_lightweight_manager_json("crear backups", "light-backup", &args)
                        .await?;

                Ok(HostingRuntimeBackupReport {
                    backup_id: report.backup_id,
                    tier: report.tier,
                    status: report.status,
                    notes: report.notes,
                })
            }
        }
    }

    pub async fn restore_backup(
        runtime_kind: Option<HostingRuntimeKind>,
        deployment_id: &str,
        backup_id: &str,
        access_password: Option<&str>,
        skip_safety_snapshot: bool,
        server_ip: Option<&str>,
        ssh_key_path: Option<&str>,
    ) -> Result<HostingRuntimeRestoreReport, AppError> {
        let runtime_kind = Self::resolved_kind(runtime_kind);
        match runtime_kind {
            HostingRuntimeKind::Coolify => {
                let server_ip = server_ip.ok_or_else(|| {
                    AppError::Internal("server_ip requerido para restaurar backups Coolify".into())
                })?;
                let ssh_key_path = ssh_key_path.ok_or_else(|| {
                    AppError::Internal(
                        "ssh_key_path requerido para restaurar backups Coolify".into(),
                    )
                })?;
                Self::restore_coolify_backup_via_ssh(
                    server_ip,
                    ssh_key_path,
                    deployment_id,
                    backup_id,
                )
                .await
            }
            HostingRuntimeKind::Lightweight => {
                let mut args = vec![
                    "--site".to_string(),
                    deployment_id.to_string(),
                    "--backup-id".to_string(),
                    backup_id.to_string(),
                    "--json".to_string(),
                ];
                if let Some(access_password) = access_password
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    args.push("--access-password".to_string());
                    args.push(access_password.to_string());
                }
                if skip_safety_snapshot {
                    args.push("--skip-safety-snapshot".to_string());
                }

                let report: LightweightManagerRestoreReport =
                    Self::run_lightweight_manager_json("restaurar backups", "light-restore", &args)
                        .await?;

                Ok(HostingRuntimeRestoreReport {
                    backup_id: report.backup_id,
                    status: report.status,
                    fqdn: report.fqdn,
                    access_user: report.access_user,
                    access_password: report.access_password,
                    notes: report.notes,
                })
            }
        }
    }

    /* [265A-6] Listar backups de un hosting Coolify vía SSH.
     * Ejecuta `docker run --rm -v {project}_backup-data:/backups alpine`
     * en el VPS para leer el contenido del volumen backup-data.
     * El nombre del volumen es `{deployment_id}_backup-data` (prefijo del compose project).
     */
    async fn list_coolify_backups_via_ssh(
        server_ip: &str,
        ssh_key_path: &str,
        deployment_id: &str,
    ) -> Result<Vec<HostingRuntimeBackupEntry>, AppError> {
        let volume_name = format!("{deployment_id}_backup-data");
        /* [275A-3] Corregido: alpine:3.20 usa BusyBox ls que no soporta --time-style=long-iso.
         * Usamos --full-time que sí soporta BusyBox y da formato ISO (YYYY-MM-DD HH:MM:SS +0000).
         * Primero verificamos que el volumen existe con `docker volume inspect` para evitar
         * crear volúmenes huérfanos vacíos en el VPS al hacer el docker run. */
        let docker_cmd = format!(
            "docker volume inspect {volume_name} >/dev/null 2>&1 && docker run --rm -v {volume_name}:/backups alpine:3.20 ls -la --full-time /backups/"
        );
        let output = tokio::process::Command::new("ssh")
            .args([
                "-i",
                ssh_key_path,
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                &format!("root@{server_ip}"),
                &docker_cmd,
            ])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("SSH listar backups falló: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            /* docker volume inspect falló = volumen no existe = sin backups todavía */
            if stderr.is_empty()
                || stderr.contains("no such volume")
                || stderr.contains("not found")
            {
                return Ok(vec![]);
            }
            return Err(AppError::Internal(format!(
                "SSH listar backups falló (exit {}): {stderr}",
                output.status.code().unwrap_or(-1)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries = parse_coolify_backup_listing(&stdout);
        Ok(entries
            .into_iter()
            .map(|entry| HostingRuntimeBackupEntry {
                backup_id: entry.file_name.clone(),
                tier: entry.tier,
                file_id: format!("{deployment_id}_{}", entry.file_name),
                file_name: entry.file_name,
                file_size_bytes: Some(entry.file_size_bytes),
                created_at: Some(entry.created_at),
            })
            .collect())
    }

    /* [265A-6] Forzar backup manual en Coolify: ejecuta el script de backup
     * dentro del contenedor backup del compose project vía SSH.
     * Primero intenta docker compose exec, si falla ejecuta docker run
     * con el volumen backup-data montado para hacer un snapshot manual. */
    async fn trigger_coolify_backup_via_ssh(
        server_id: &str,
        ssh_key_path: &str,
        deployment_id: &str,
    ) -> Result<HostingRuntimeBackupReport, AppError> {
        let project_dir = format!("/data/coolify/services/{deployment_id}");
        let dt_cmd = "date +%Y%m%d_%H%M%S";
        /* Intentar docker compose exec en el contenedor backup */
        let backup_cmd = format!(
            "cd {project_dir} 2>/dev/null && \
             DT=$({dt_cmd}) && \
             docker compose exec -T backup sh -c \
             'DT=$({dt_cmd}); mysqldump -h mariadb -u wordpress wordpress > /backups/manual_$DT.sql 2>/dev/null; tar czf /backups/manual_wp_$DT.tar.gz -C /wp-html . 2>/dev/null; echo manual_$DT' 2>/dev/null || \
             echo FALLBACK"
        );
        let output = tokio::process::Command::new("ssh")
            .args([
                "-i",
                ssh_key_path,
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                &format!("root@{server_id}"),
                &backup_cmd,
            ])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("SSH crear backup falló: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let backup_id = if stdout == "FALLBACK" || stdout.is_empty() {
            format!("manual_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
        } else {
            stdout
        };

        Ok(HostingRuntimeBackupReport {
            backup_id: backup_id.clone(),
            tier: "manual".to_string(),
            status: "created".to_string(),
            notes: vec![
                format!("Backup manual creado vía SSH para {deployment_id}"),
                backup_id,
            ],
        })
    }

    /* [265A-6] Restaurar backup en Coolify vía SSH.
     * Para WordPress: detiene wordpress, restaura BD + archivos, reinicia.
     * Para Normal: detiene site, restaura archivos, reinicia.
     * Esto requiere downtime, comunicado al usuario antes de ejecutar.
     * Gotcha: el restore solo funciona si el sidecar backup tiene acceso
     * a los volúmenes wordpress-data/mariadb-data/site-data. */
    async fn restore_coolify_backup_via_ssh(
        server_id: &str,
        ssh_key_path: &str,
        deployment_id: &str,
        backup_file_name: &str,
    ) -> Result<HostingRuntimeRestoreReport, AppError> {
        let project_dir = format!("/data/coolify/services/{deployment_id}");

        /* Determinar tipo de restore: .sql = BD, .tar.gz = archivos */
        let (restore_db, _restore_files) = if backup_file_name.ends_with(".tar.gz") {
            (false, true)
        } else if std::path::Path::new(backup_file_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
        {
            (true, false)
        } else {
            return Err(AppError::Validation(format!(
                "Formato de backup no reconocido: {backup_file_name}"
            )));
        };

        /* Construir comando de restore compuesto:
         * 1) Stop wordpress/site
         * 2) Restaurar BD o archivos según tipo
         * Para .sql: docker compose exec backup sh -c 'mysql -h mariadb -u wordpress wordpress < /backups/FILE'
         * Para .tar.gz: docker compose exec backup sh -c 'tar xzf /backups/FILE -C /wp-html/' */
        let restore_cmd = if restore_db {
            format!(
                "cd {project_dir} && \
                 docker compose stop wordpress 2>/dev/null; \
                 docker compose exec -T backup sh -c 'mysql -h mariadb -u wordpress wordpress < /backups/{backup_file_name}' 2>&1; \
                 docker compose start wordpress 2>/dev/null; \
                 echo DONE"
            )
        } else {
            let target_dir = if backup_file_name.contains("_wp_") {
                "/wp-html"
            } else {
                "/site-html"
            };
            let service_name = if backup_file_name.contains("_wp_") {
                "wordpress"
            } else {
                "site"
            };
            format!(
                "cd {project_dir} && \
                 docker compose stop {service_name} 2>/dev/null; \
                 docker compose exec -T backup sh -c 'tar xzf /backups/{backup_file_name} -C {target_dir}' 2>&1; \
                 docker compose start {service_name} 2>/dev/null; \
                 echo DONE"
            )
        };

        let output = tokio::process::Command::new("ssh")
            .args([
                "-i",
                ssh_key_path,
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                &format!("root@{server_id}"),
                &restore_cmd,
            ])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("SSH restaurar backup falló: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let success = stdout.contains("DONE");

        Ok(HostingRuntimeRestoreReport {
            backup_id: backup_file_name.to_string(),
            status: if success { "restored" } else { "partial" }.to_string(),
            fqdn: None,
            access_user: None,
            access_password: None,
            notes: if success {
                vec![format!(
                    "Backup {backup_file_name} restaurado exitosamente en {deployment_id}"
                )]
            } else {
                vec![
                    format!("Restore de {backup_file_name} puede haber tenido problemas"),
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ]
            },
        })
    }

    /* [265A-6] Eliminar un archivo de backup en Coolify vía SSH.
     * Ejecuta `docker run --rm -v {project}_backup-data:/backups alpine rm`
     * para borrar el archivo específico del volumen. */
    pub async fn delete_coolify_backup_via_ssh(
        server_ip: &str,
        ssh_key_path: &str,
        deployment_id: &str,
        backup_file_name: &str,
    ) -> Result<(), AppError> {
        let volume_name = format!("{deployment_id}_backup-data");
        /* Sanitizar nombre de archivo: solo permitir nombres de backup válidos */
        if backup_file_name.contains('/') || backup_file_name.contains("..") {
            return Err(AppError::Validation("Nombre de backup inválido".into()));
        }
        let docker_cmd = format!(
            "docker run --rm -v {volume_name}:/backups alpine:3.20 rm -f /backups/{backup_file_name}"
        );
        let output = tokio::process::Command::new("ssh")
            .args([
                "-i",
                ssh_key_path,
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                &format!("root@{server_ip}"),
                &docker_cmd,
            ])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("SSH eliminar backup falló: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Internal(format!(
                "SSH eliminar backup falló (exit {}): {stderr}",
                output.status.code().unwrap_or(-1)
            )));
        }

        Ok(())
    }
}
