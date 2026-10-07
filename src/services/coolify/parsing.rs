/* [07AA-14] Helpers de parsing Coolify (split sin cambios de lógica desde coolify.rs). */

use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::Value;

use super::config::CoolifyConfig;
use super::types::CoolifyServiceSummary;

fn read_nested_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }

    current
        .as_str()
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(ToOwned::to_owned)
}

fn read_first_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| read_nested_string(value, path))
}

pub(super) fn extract_service_fqdn(value: &Value) -> Option<String> {
    read_first_string(
        value,
        &[&["fqdn"], &["destination", "fqdn"], &["service", "fqdn"]],
    )
    .or_else(|| {
        value
            .get("domains")
            .and_then(Value::as_array)
            .and_then(|domains| {
                domains.iter().find_map(|domain| {
                    domain
                        .as_str()
                        .map(str::trim)
                        .filter(|candidate| !candidate.is_empty())
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            domain
                                .get("domain")
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .filter(|candidate| !candidate.is_empty())
                                .map(ToOwned::to_owned)
                        })
                        .or_else(|| {
                            domain
                                .get("fqdn")
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .filter(|candidate| !candidate.is_empty())
                                .map(ToOwned::to_owned)
                        })
                })
            })
    })
}

pub(super) fn parse_service_summary(value: &Value) -> Option<CoolifyServiceSummary> {
    let uuid = read_first_string(value, &[&["uuid"]])?;
    let name = read_first_string(value, &[&["name"]])?;

    Some(CoolifyServiceSummary {
        uuid,
        name,
        status: read_first_string(value, &[&["status"], &["deployment_status"]])
            .unwrap_or_else(|| "unknown".to_string()),
        fqdn: extract_service_fqdn(value),
        server_uuid: read_first_string(
            value,
            &[
                &["server_uuid"],
                &["server", "uuid"],
                &["server", "server_uuid"],
            ],
        ),
        server_name: read_first_string(value, &[&["server_name"], &["server", "name"]]),
        project_uuid: read_first_string(value, &[&["project_uuid"], &["project", "uuid"]]),
        environment_name: read_first_string(
            value,
            &[&["environment_name"], &["environment", "name"]],
        ),
    })
}

pub(super) fn service_matches_target(
    service: &CoolifyServiceSummary,
    config: &CoolifyConfig,
) -> bool {
    let matches_server = service
        .server_uuid
        .as_deref()
        .is_none_or(|server_uuid| server_uuid == config.server_uuid);
    let matches_project = service
        .project_uuid
        .as_deref()
        .is_none_or(|project_uuid| project_uuid == config.project_uuid);

    matches_server && matches_project
}

pub(super) fn is_normal_hosting_plan(plan_name: &str) -> bool {
    plan_name.starts_with("normal-")
}

pub(super) fn millicores_to_cpu(millicores: i32) -> String {
    format!("{:.2}", f64::from(millicores) / 1000.0)
}

pub(super) fn clean_route_host(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

pub(super) fn route_host_slug(host: &str) -> String {
    let slug = clean_route_host(host)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();

    slug.split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub(super) fn hosting_bootstrap_host(
    plan_name: &str,
    service_name: &str,
    server_ip: &str,
) -> String {
    let service_prefix = if is_normal_hosting_plan(plan_name) {
        "site"
    } else {
        "wordpress"
    };

    format!("{service_prefix}-{service_name}.{server_ip}.sslip.io")
}

pub(super) fn hosting_route_hosts(
    plan_name: &str,
    service_name: &str,
    server_ip: &str,
    custom_domain: Option<&str>,
) -> Vec<String> {
    let mut hosts = vec![hosting_bootstrap_host(plan_name, service_name, server_ip)];

    if let Some(domain) = custom_domain {
        let cleaned = clean_route_host(domain);
        if !cleaned.is_empty() && !hosts.iter().any(|host| host == &cleaned) {
            hosts.push(cleaned);
        }
    }

    hosts
}

/* [165A-13] Las labels Traefik deben alinearse con las claves del servicio.
 * El compose estático usa 8/12 espacios y el de WordPress 4/6; si se reutiliza
 * una indentación fija, YAML queda inválido y Coolify deja el stack en `exited`. */
pub(super) fn build_traefik_labels(
    route_hosts: &[String],
    property_indent: usize,
    item_indent: usize,
    ingress_network: Option<&str>,
) -> String {
    if route_hosts.is_empty() {
        return String::new();
    }

    let property_prefix = " ".repeat(property_indent);
    let item_prefix = " ".repeat(item_indent);

    let mut lines = vec![
        format!("{property_prefix}labels:"),
        format!("{item_prefix}- 'traefik.enable=true'"),
    ];

    if let Some(network) = ingress_network
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{item_prefix}- 'traefik.docker.network={network}'"));
    }

    for host in route_hosts {
        let cleaned_host = clean_route_host(host);
        if cleaned_host.is_empty() {
            continue;
        }

        let slug = route_host_slug(&cleaned_host);
        if slug.is_empty() {
            continue;
        }

        lines.push(format!(
            "{item_prefix}- 'traefik.http.services.{slug}-svc.loadbalancer.server.port=80'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-http.rule=Host(`{cleaned_host}`)'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-http.entrypoints=http'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-http.service={slug}-svc'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-https.rule=Host(`{cleaned_host}`)'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-https.entrypoints=https'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-https.tls=true'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-https.tls.certresolver=letsencrypt'"
        ));
        lines.push(format!(
            "{item_prefix}- 'traefik.http.routers.{slug}-https.service={slug}-svc'"
        ));
    }

    format!("{}\n", lines.join("\n"))
}

pub(super) fn generate_sftp_credentials() -> (String, String) {
    let sftp_user: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect::<String>()
        .to_lowercase();
    let sftp_password: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();

    (format!("wp_{sftp_user}"), sftp_password)
}
