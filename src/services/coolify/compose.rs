/* [07AA-14] Compose builders Coolify WP/static (split sin cambios desde coolify.rs). */

use crate::models::HostingPlanConfig;

use super::compose_backup::{
    backup_cadence_for_plan, build_compose_static_backup, build_compose_wordpress_backup,
};
use super::compose_ssh::{build_compose_ssh, build_compose_static_ssh};
use super::parsing::{
    build_traefik_labels, hosting_route_hosts, is_normal_hosting_plan, millicores_to_cpu,
};

/* [114A-3] Límites dinámicos por plan. WP reserva 25% de su límite, DB reserva 25% también.
 * [265A-6] SMTP: si las env vars GLORY_SMTP_HOST/SMTP_HOST están disponibles,
 * inyecta las vars WORDPRESS_SMTP_* en el container + PHP phpmailer_init en
 * WORDPRESS_CONFIG_EXTRA para que WordPress envíe correos vía SMTP (Brevo). */
pub(super) fn build_compose_wp_db(
    wp_cpu: &str,
    wp_mem: &str,
    db_cpu: &str,
    db_mem: &str,
    route_hosts: &[String],
    ingress_network: Option<&str>,
) -> String {
    let traefik_labels = build_traefik_labels(route_hosts, 4, 6, ingress_network);

    // SMTP injection: leer desde env vars del host (compat GLORY_SMTP_* y SMTP_*)
    let smtp_host = std::env::var("SMTP_HOST")
        .or_else(|_| std::env::var("GLORY_SMTP_HOST"))
        .unwrap_or_default();
    let smtp_has = !smtp_host.is_empty();
    let smtp_env = if smtp_has {
        let smtp_port = std::env::var("SMTP_PORT")
            .or_else(|_| std::env::var("GLORY_SMTP_PORT"))
            .unwrap_or_else(|_| "587".to_string());
        let smtp_user = std::env::var("SMTP_USER")
            .or_else(|_| std::env::var("GLORY_SMTP_USER"))
            .unwrap_or_default();
        let smtp_pass = std::env::var("SMTP_PASS")
            .or_else(|_| std::env::var("GLORY_SMTP_PASSWORD"))
            .unwrap_or_default();
        let smtp_from = std::env::var("SMTP_FROM").unwrap_or_else(|_| smtp_user.clone());
        let smtp_from_name =
            std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Nakomi Studio".to_string());
        format!(
            "      - WORDPRESS_SMTP_HOST={smtp_host}\n      - WORDPRESS_SMTP_PORT={smtp_port}\n      - WORDPRESS_SMTP_USER={smtp_user}\n      - WORDPRESS_SMTP_PASSWORD={smtp_pass}\n      - WORDPRESS_SMTP_FROM={smtp_from}\n      - WORDPRESS_SMTP_FROM_NAME={smtp_from_name}\n",
        )
    } else {
        String::new()
    };
    let config_extra = if smtp_has {
        // Minified PHP que lee WORDPRESS_SMTP_* env vars y configura phpmailer vía hook
        "define('DISALLOW_FILE_EDIT', true);$wpg=getenv('WORDPRESS_SMTP_HOST');if($wpg){add_action('phpmailer_init',function($m){$m->isSMTP();$m->Host=$wpg;$m->Port=getenv('WORDPRESS_SMTP_PORT')?:587;$m->SMTPAuth=true;$m->Username=getenv('WORDPRESS_SMTP_USER');$m->Password=getenv('WORDPRESS_SMTP_PASSWORD');$m->From=getenv('WORDPRESS_SMTP_FROM')?:'noreply';$m->FromName=getenv('WORDPRESS_SMTP_FROM_NAME')?:'Nakomi';$m->SMTPSecure='tls';});}"
    } else {
        "define('DISALLOW_FILE_EDIT', true);"
    };

    format!(
        "  wordpress:\n    image: 'wordpress:6.7-php8.3-apache'\n    environment:\n      - SERVICE_FQDN_WORDPRESS=\n      - WORDPRESS_DB_HOST=mariadb\n      - WORDPRESS_DB_USER=wordpress\n      - WORDPRESS_DB_PASSWORD=SERVICE_PASSWORD_DB\n      - WORDPRESS_DB_NAME=wordpress\n      - WORDPRESS_CONFIG_EXTRA={config_extra}\n{smtp_env}    volumes:\n      - 'wordpress-data:/var/www/html'\n    depends_on:\n      - mariadb\n    restart: unless-stopped\n    networks:\n      - frontend_net\n      - backend_net\n{traefik_labels}    cap_drop:\n      - ALL\n    cap_add:\n      - CHOWN\n      - SETUID\n      - SETGID\n      - DAC_OVERRIDE\n      - NET_BIND_SERVICE\n    security_opt:\n      - no-new-privileges:true\n    deploy:\n      resources:\n        limits:\n          cpus: '{wp_cpu}'\n          memory: {wp_mem}\n        reservations:\n          memory: 128M\n  mariadb:\n    image: 'mariadb:11.4'\n    environment:\n      - MYSQL_ROOT_PASSWORD=SERVICE_PASSWORD_ROOT\n      - MYSQL_DATABASE=wordpress\n      - MYSQL_USER=wordpress\n      - MYSQL_PASSWORD=SERVICE_PASSWORD_DB\n    volumes:\n      - 'mariadb-data:/var/lib/mysql'\n    restart: unless-stopped\n    networks:\n      - backend_net\n    cap_drop:\n      - ALL\n    cap_add:\n      - CHOWN\n      - SETUID\n      - SETGID\n      - DAC_OVERRIDE\n    security_opt:\n      - no-new-privileges:true\n    deploy:\n      resources:\n        limits:\n          cpus: '{db_cpu}'\n          memory: {db_mem}\n        reservations:\n          memory: 128M\n"
    )
}

