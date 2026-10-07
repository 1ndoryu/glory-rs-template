/* [07AA-14] Tests compose A Coolify: estructura/credenciales/seguridad (split desde coolify.rs).
 * Fichero solo-test: `#![cfg(test)]` interno = marcador honesto para el gate
 * (rustTestScope `esArchivoSoloTest`); el padre ya lo incluye bajo `#[cfg(test)]`. */
#![cfg(test)]

use super::compose::*;
use super::tests_units::test_plan_config;
use serde_yaml::Value as YamlValue;

/* --- build_hosting_compose (estructura general) --- */

#[test]
fn compose_contains_all_required_services() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("testuser", "testpass", 10001, &config);

    assert!(
        compose.contains("wordpress:"),
        "Debe contener servicio WordPress"
    );
    assert!(
        compose.contains("mariadb:"),
        "Debe contener servicio MariaDB"
    );
    assert!(compose.contains("ssh:"), "Debe contener servicio SSH");
}

#[test]
fn normal_hosting_compose_uses_nginx_without_database() {
    let config = test_plan_config("normal-basico");
    let compose = build_hosting_compose("testuser", "testpass", 10001, &config);

    assert!(
        compose.contains("site:"),
        "Debe contener servicio web normal"
    );
    assert!(compose.contains("nginx:1.27-alpine"), "Debe usar Nginx");
    assert!(compose.contains("SERVICE_FQDN_SITE"));
    assert!(!compose.contains("wordpress:"), "No debe crear WordPress");
    assert!(!compose.contains("mariadb:"), "No debe crear base de datos");
    assert!(compose.contains("site-data:/home/testuser/html"));
}

#[test]
fn compose_for_service_adds_bootstrap_route_labels() {
    let config = test_plan_config("normal-basico");
    let compose = build_hosting_compose_for_service(
        "hosting-6d746a75",
        "173.249.50.44",
        None,
        "testuser",
        "testpass",
        10001,
        &config,
    );

    assert!(compose.contains("site-hosting-6d746a75.173.249.50.44.sslip.io"));
    assert!(compose
        .contains("traefik.http.routers.site-hosting-6d746a75-173-249-50-44-sslip-io-http.rule"));

    let wp_config = test_plan_config("basico");
    let wp_compose = build_hosting_compose_for_service(
        "hosting-de17015b",
        "173.249.50.44",
        None,
        "testuser",
        "testpass",
        10001,
        &wp_config,
    );

    assert!(wp_compose.contains("wordpress-hosting-de17015b.173.249.50.44.sslip.io"));
    assert!(wp_compose.contains(
        "traefik.http.routers.wordpress-hosting-de17015b-173-249-50-44-sslip-io-http.rule"
    ));
}

#[test]
fn compose_for_service_pins_traefik_ingress_network() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose_for_service_with_ingress_network(
        "hosting-de17015b",
        "173.249.50.44",
        None,
        "testuser",
        "testpass",
        10001,
        &config,
        Some("e1m95ycgcc72zjiles3mov3c"),
    );

    assert!(compose.contains("traefik.docker.network=e1m95ycgcc72zjiles3mov3c"));
}

#[test]
fn compose_for_service_keeps_custom_domain_route() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose_for_service(
        "hosting-6d746a75",
        "173.249.50.44",
        Some("cliente.example.com"),
        "testuser",
        "testpass",
        10001,
        &config,
    );

    assert!(compose.contains("Host(`cliente.example.com`)"));
    assert!(compose.contains("traefik.http.routers.cliente-example-com-https.rule"));
}

#[test]
fn compose_contains_required_networks() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("testuser", "testpass", 10001, &config);

    assert!(compose.contains("frontend_net:"));
    assert!(compose.contains("backend_net:"));
    assert!(compose.contains("ssh_net:"));
    assert!(
        compose.contains("internal: true"),
        "backend_net debe ser internal"
    );
}

#[test]
fn compose_contains_required_volumes() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("testuser", "testpass", 10001, &config);

    assert!(compose.contains("wordpress-data:"));
    assert!(compose.contains("mariadb-data:"));
}

/* --- Compose: credenciales SSH/SFTP --- */

#[test]
fn compose_uses_provided_sftp_credentials() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("myuser", "s3cur3P@ss", 12345, &config);

    assert!(compose.contains("USER_NAME=myuser"));
    assert!(compose.contains("USER_PASSWORD=s3cur3P@ss"));
    assert!(
        compose.contains("12345:2222"),
        "Puerto SSH debe mapearse correctamente"
    );
}

#[test]
fn compose_ssh_volume_maps_to_user_home() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("webmaster", "pass", 10001, &config);

    assert!(compose.contains("wordpress-data:/home/webmaster/html"));
}

/* --- Compose: seguridad (hardening) --- */

#[test]
fn compose_has_security_restrictions() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    /* cap_drop: ALL en todos los contenedores */
    let cap_drop_count = compose.matches("cap_drop:").count();
    assert!(
        cap_drop_count >= 3,
        "wp, mariadb y ssh deben tener cap_drop: {cap_drop_count}"
    );

    /* no-new-privileges */
    let nnp_count = compose.matches("no-new-privileges:true").count();
    assert!(
        nnp_count >= 3,
        "wp, mariadb y ssh deben tener no-new-privileges: {nnp_count}"
    );

    assert!(
        !compose.contains("pids_limit:"),
        "Coolify inyecta deploy.resources.limits.pids; no debemos duplicarlo"
    );
}

#[test]
fn compose_avoids_pids_limit_conflict_with_coolify() {
    let wp_config = test_plan_config("basico");
    let wp_compose = build_hosting_compose_for_service(
        "hosting-2dad31af",
        "173.249.50.44",
        None,
        "user",
        "pass",
        12132,
        &wp_config,
    );
    let normal_config = test_plan_config("normal-basico");
    let normal_compose = build_hosting_compose_for_service(
        "hosting-2dad31af",
        "173.249.50.44",
        None,
        "user",
        "pass",
        12132,
        &normal_config,
    );

    assert!(!wp_compose.contains("pids_limit:"));
    assert!(!normal_compose.contains("pids_limit:"));
}

#[test]
fn compose_ssh_has_sudo_disabled() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(compose.contains("SUDO_ACCESS=false"));
}

#[test]
fn compose_ssh_has_sshd_hardening_script() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(compose.contains("AllowTcpForwarding no"));
    assert!(compose.contains("X11Forwarding no"));
    assert!(compose.contains("PermitTunnel no"));
    assert!(compose.contains("GatewayPorts no"));
}

#[test]
fn compose_ssh_uses_existing_linuxserver_tag() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    assert!(compose.contains("lscr.io/linuxserver/openssh-server:version-9.9_p2-r0"));
    assert!(!compose.contains("9.9_p2-r0-ls190"));
}

#[test]
fn compose_wordpress_yaml_parses() {
    let config = test_plan_config("basico");
    let compose = build_hosting_compose("user", "pass", 10001, &config);

    serde_yaml::from_str::<YamlValue>(&compose).expect("wordpress compose debe parsear");
}
