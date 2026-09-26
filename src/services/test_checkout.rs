/* [155A-11] Allowlist de cuentas de prueba que no deben pagar en checkout.
 * No hardcodea secretos ni habilita compras gratis globales: produccion debe configurar
 * GLORY_TEST_CHECKOUT_EMAILS con correos comma-separated, por ejemplo test@test.com. */

#[must_use]
pub fn checkout_bypass_is_configured() -> bool {
    std::env::var("GLORY_TEST_CHECKOUT_EMAILS")
        .is_ok_and(|raw| raw.split(',').any(|entry| !entry.trim().is_empty()))
}

#[must_use]
pub fn is_checkout_bypass_email(email: &str) -> bool {
    let normalized = email.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    std::env::var("GLORY_TEST_CHECKOUT_EMAILS").is_ok_and(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_ascii_lowercase)
            .any(|entry| entry == normalized)
    })
}

#[cfg(test)]
mod tests {
    use super::is_checkout_bypass_email;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn bypass_email_is_disabled_without_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("GLORY_TEST_CHECKOUT_EMAILS");
        assert!(!super::checkout_bypass_is_configured());
        assert!(!is_checkout_bypass_email("test@test.com"));
    }

    #[test]
    fn bypass_email_matches_comma_separated_env_case_insensitive() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var(
            "GLORY_TEST_CHECKOUT_EMAILS",
            "qa@example.com, test@test.com",
        );
        assert!(super::checkout_bypass_is_configured());
        assert!(is_checkout_bypass_email(" TEST@Test.com "));
        assert!(!is_checkout_bypass_email("other@example.com"));
        std::env::remove_var("GLORY_TEST_CHECKOUT_EMAILS");
    }
}
