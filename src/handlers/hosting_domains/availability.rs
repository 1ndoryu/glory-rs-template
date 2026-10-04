/* [01AA-4-f3p] Disponibilidad y checkout de dominios (extraido de hosting_domains.rs). */

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use validator::Validate;

use super::helpers::{contabo, domain_price_quote, normalize_domain, resolve_public_base_url};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateDomainCheckoutRequest, DomainCheckoutResponse, DomainOrder, DomainPriceQuote, UserRole,
};
use crate::repositories::{CreateDomainOrderParams, DomainOrderRepository, UserRepository};
use crate::services::{DomainCheckoutParams, DomainStripeService};
use crate::AppState;

pub(super) async fn check_domain_availability(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(domain): Path<String>,
) -> Result<Json<DomainPriceQuote>, AppError> {
    let normalized = normalize_domain(&domain)?;
    let svc = contabo(&state)?;
    let available = svc
        .check_domain_availability(&normalized)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(domain_price_quote(normalized, available)?))
}

pub(super) async fn list_domain_orders(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<DomainOrder>>, AppError> {
    let orders = if auth.effective_role == UserRole::Admin {
        DomainOrderRepository::list_all(&state.pool).await?
    } else {
        DomainOrderRepository::list_by_user(&state.pool, auth.user_id).await?
    };
    Ok(Json(orders))
}

pub(super) async fn create_domain_checkout(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(body): Json<CreateDomainCheckoutRequest>,
) -> Result<(StatusCode, Json<DomainCheckoutResponse>), AppError> {
    body.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let normalized = normalize_domain(&body.domain)?;
    let svc = contabo(&state)?;
    let available = svc
        .check_domain_availability(&normalized)
        .await
        .map_err(AppError::Internal)?;
    if !available {
        return Err(AppError::Validation("El dominio no está disponible".into()));
    }

    let quote = domain_price_quote(normalized.clone(), true)?;
    let user = UserRepository::find_by_id(&state.pool, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("Usuario no encontrado".into()))?;
    let order = DomainOrderRepository::create(
        &state.pool,
        CreateDomainOrderParams {
            user_id: auth.user_id,
            domain: &quote.domain,
            tld: &quote.tld,
            base_cost_cents: quote.base_cost_cents,
            price_cents: quote.price_cents,
        },
    )
    .await?;

    let stripe_key = state
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::ServiceUnavailable("Stripe no configurado".into()))?;
    let base_url = resolve_public_base_url(&headers);
    let success_url = format!("{base_url}/panel?seccion=dominios&domain=success");
    let cancel_url = format!("{base_url}/panel?seccion=dominios&domain=cancelled");
    let (session_id, checkout_url) =
        DomainStripeService::create_checkout_session(&DomainCheckoutParams {
            http_client: &state.http_client,
            stripe_key,
            order_id: order.id,
            domain: &order.domain,
            amount_cents: order.price_cents,
            customer_email: &user.email,
            success_url: &success_url,
            cancel_url: &cancel_url,
        })
        .await?;
    let order =
        DomainOrderRepository::set_checkout_session(&state.pool, order.id, &session_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(DomainCheckoutResponse {
            order,
            checkout_url,
        }),
    ))
}