/* [155A-13] Servicio web para hosting normal: Nginx sirve el volumen editable por SFTP. */
pub(super) fn build_compose_static_site(
    site_cpu: &str,
    site_mem: &str,
    route_hosts: &[String],
    ingress_network: Option<&str>,
) -> String {
    let traefik_labels = build_traefik_labels(route_hosts, 8, 12, ingress_network);
    format!(
        r#"  site:
        image: 'nginx:1.27-alpine'
        environment:
            - SERVICE_FQDN_SITE=
        command:
            - sh
            - -c
            - 'mkdir -p /usr/share/nginx/html; test -f /usr/share/nginx/html/index.html || printf "%s\n" "<h1>Hosting activo</h1>" > /usr/share/nginx/html/index.html; chown -R nginx:nginx /usr/share/nginx/html; nginx -g "daemon off;"'
        volumes:
            - 'site-data:/usr/share/nginx/html'
        restart: unless-stopped
        networks:
            - frontend_net
{traefik_labels}        cap_drop:
            - ALL
        cap_add:
            - CHOWN
            - SETUID
            - SETGID
            - DAC_OVERRIDE
            - NET_BIND_SERVICE
        security_opt:
            - no-new-privileges:true
        deploy:
            resources:
                limits:
                    cpus: '{site_cpu}'
                    memory: {site_mem}
                reservations:
                    memory: 64M
"#
    )
}

/* [164A-6][155A-13] Genera el compose YAML para hosting administrado.
 * Los slugs `normal-*` usan Nginx + SFTP; los slugs legacy usan WordPress + MariaDB + SSH/SFTP.
 * [164A-16] Hardening: imágenes pineadas, network isolation, cap_drop ALL,
 * no-new-privileges y WP file editing deshabilitado.
 * [165A-12] NO incluir `pids_limit` en el compose: Coolify inyecta
 * `deploy.resources.limits.pids` y Compose rechaza ambas definiciones juntas.
 * [165A-12] El sidecar SSH debe usar una tag existente de linuxserver
 * (`version-9.9_p2-r0`): la tag legacy `9.9_p2-r0-ls190` ya no resuelve en lscr.io.
 * [174A-17][215A-6] Incluye sidecar de backup automático: básico semanal,
 * Pro/Avanzado diario con copia semanal.
 * [114A-3] Límites de CPU/RAM dinámicos desde HostingPlanConfig (admin-configurable). */
