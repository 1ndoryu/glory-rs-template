/* [01AA-4-f3p] Helpers de dominios Contabo (extraido de hosting_domains.rs).
 * [154A-1] Acceso al servicio, guard de admin, normalización y cotización. */

use axum::http::HeaderMap;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{DomainPriceQuote, UserRole};
use crate::AppState;

pub(super) fn contabo(state: &AppState) -> Result<&crate::services::ContaboService, AppError> {
    state
        .contabo_service
        .as_ref()
        .ok_or_else(|| AppError::Internal("Contabo service not configured".into()))
}

pub(super) fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.effective_role != UserRole::Admin {
        return Err(AppError::Forbidden("Admin only".into()));
    }
    Ok(())
}

pub(super) fn resolve_public_base_url(headers: &HeaderMap) -> String {
    if let Some(origin) = headers.get("origin").and_then(|value| value.to_str().ok()) {
        let trimmed = origin.trim_end_matches('/');
        if !trimmed.is_empty() && !trimmed.contains("localhost") {
            return trimmed.to_string();
        }
    }

    if let Ok(env_url) = std::env::var("GLORY_PUBLIC_URL") {
        return env_url.trim_end_matches('/').to_string();
    }

    "http://localhost:5173".to_string()
}

pub(super) fn normalize_domain(raw: &str) -> Result<String, AppError> {
    let value = raw
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_matches('/')
        .to_ascii_lowercase();
    if value.contains('/') || value.contains(' ') || !value.contains('.') {
        return Err(AppError::Validation("Dominio inválido".into()));
    }
    Ok(value)
}

pub(super) fn domain_tld(domain: &str) -> Result<String, AppError> {
    domain
        .rsplit('.')
        .next()
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| AppError::Validation("Dominio sin TLD válido".into()))
}

pub(super) fn base_domain_cost_cents(tld: &str) -> Result<i32, AppError> {
    match tld {
        "com" => Ok(1200),
        "net" | "org" => Ok(1400),
        "studio" => Ok(3500),
        "io" => Ok(4500),
        _ => Err(AppError::Validation(format!(
            "El TLD .{tld} aún no está habilitado para checkout automático"
        ))),
    }
}

pub(super) fn domain_price_quote(
    domain: String,
    available: bool,
) -> Result<DomainPriceQuote, AppError> {
    let tld = domain_tld(&domain)?;
    let base_cost_cents = base_domain_cost_cents(&tld)?;
    let price_cents = ((base_cost_cents * 105) + 99) / 100;
    Ok(DomainPriceQuote {
        domain,
        available,
        tld,
        base_cost_cents,
        price_cents,
    })
}
