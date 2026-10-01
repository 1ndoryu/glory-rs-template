use crate::services::coolify::HostingProvisionPreferences;

pub(crate) fn humanize_plan_name(plan: &str) -> &'static str {
    match plan.strip_prefix("normal-").unwrap_or(plan) {
        "basico" => "Basico",
        "pro" => "Pro",
        "ecommerce" => "Avanzado",
        _ => "Personalizado",
    }
}

pub(crate) fn read_checkout_config_string(
    details: &serde_json::Value,
    field: &str,
) -> Option<String> {
    let config = details.get("checkout_config").unwrap_or(details);
    config
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn provisioning_preferences_from_details(
    details: &serde_json::Value,
) -> Option<HostingProvisionPreferences> {
    let preferences = HostingProvisionPreferences {
        wp_admin_username: read_checkout_config_string(details, "wp_admin_username"),
        wp_admin_password: read_checkout_config_string(details, "wp_admin_password"),
        wp_language: read_checkout_config_string(details, "wp_language"),
        sftp_user: read_checkout_config_string(details, "sftp_user"),
        sftp_password: read_checkout_config_string(details, "sftp_password"),
    };

    if preferences.wp_admin_username.is_some()
        || preferences.wp_admin_password.is_some()
        || preferences.wp_language.is_some()
        || preferences.sftp_user.is_some()
        || preferences.sftp_password.is_some()
    {
        return Some(preferences);
    }

    None
}

pub(crate) fn hosting_product_copy(plan: &str) -> (String, String) {
    let plan_name = humanize_plan_name(plan);
    if plan.starts_with("normal-") {
        return (
            format!("Hosting {plan_name}"),
            format!("Plan {plan_name} de hosting administrado para sitios a medida y frontends"),
        );
    }

    (
        format!("Hosting WordPress {plan_name}"),
        format!("Plan {plan_name} de hosting WordPress administrado"),
    )
}

/* ============================================================
TESTS — [094A-10] Validación de lógica Stripe hosting
=========================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_plan_name_maps_known_slugs() {
        assert_eq!(humanize_plan_name("basico"), "Basico");
        assert_eq!(humanize_plan_name("pro"), "Pro");
        assert_eq!(humanize_plan_name("ecommerce"), "Avanzado");
        assert_eq!(humanize_plan_name("normal-pro"), "Pro");
        assert_eq!(humanize_plan_name("custom"), "Personalizado");
    }

    #[test]
    fn hosting_product_copy_distinguishes_normal_hosting() {
        let (normal_name, normal_desc) = hosting_product_copy("normal-basico");
        assert_eq!(normal_name, "Hosting Basico");
        assert!(normal_desc.contains("sitios a medida"));

        let (wordpress_name, wordpress_desc) = hosting_product_copy("basico");
        assert_eq!(wordpress_name, "Hosting WordPress Basico");
        assert!(wordpress_desc.contains("hosting WordPress"));
    }
}
