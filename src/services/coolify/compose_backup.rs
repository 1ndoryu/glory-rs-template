/* [07AA-14] Sidecars backup Coolify (split sin cambios desde coolify.rs). */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HostingBackupCadence {
    Weekly,
    Daily,
}

pub(super) fn backup_cadence_for_plan(plan_name: &str) -> HostingBackupCadence {
    let base_plan = plan_name.strip_prefix("normal-").unwrap_or(plan_name);
    if base_plan == "basico" {
        HostingBackupCadence::Weekly
    } else {
        HostingBackupCadence::Daily
    }
}

pub(super) fn wordpress_backup_command(cadence: HostingBackupCadence) -> &'static str {
    match cadence {
        HostingBackupCadence::Weekly => {
            "sleep 60; while true; do DT=$$(date +%Y%m%d_%H%M%S); DOW=$$(date +%u); if [ $$DOW = 7 ]; then mysqldump -h mariadb -u wordpress wordpress > /backups/weekly_$$DT.sql 2>&1; tar czf /backups/weekly_wp_$$DT.tar.gz -C /wp-html . 2>&1; find /backups -maxdepth 1 -name \"weekly_*\" -mtime +28 -delete; fi; sleep 86400; done"
        }
        HostingBackupCadence::Daily => {
            "sleep 60; while true; do DT=$$(date +%Y%m%d_%H%M%S); DOW=$$(date +%u); mysqldump -h mariadb -u wordpress wordpress > /backups/daily_$$DT.sql 2>&1; tar czf /backups/daily_wp_$$DT.tar.gz -C /wp-html . 2>&1; if [ $$DOW = 7 ]; then cp /backups/daily_$$DT.sql /backups/weekly_$$DT.sql; cp /backups/daily_wp_$$DT.tar.gz /backups/weekly_wp_$$DT.tar.gz; fi; find /backups -maxdepth 1 -name \"daily_*\" -mtime +3 -delete; find /backups -maxdepth 1 -name \"weekly_*\" -mtime +14 -delete; sleep 86400; done"
        }
    }
}

pub(super) fn static_backup_command(cadence: HostingBackupCadence) -> &'static str {
    match cadence {
        HostingBackupCadence::Weekly => {
            "sleep 60; while true; do DT=$$(date +%Y%m%d_%H%M%S); DOW=$$(date +%u); if [ $$DOW = 7 ]; then tar czf /backups/weekly_site_$$DT.tar.gz -C /site-html . 2>&1; find /backups -maxdepth 1 -name \"weekly_*\" -mtime +28 -delete; fi; sleep 86400; done"
        }
        HostingBackupCadence::Daily => {
            "sleep 60; while true; do DT=$$(date +%Y%m%d_%H%M%S); DOW=$$(date +%u); tar czf /backups/daily_site_$$DT.tar.gz -C /site-html . 2>&1; if [ $$DOW = 7 ]; then cp /backups/daily_site_$$DT.tar.gz /backups/weekly_site_$$DT.tar.gz; fi; find /backups -maxdepth 1 -name \"daily_*\" -mtime +3 -delete; find /backups -maxdepth 1 -name \"weekly_*\" -mtime +14 -delete; sleep 86400; done"
        }
    }
}

/* [174A-17][215A-6] Sidecar de backup automático para WordPress.
 * Básico crea una copia semanal; Pro/Avanzado crean copia diaria y retienen semanal dominical.
 * MYSQL_PWD es leído automáticamente por mysqldump — no se pasa en CLI. */
pub(super) fn build_compose_wordpress_backup(cadence: HostingBackupCadence) -> String {
    let command = wordpress_backup_command(cadence);
    format!(
        r"  backup:
        image: 'mariadb:11.4'
        environment:
            - MYSQL_PWD=SERVICE_PASSWORD_DB
        command:
            - sh
            - -c
            - '{command}'
        volumes:
            - 'wordpress-data:/wp-html:ro'
            - 'backup-data:/backups'
        networks:
            - backend_net
        depends_on:
            - mariadb
        restart: unless-stopped
        cap_drop:
            - ALL
        cap_add:
            - CHOWN
            - SETUID
            - SETGID
            - DAC_OVERRIDE
        security_opt:
            - no-new-privileges:true
        deploy:
            resources:
                limits:
                    cpus: '0.25'
                    memory: 256M
                reservations:
                    memory: 64M
"
    )
}

pub(super) fn build_compose_static_backup(cadence: HostingBackupCadence) -> String {
    let command = static_backup_command(cadence);
    format!(
        r"  backup:
        image: 'alpine:3.20'
        command:
            - sh
            - -c
            - '{command}'
        volumes:
            - 'site-data:/site-html:ro'
            - 'backup-data:/backups'
        networks:
            - ssh_net
        restart: unless-stopped
        cap_drop:
            - ALL
        security_opt:
            - no-new-privileges:true
        deploy:
            resources:
                limits:
                    cpus: '0.15'
                    memory: 128M
                reservations:
                    memory: 32M
"
    )
}
