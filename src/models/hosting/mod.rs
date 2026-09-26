/* [054A-2] Modelos de hosting: suscripciones y eventos.
 * hosting_subscriptions registra planes contratados por clientes.
 * hosting_events guarda eventos del ciclo de vida: provisioned, backup, health_fail, etc.
 * [225A-1] El contrato publico queda dividido por responsabilidad para evitar que
 * un modelo vuelva a crecer hasta requerir sentinel-disable-file limite-lineas. */

mod entities;
mod requests;
mod responses;
mod sanitization;
mod validation;

pub use entities::{
    normalize_cpu_scaling_policy, HostingEmailAlias, HostingEmailMailbox, HostingEvent,
    HostingPlanConfig, HostingSubscription, PublicHostingPlan, CPU_SCALING_POLICY_BASELINE_BURST,
    CPU_SCALING_POLICY_CONTENTION_THROTTLE,
};
pub use requests::{
    AssignHostingRequest, CreateEmailAliasRequest, CreateHostingRequest, SelfSubscribeRequest,
    UpdateHostingRequest, UpdateHostingStatusRequest, UpdatePlanConfigRequest,
};
pub use responses::{
    CoolifyDeploymentResponse, EmailAliasResponse, EmailMailboxResponse, HostingEmailInfoResponse,
    HostingStatsResponse, HostingSubscriptionResponse, SelfSubscribeResponse,
};
pub use sanitization::{sanitize_hosting_event, sanitize_hosting_event_details};

#[cfg(test)]
mod tests {
    use super::validation::DOMAIN_REGEX;
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use validator::Validate;

    fn self_subscribe_request(plan: &str, domain: Option<&str>) -> SelfSubscribeRequest {
        SelfSubscribeRequest {
            plan: plan.to_string(),
            domain: domain.map(ToOwned::to_owned),
            billing_cycle_months: None,
            wp_admin_username: None,
            wp_admin_password: None,
            wp_language: None,
            sftp_user: None,
            sftp_password: None,
        }
    }

    #[test]
    fn domain_regex_valid_domains() {
        let valid = [
            "example.com",
            "sub.example.com",
            "my-site.example.co.uk",
            "a.b.c.d.example.com",
            "x.com",
            "example123.com",
        ];
        for d in valid {
            assert!(DOMAIN_REGEX.is_match(d), "Should be valid: {d}");
        }
    }

    #[test]
    fn domain_regex_rejects_invalid() {
        let invalid = [
            "",
            " ",
            "http://example.com",
            "https://example.com",
            "example.com/path",
            "-example.com",
            "example-.com",
            "example..com",
            "'; DROP TABLE users; --",
            "javascript:alert(1)",
            "../../etc/passwd",
            "example .com",
        ];
        for d in invalid {
            assert!(!DOMAIN_REGEX.is_match(d), "Should be invalid: {d}");
        }
    }

    #[test]
    fn self_subscribe_request_valid_domain() {
        let req = self_subscribe_request("basico", Some("example.com"));
        assert!(req.validate().is_ok());
    }

    #[test]
    fn self_subscribe_request_invalid_domain_rejected() {
        let req = self_subscribe_request("basico", Some("'; DROP TABLE hosting_subscriptions; --"));
        assert!(req.validate().is_err());
    }

    #[test]
    fn self_subscribe_request_no_domain_ok() {
        let req = self_subscribe_request("basico", None);
        assert!(req.validate().is_ok());
    }

    #[test]
    fn self_subscribe_request_empty_plan_rejected() {
        let req = self_subscribe_request("", None);
        assert!(req.validate().is_err());
    }

