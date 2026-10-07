/* [07AA-14] Auto-suscripción VPS extraída de `vps.rs`: crea la suscripción en
 * pending_payment, computa extras de región/storage y abre checkout Stripe
 * (o bypass de pruebas por email). */

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{SelfSubscribeVpsRequest, SelfSubscribeVpsResponse, VpsSubscription};
use crate::repositories::{CreateVpsSubscriptionParams, UserRepository, VpsRepository};
use crate::services::{
    is_checkout_bypass_email, vps_stripe_fee_cents, VpsCheckoutParams, VpsStripeService,
};
use crate::AppState;

fn resolve_public_base_url(headers: &HeaderMap) -> String {
    if let Some(origin) = headers.get("origin").and_then(|value| value.to_str().ok()) {
        let trimmed = origin.trim_end_matches('/');
        if !trimmed.is_empty() && !trimmed.contains("localhost") {
            return trimmed.to_string();
        }
    }

    if let Some(referer) = headers.get("referer").and_then(|value| value.to_str().ok()) {
        if let Some(scheme_index) = referer.find("://") {
            let after_scheme = &referer[scheme_index + 3..];
            let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
            let base_url = &referer[..scheme_index + 3 + host_end];
            if !base_url.contains("localhost") {
                return base_url.to_string();
            }
        }
    }

    if let Ok(env_url) = std::env::var("GLORY_PUBLIC_URL") {
        return env_url.trim_end_matches('/').to_string();
    }

    "http://localhost:5173".to_string()
}

fn build_client_notes(
    storage: Option<&str>,
    region: Option<&str>,
    os: Option<&str>,
    server_password: Option<&str>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = storage {
        if !s.trim().is_empty() {
            parts.push(format!("Storage: {s}"));
        }
    }
    if let Some(r) = region {
        if !r.trim().is_empty() {
            parts.push(format!("Región: {r}"));
        }
    }
    if let Some(o) = os {
        if !o.trim().is_empty() {
            parts.push(format!("OS: {o}"));
        }
    }
    if let Some(p) = server_password {
        if !p.trim().is_empty() {
            parts.push(format!("Password: {p}"));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn selected_extra_cents(
    options: &serde_json::Value,
    selected: Option<&str>,
) -> Result<i32, AppError> {
    let amount = selected
        .and_then(|key| options.get(key))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    i32::try_from(amount)
        .map_err(|_| AppError::Validation("Extra de precio VPS fuera de rango".into()))
}

async fn complete_test_bypass_checkout(
    pool: &PgPool,
    subscription: VpsSubscription,
    tier: &str,
    user_id: Uuid,
    base_url: &str,
) -> Result<SelfSubscribeVpsResponse, AppError> {
    VpsRepository::update_status(pool, subscription.id, "pending_approval").await?;
    let _ = VpsRepository::add_event(
        pool,
        subscription.id,
        "test_checkout_bypassed",
        Some(serde_json::json!({
            "tier": tier,
            "source": "self-service",
            "by": user_id.to_string(),
        })),
    )
    .await;

    let updated = VpsRepository::find_by_id(pool, subscription.id)
        .await?
        .unwrap_or(subscription);
    let checkout_url = format!(
        "{base_url}/panel?vps=test-bypass&subscription_id={}",
        updated.id
    );

    Ok(SelfSubscribeVpsResponse {
        subscription: updated.into(),
        checkout_url,
    })
}

#[utoipa::path(
    post,
    path = "/api/vps/subscribe",
    request_body = SelfSubscribeVpsRequest,
    responses(
        (status = 201, description = "Suscripción VPS creada + URL de checkout", body = SelfSubscribeVpsResponse),
        (status = 400, description = "Datos inválidos"),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn subscribe_self(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(req): Json<SelfSubscribeVpsRequest>,
) -> Result<(StatusCode, Json<SelfSubscribeVpsResponse>), AppError> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;

    let plan_config = VpsRepository::get_plan_config(&state.pool, &req.tier)
        .await?
        .filter(|config| config.is_active)
        .ok_or_else(|| AppError::Validation(format!("Tier inválido: {}", req.tier)))?;

    let user = UserRepository::find_by_id(&state.pool, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Usuario no encontrado".into()))?;

    let client_name = user.display_name.unwrap_or_else(|| user.email.clone());
    let client_email = user.email;

    let notes_str = build_client_notes(
        req.storage_preference.as_deref(),
        req.region_preference.as_deref(),
        req.os_preference.as_deref(),
        req.server_password.as_deref(),
    );

    /* [205A-1] Computar precio total real incluyendo extra de región y storage elegidos */
    let region_extra = selected_extra_cents(
        &plan_config.region_extra_cents,
        req.region_preference.as_deref(),
    )?;
    let storage_extra = selected_extra_cents(
        &plan_config.storage_extra_cents,
        req.storage_preference.as_deref(),
    )?;
    let total_monthly_cents = plan_config.monthly_price_cents + region_extra + storage_extra;

    let subscription = VpsRepository::create(
        &state.pool,
        CreateVpsSubscriptionParams {
            user_id: Some(auth.user_id),
            client_name: &client_name,
            client_email: &client_email,
            tier_name: &req.tier,
            requested_hostname: req.hostname.as_deref(),
            client_notes: notes_str.as_deref(),
            monthly_price_cents: total_monthly_cents,
        },
    )
    .await?;

    let _ = VpsRepository::add_event(
        &state.pool,
        subscription.id,
        "created",
        Some(serde_json::json!({
            "tier": req.tier,
            "source": "self-service",
            "by": auth.user_id.to_string(),
        })),
    )
    .await;

    let base_url = resolve_public_base_url(&headers);
    if is_checkout_bypass_email(&client_email) {
        let response = complete_test_bypass_checkout(
            &state.pool,
            subscription,
            &req.tier,
            auth.user_id,
            &base_url,
        )
        .await?;
        return Ok((StatusCode::CREATED, Json(response)));
    }

    let stripe_key = state
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::ServiceUnavailable("Stripe no configurado".into()))?;

    let success_url = format!("{base_url}/panel?vps=success&session_id={{CHECKOUT_SESSION_ID}}");
    let cancel_url = format!("{base_url}/panel?vps=cancelled");

    let total_first_payment = subscription.monthly_price_cents + plan_config.setup_fee_cents;
    let stripe_fee = vps_stripe_fee_cents(total_first_payment);

    let checkout_url = VpsStripeService::create_checkout_session(&VpsCheckoutParams {
        http_client: &state.http_client,
        stripe_key,
        subscription_id: subscription.id,
        tier_name: &subscription.tier_name,
        amount_cents: subscription.monthly_price_cents,
        setup_fee_cents: plan_config.setup_fee_cents,
        processing_fee_cents: stripe_fee,
        customer_email: &subscription.client_email,
        success_url: &success_url,
        cancel_url: &cancel_url,
    })
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(SelfSubscribeVpsResponse {
            subscription: subscription.into(),
            checkout_url,
        }),
    ))
}