#[cfg(test)]
pub(super) fn build_hosting_compose(
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    config: &HostingPlanConfig,
) -> String {
    build_hosting_compose_with_routes(&[], sftp_user, sftp_password, sftp_port, config, None)
}

pub(super) fn build_hosting_compose_for_service(
    service_name: &str,
    server_ip: &str,
    custom_domain: Option<&str>,
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    config: &HostingPlanConfig,
) -> String {
    build_hosting_compose_for_service_with_ingress_network(
        service_name,
        server_ip,
        custom_domain,
        sftp_user,
        sftp_password,
        sftp_port,
        config,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_hosting_compose_for_service_with_ingress_network(
    service_name: &str,
    server_ip: &str,
    custom_domain: Option<&str>,
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    config: &HostingPlanConfig,
    ingress_network: Option<&str>,
) -> String {
    let route_hosts =
        hosting_route_hosts(&config.plan_name, service_name, server_ip, custom_domain);
    build_hosting_compose_with_routes(
        &route_hosts,
        sftp_user,
        sftp_password,
        sftp_port,
        config,
        ingress_network,
    )
}

pub(super) fn build_hosting_compose_with_routes(
    route_hosts: &[String],
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    config: &HostingPlanConfig,
    ingress_network: Option<&str>,
) -> String {
    if is_normal_hosting_plan(&config.plan_name) {
        return build_normal_hosting_compose(
            route_hosts,
            sftp_user,
            sftp_password,
            sftp_port,
            config,
            ingress_network,
        );
    }

    build_wordpress_hosting_compose(
        route_hosts,
        sftp_user,
        sftp_password,
        sftp_port,
        config,
        ingress_network,
    )
}

pub(super) fn build_wordpress_hosting_compose(
    route_hosts: &[String],
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    config: &HostingPlanConfig,
    ingress_network: Option<&str>,
) -> String {
    let wp_cpu = millicores_to_cpu(config.wp_cpu_millicores);
    let wp_mem = format!("{}M", config.wp_memory_mb);
    let db_cpu = millicores_to_cpu(config.db_cpu_millicores);
    let db_mem = format!("{}M", config.db_memory_mb);
    let ssh_cpu = millicores_to_cpu(config.ssh_cpu_millicores);
    let ssh_mem = format!("{}M", config.ssh_memory_mb);

    let wp_db = build_compose_wp_db(
        &wp_cpu,
        &wp_mem,
        &db_cpu,
        &db_mem,
        route_hosts,
        ingress_network,
    );
    let ssh = build_compose_ssh(sftp_user, sftp_password, sftp_port, &ssh_cpu, &ssh_mem);
    let backup = build_compose_wordpress_backup(backup_cadence_for_plan(&config.plan_name));
    let backup_vol = "  backup-data:\n";
    format!(
        "services:\n{wp_db}{ssh}{backup}\nnetworks:\n  frontend_net:\n  backend_net:\n    internal: true\n  ssh_net:\nvolumes:\n  wordpress-data:\n  mariadb-data:\n{backup_vol}"
    )
}

pub(super) fn build_normal_hosting_compose(
    route_hosts: &[String],
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    config: &HostingPlanConfig,
    ingress_network: Option<&str>,
) -> String {
    let site_cpu = millicores_to_cpu(config.wp_cpu_millicores);
    let site_mem = format!("{}M", config.wp_memory_mb);
    let ssh_cpu = millicores_to_cpu(config.ssh_cpu_millicores);
    let ssh_mem = format!("{}M", config.ssh_memory_mb);
    let site = build_compose_static_site(&site_cpu, &site_mem, route_hosts, ingress_network);
    let ssh = build_compose_static_ssh(sftp_user, sftp_password, sftp_port, &ssh_cpu, &ssh_mem);
    let backup = build_compose_static_backup(backup_cadence_for_plan(&config.plan_name));

    format!(
        "services:\n{site}{ssh}{backup}\nnetworks:\n  frontend_net:\n  ssh_net:\nvolumes:\n  site-data:\n  backup-data:\n"
    )
}