    #[test]
    fn create_request_valid_all_fields() {
        let req = CreateHostingRequest {
            client_name: "Juan García".to_string(),
            client_email: "juan@example.com".to_string(),
            plan: "pro".to_string(),
            domain: Some("nakomi.studio".to_string()),
            coolify_site_name: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn create_request_empty_name_rejected() {
        let req = CreateHostingRequest {
            client_name: String::new(),
            client_email: "test@test.com".to_string(),
            plan: "basico".to_string(),
            domain: None,
            coolify_site_name: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn create_request_invalid_email_rejected() {
        let req = CreateHostingRequest {
            client_name: "Test".to_string(),
            client_email: "not-an-email".to_string(),
            plan: "basico".to_string(),
            domain: None,
            coolify_site_name: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn create_request_sql_injection_in_domain_rejected() {
        let req = CreateHostingRequest {
            client_name: "Test".to_string(),
            client_email: "test@test.com".to_string(),
            plan: "basico".to_string(),
            domain: Some("' OR 1=1; --".to_string()),
            coolify_site_name: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn create_request_xss_in_domain_rejected() {
        let req = CreateHostingRequest {
            client_name: "Test".to_string(),
            client_email: "test@test.com".to_string(),
            plan: "basico".to_string(),
            domain: Some("<script>alert(1)</script>.com".to_string()),
            coolify_site_name: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn update_request_valid() {
        let req = UpdateHostingRequest {
            plan: "ecommerce".to_string(),
            domain: Some("new-domain.com".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn update_request_empty_plan_rejected() {
        let req = UpdateHostingRequest {
            plan: String::new(),
            domain: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn status_request_valid() {
        let req = UpdateHostingStatusRequest {
            status: "active".to_string(),
            reason: Some("Payment received".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn status_request_empty_status_rejected() {
        let req = UpdateHostingStatusRequest {
            status: String::new(),
            reason: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn domain_regex_max_label_length_63() {
        let long_label = "a".repeat(63);
        let domain = format!("{long_label}.com");
        assert!(
            DOMAIN_REGEX.is_match(&domain),
            "63 char label debería ser válido"
        );

        let too_long = "a".repeat(64);
        let domain = format!("{too_long}.com");
        assert!(
            !DOMAIN_REGEX.is_match(&domain),
            "64 char label debería ser inválido"
        );
    }

    #[test]
    fn domain_regex_accepts_numeric_tld() {
        assert!(DOMAIN_REGEX.is_match("12345.678"));
    }

    #[test]
    fn domain_regex_rejects_trailing_dot() {
        assert!(!DOMAIN_REGEX.is_match("example.com."));
    }

    #[test]
    fn subscription_response_preserves_all_fields() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let sub = HostingSubscription {
            id,
            user_id: Some(user_id),
            client_name: "Test Client".to_string(),
            client_email: "test@test.com".to_string(),
            plan: "pro".to_string(),
            domain: Some("test.com".to_string()),
            domain_verification_status: "active".to_string(),
            domain_verification_token: Some("nakomi-verification=abc123".to_string()),
            domain_verified_at: Some(now),
            runtime_kind: "coolify".to_string(),
            deployment_id: Some("deploy-123".to_string()),
            coolify_site_name: Some("hosting-abc123".to_string()),
            status: "active".to_string(),
            stripe_subscription_id: Some("sub_123".to_string()),
            monthly_price_cents: 1000,
            storage_limit_mb: 20480,
            server_uuid: Some("uuid-123".to_string()),
            server_ip: Some("1.2.3.4".to_string()),
            sftp_user: Some("user".to_string()),
            sftp_password: Some("pass".to_string()),
            sftp_port: Some(10001),
            created_at: now,
            updated_at: now,
        };

        let resp = HostingSubscriptionResponse::from(sub);
        assert_eq!(resp.id, id);
        assert_eq!(resp.user_id, Some(user_id));
        assert_eq!(resp.plan, "pro");
        assert_eq!(resp.domain.as_deref(), Some("test.com"));
        assert_eq!(resp.domain_verification_status, "active");
        assert_eq!(
            resp.domain_verification_token.as_deref(),
            Some("nakomi-verification=abc123")
        );
        assert_eq!(resp.domain_verified_at, Some(now));
        assert_eq!(resp.runtime_kind.as_deref(), Some("coolify"));
        assert_eq!(resp.deployment_id.as_deref(), Some("deploy-123"));
        assert_eq!(resp.status, "active");
        assert_eq!(resp.monthly_price_cents, 1000);
        assert_eq!(resp.storage_limit_mb, 20480);
        assert_eq!(resp.server_ip.as_deref(), Some("1.2.3.4"));
        assert_eq!(resp.sftp_user.as_deref(), Some("user"));
        assert_eq!(resp.sftp_port, Some(10001));
    }

    #[test]
    fn subscription_runtime_defaults_empty_to_coolify() {
        let sub = HostingSubscription {
            id: Uuid::new_v4(),
            user_id: None,
            client_name: "Test Client".to_string(),
            client_email: "test@test.com".to_string(),
            plan: "pro".to_string(),
            domain: None,
            domain_verification_status: "none".to_string(),
            domain_verification_token: None,
            domain_verified_at: None,
            runtime_kind: String::new(),
            deployment_id: None,
            coolify_site_name: None,
            status: "pending".to_string(),
            stripe_subscription_id: None,
            monthly_price_cents: 1000,
            storage_limit_mb: 20480,
            server_uuid: None,
            server_ip: None,
            sftp_user: None,
            sftp_password: None,
            sftp_port: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(sub.is_coolify_runtime());
    }

    /* [265A-11] Tests de validacion de CreateEmailAliasRequest */
    #[test]
    fn email_alias_request_valid() {
        let req = CreateEmailAliasRequest {
            alias: "info".to_string(),
            domain: "nakomi.studio".to_string(),
            destination: "cliente@gmail.com".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn email_alias_request_empty_alias_rejected() {
        let req = CreateEmailAliasRequest {
            alias: String::new(),
            domain: "nakomi.studio".to_string(),
            destination: "cliente@gmail.com".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn email_alias_request_invalid_destination_rejected() {
        let req = CreateEmailAliasRequest {
            alias: "ventas".to_string(),
            domain: "nakomi.studio".to_string(),
            destination: "not-an-email".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn email_alias_request_long_alias_rejected() {
        let req = CreateEmailAliasRequest {
            alias: "a".repeat(101),
            domain: "nakomi.studio".to_string(),
            destination: "cliente@gmail.com".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn email_alias_request_long_domain_rejected() {
        let req = CreateEmailAliasRequest {
            alias: "soporte".to_string(),
            domain: "a".repeat(254),
            destination: "cliente@gmail.com".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn email_alias_request_long_destination_rejected() {
        let req = CreateEmailAliasRequest {
            alias: "soporte".to_string(),
            domain: "nakomi.studio".to_string(),
            destination: "a".repeat(255),
        };
        assert!(req.validate().is_err());
    }

    /* [265A-11] Tests de conversion EmailAliasResponse */
    #[test]
    fn email_alias_response_from_entity_preserves_all_fields() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let sub_id = Uuid::new_v4();
        let entity = HostingEmailAlias {
            id,
            subscription_id: sub_id,
            alias: "info".to_string(),
            domain: "nakomi.studio".to_string(),
            destination: "cliente@gmail.com".to_string(),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        };

        let resp = EmailAliasResponse::from(entity);
        assert_eq!(resp.id, id);
        assert_eq!(resp.alias, "info");
        assert_eq!(resp.domain, "nakomi.studio");
        assert_eq!(resp.destination, "cliente@gmail.com");
        assert_eq!(resp.full_email, "info@nakomi.studio");
        assert_eq!(resp.status, "active");
        assert_eq!(resp.created_at, now);
    }
}
