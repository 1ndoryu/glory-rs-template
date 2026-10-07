/* [07AA-14] Tests units Coolify: config/parsing/service_name/wordpress (split desde coolify.rs).
 * Fichero solo-test: `#![cfg(test)]` interno = marcador honesto para el gate
 * (rustTestScope `esArchivoSoloTest`); el padre ya lo incluye bajo `#[cfg(test)]`. */
#![cfg(test)]

use super::config::*;
use super::parsing::*;
use super::types::*;
use super::wordpress::*;
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::models::HostingPlanConfig;

pub(super) fn test_plan_config(plan_name: &str) -> HostingPlanConfig {
    HostingPlanConfig {
        id: Uuid::new_v4(),
        plan_name: plan_name.to_string(),
        monthly_price_cents: 500,
        wp_cpu_millicores: 500,
        wp_memory_mb: 256,
        db_cpu_millicores: 500,
        db_memory_mb: 512,
        ssh_cpu_millicores: 250,
        ssh_memory_mb: 128,
        storage_limit_mb: 5120,
        bandwidth_limit_gb: 50,
        cpu_scaling_policy: "contention_throttle".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/* --- millicores_to_cpu --- */

#[test]
fn millicores_to_cpu_standard_values() {
    assert_eq!(millicores_to_cpu(1000), "1.00");
    assert_eq!(millicores_to_cpu(500), "0.50");
    assert_eq!(millicores_to_cpu(250), "0.25");
    assert_eq!(millicores_to_cpu(1500), "1.50");
}

#[test]
fn millicores_to_cpu_edge_cases() {
    assert_eq!(millicores_to_cpu(0), "0.00");
    assert_eq!(millicores_to_cpu(1), "0.00");
    assert_eq!(millicores_to_cpu(10), "0.01");
    assert_eq!(millicores_to_cpu(100), "0.10");
    assert_eq!(millicores_to_cpu(4000), "4.00");
}

#[test]
fn parse_service_summary_reads_flat_service_payload() {
    let payload = json!({
        "uuid": "svc-123",
        "name": "hosting-demo",
        "status": "running",
        "server_uuid": "srv-vps2",
        "server_name": "vps2",
        "project_uuid": "project-1",
        "environment_name": "production",
        "domains": ["https://demo.example.com"]
    });

    let service = parse_service_summary(&payload).expect("service parse");

    assert_eq!(service.uuid, "svc-123");
    assert_eq!(service.name, "hosting-demo");
    assert_eq!(service.status, "running");
    assert_eq!(service.fqdn.as_deref(), Some("https://demo.example.com"));
    assert_eq!(service.server_uuid.as_deref(), Some("srv-vps2"));
    assert_eq!(service.server_name.as_deref(), Some("vps2"));
    assert_eq!(service.project_uuid.as_deref(), Some("project-1"));
    assert_eq!(service.environment_name.as_deref(), Some("production"));
}

#[test]
fn create_service_body_enables_instant_deploy() {
    let body = CreateServiceBody {
        name: "hosting-abcdef01",
        project_uuid: "project-1",
        environment_name: "production",
        server_uuid: "srv-1",
        docker_compose_raw: "ZHVtbXk=".to_string(),
        instant_deploy: true,
    };

    let payload = serde_json::to_value(&body).expect("serialize create body");

    assert_eq!(payload["instant_deploy"], Value::Bool(true));
    assert_eq!(
        payload["environment_name"],
        Value::String("production".to_string())
    );
}

#[test]
fn parse_service_summary_reads_nested_service_payload() {
    let payload = json!({
        "uuid": "svc-456",
        "name": "wordpress-demo",
        "deployment_status": "healthy",
        "fqdn": "https://wp.example.com",
        "server": {
            "uuid": "srv-vps2",
            "name": "vps2"
        },
        "project": {
            "uuid": "project-1"
        },
        "environment": {
            "name": "production"
        }
    });

    let service = parse_service_summary(&payload).expect("service parse");

    assert_eq!(service.status, "healthy");
    assert_eq!(service.fqdn.as_deref(), Some("https://wp.example.com"));
    assert_eq!(service.server_uuid.as_deref(), Some("srv-vps2"));
    assert_eq!(service.project_uuid.as_deref(), Some("project-1"));
    assert_eq!(service.environment_name.as_deref(), Some("production"));
}

#[test]
fn service_matches_target_rejects_other_server_or_project() {
    let config = CoolifyConfig {
        base_url: "https://coolify.example.com".to_string(),
        api_token: "token".to_string(),
        server_uuid: "srv-vps2".to_string(),
        project_uuid: "project-vps2".to_string(),
        server_ip: "10.0.0.2".to_string(),
        ssh_key_path: None,
    };

    let matching = CoolifyServiceSummary {
        uuid: "svc-1".to_string(),
        name: "hosting-a".to_string(),
        status: "running".to_string(),
        fqdn: None,
        server_uuid: Some("srv-vps2".to_string()),
        server_name: None,
        project_uuid: Some("project-vps2".to_string()),
        environment_name: Some("production".to_string()),
    };

    let other_server = CoolifyServiceSummary {
        server_uuid: Some("srv-vps1".to_string()),
        ..matching.clone()
    };

    let other_project = CoolifyServiceSummary {
        project_uuid: Some("project-vps1".to_string()),
        ..matching.clone()
    };

    assert!(service_matches_target(&matching, &config));
    assert!(!service_matches_target(&other_server, &config));
    assert!(!service_matches_target(&other_project, &config));
}

/* --- service_name_for --- */

#[test]
fn service_name_for_uses_first_8_chars() {
    let id = Uuid::parse_str("abcdef01-2345-6789-abcd-ef0123456789").unwrap();
    assert_eq!(CoolifyService::service_name_for(&id), "hosting-abcdef01");
}

#[test]
fn service_name_for_different_uuids_are_unique() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    assert_ne!(
        CoolifyService::service_name_for(&id1),
        CoolifyService::service_name_for(&id2),
    );
}

#[test]
fn service_name_for_starts_with_hosting_prefix() {
    let id = Uuid::new_v4();
    let name = CoolifyService::service_name_for(&id);
    assert!(name.starts_with("hosting-"));
    assert_eq!(name.len(), 16); /* "hosting-" (8) + 8 chars UUID */
}

#[test]
fn wordpress_site_title_prefers_client_name() {
    assert_eq!(
        build_wordpress_site_title("Mi Sitio", "hosting-abcdef01"),
        "Mi Sitio"
    );
}

#[test]
fn wordpress_site_title_falls_back_to_service_name() {
    assert_eq!(
        build_wordpress_site_title("   ", "hosting-abcdef01"),
        "Sitio hosting-abcdef01"
    );
}

#[test]
fn wordpress_install_form_detection_requires_step_two_form() {
    let html = r#"
            <form method="post" action="install.php?step=2">
                <input type="text" name="weblog_title" />
                <input type="text" name="user_name" />
                <input type="password" name="admin_password" />
            </form>
        "#;

    assert!(wordpress_install_form_is_ready(html));
    assert!(!wordpress_install_form_is_ready("<html>login</html>"));
}

/* --- CoolifyConfig::from_env --- */

#[test]
fn coolify_config_requires_all_env_vars() {
    /* Sin ninguna variable = None */
    std::env::remove_var("COOLIFY_BASE_URL");
    std::env::remove_var("COOLIFY_API_TOKEN");
    std::env::remove_var("COOLIFY_SERVER_UUID");
    std::env::remove_var("COOLIFY_PROJECT_UUID");
    std::env::remove_var("COOLIFY_SERVER_IP");
    let config = CoolifyConfig::from_env();
    assert!(config.is_none());
}
