/* [07AA-14] Tests compose B Coolify: yaml/recursos/backup/red (split desde coolify.rs).
 * Fichero solo-test: `#![cfg(test)]` interno = marcador honesto para el gate
 * (rustTestScope `esArchivoSoloTest`); el padre ya lo incluye bajo `#[cfg(test)]`. */
#![cfg(test)]

use super::compose::*;
use super::compose_ssh::*;
use super::tests_units::test_plan_config;
use serde_yaml::Value as YamlValue;

#[test]
fn compose_normal_yaml_parses() {
    let config = test_plan_config("normal-basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    serde_yaml::from_str::<YamlValue>(&compose).expect("normal compose debe parsear");
}

#[test]
fn compose_for_service_yaml_parses_with_bootstrap_routes() {
    let wp_config = test_plan_config("basico");
    let wp_compose = build_hosting_compose_for_service(
        "hosting-de17015b",
        "173.249.50.44",
        None,
        "user",
        "pass",
        12132,
        &wp_config,
    );
    let normal_config = test_plan_config("normal-basico");
    let normal_compose = build_hosting_compose_for_service(
        "hosting-de17015b",
        "173.249.50.44",
        None,
        "user",
        "pass",
        12132,
        &normal_config,
    );

    serde_yaml::from_str::<YamlValue>(&wp_compose)
        .expect("wordpress compose con bootstrap route debe parsear");
    serde_yaml::from_str::<YamlValue>(&normal_compose)
        .expect("normal compose con bootstrap route debe parsear");
}

#[test]
fn compose_wp_disables_file_editing() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(compose.contains("DISALLOW_FILE_EDIT"));
}

/* --- Compose: recursos dinámicos --- */

#[test]
fn compose_applies_dynamic_resource_limits() {
    let mut config = test_plan_config("pro");
    config.wp_cpu_millicores = 1000;
    config.wp_memory_mb = 512;
    config.db_cpu_millicores = 750;
    config.db_memory_mb = 256;
    config.ssh_cpu_millicores = 500;
    config.ssh_memory_mb = 128;

    let compose = build_hosting_compose("user", "pass", 10001, &config);

    /* WordPress: 1.0 CPU, 512M */
    assert!(compose.contains("cpus: '1.00'"), "WP CPU debe ser 1.00");
    assert!(compose.contains("memory: 512M"), "WP memory debe ser 512M");
    /* DB: 0.75 CPU, 256M */
    assert!(compose.contains("cpus: '0.75'"), "DB CPU debe ser 0.75");
    assert!(compose.contains("memory: 256M"), "DB memory debe ser 256M");
    /* SSH: 0.50 CPU, 128M */
    assert!(compose.contains("cpus: '0.50'"), "SSH CPU debe ser 0.50");
    assert!(compose.contains("memory: 128M"), "SSH memory debe ser 128M");
}

/* --- Compose: backup sidecar --- */

#[test]
fn compose_wordpress_plans_include_backup_sidecar() {
    let config = test_plan_config("ecommerce");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(
        compose.contains("backup:"),
        "Ecommerce debe incluir sidecar backup"
    );
    assert!(
        compose.contains("backup-data:"),
        "Ecommerce debe incluir volumen backup"
    );
    assert!(compose.contains("mysqldump"), "Backup debe usar mysqldump");
}

#[test]
fn compose_basico_uses_weekly_backup_sidecar() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(compose.contains("backup:"), "Basico debe incluir backup");
    assert!(
        compose.contains("weekly_wp_"),
        "Basico guarda copia semanal"
    );
    assert!(
        !compose.contains("daily_wp_"),
        "Basico no crea copia diaria"
    );
}

#[test]
fn compose_pro_uses_daily_backup_sidecar() {
    let config = test_plan_config("pro");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(compose.contains("backup:"), "Pro debe incluir backup");
    assert!(compose.contains("daily_wp_"), "Pro guarda copia diaria");
    assert!(compose.contains("weekly_wp_"), "Pro conserva copia semanal");
}

#[test]
fn compose_normal_plans_include_static_backup_sidecar() {
    let config = test_plan_config("normal-pro");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(
        compose.contains("backup:"),
        "Hosting normal debe incluir backup"
    );
    assert!(compose.contains("site-data:/site-html:ro"));
    assert!(compose.contains("daily_site_"));
    assert!(!compose.contains("mysqldump"));
}

#[test]
fn compose_backup_retention_policy() {
    let config = test_plan_config("ecommerce");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    /* 3 días de backups diarios */
    assert!(
        compose.contains("-mtime +3 -delete"),
        "Retención diaria: 3 días"
    );
    /* 14 días (2 semanas) de backups semanales */
    assert!(
        compose.contains("-mtime +14 -delete"),
        "Retención semanal: 14 días"
    );
}

#[test]
fn compose_backup_runs_on_sundays_only() {
    let config = test_plan_config("ecommerce");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    /* DOW = 7 es domingo */
    assert!(compose.contains("DOW = 7"), "Backup semanal solo domingos");
}

/* --- Compose: isolation de red --- */

#[test]
fn compose_wordpress_connects_to_frontend_and_backend() {
    let compose = build_compose_wp_db("0.50", "256M", "0.50", "512M", &[], None);

    /* WordPress debe estar en frontend (sirve tráfico) y backend (habla con DB) */
    assert!(compose.contains("frontend_net"));
    assert!(compose.contains("backend_net"));
}

#[test]
fn compose_mariadb_only_on_backend() {
    let compose = build_compose_wp_db("0.50", "256M", "0.50", "512M", &[], None);

    /* MariaDB aparece como servicio "  mariadb:\n". Su sección debe contener
     * backend_net pero NO frontend_net. Extraer desde la definición del servicio. */
    let mariadb_start = compose.find("  mariadb:\n").expect("mariadb service");
    let mariadb_section = &compose[mariadb_start..];
    assert!(mariadb_section.contains("backend_net"));
    assert!(
        !mariadb_section.contains("frontend_net"),
        "MariaDB no debe estar en frontend_net"
    );
}

#[test]
fn compose_ssh_on_ssh_net_and_backend() {
    let compose = build_compose_ssh("user", "pass", 10001, "0.25", "128M");

    assert!(compose.contains("ssh_net"));
    assert!(compose.contains("backend_net"));
    /* SSH no debe estar en frontend_net */
    assert!(
        !compose.contains("frontend_net"),
        "SSH no debe estar en frontend_net"
    );
}

#[test]
fn normal_compose_ssh_avoids_backend_network() {
    let config = test_plan_config("normal-pro");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(compose.contains("ssh_net"));
    assert!(!compose.contains("backend_net"));
    assert!(!compose.contains("WP-CLI"));
}
