/* sentinel-disable-file sqlx-query-sin-macro: payment handler usa runtime query para
 * UPDATE de estado tras webhook Stripe (tipo dinámico por contexto). */
/* [01AA-4-f3n] Checkout e historial de pagos (extraido de payments.rs). */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CheckoutIntentResponse, CreateCheckoutIntentRequest, InitiatePaymentRequest,
    PaymentIntentResponse, PaymentResponse, UserRole,
};
use crate::repositories::{OrderRepository, UserRepository};
use crate::services::{is_checkout_bypass_email, PaymentService};
use crate::AppState;

/// Iniciar pago de una orden (crea `PaymentIntent` en Stripe)
#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/pay",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    request_body = InitiatePaymentRequest,
    responses(
        (status = 200, description = "PaymentIntent creado", body = PaymentIntentResponse),
        (status = 400, description = "Datos inválidos", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "payments"
)]
pub async fn initiate_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<InitiatePaymentRequest>,
) -> Result<Json<PaymentIntentResponse>, AppError> {
    auth.require_role(&[UserRole::Client, UserRole::Admin])?;

    /* [064A-65] Admin puede pagar cualquier orden (testing/soporte).
     * Para clientes, el servicio verifica ownership por client_id. */
    let caller_id = if auth.role == UserRole::Admin {
        None
    } else {
        Some(auth.user_id)
    };

    /* [064A-59] Obtener email del usuario para pre-llenar en Stripe (receipt_email).
     * Así no se le pide email de nuevo en el checkout.
     * [074A-24] Log si falla la consulta en vez de silenciar con .ok(). */
    let user_email = match UserRepository::find_by_id(&state.pool, auth.user_id).await {
        Ok(Some(u)) => Some(u.email),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!("No se pudo obtener email del usuario para receipt_email: {e}");
            None
        }
    };

    if user_email.as_deref().is_some_and(is_checkout_bypass_email) {
        let result = PaymentService::initiate_bypassed_payment(
            &state.pool,
            order_id,
            caller_id,
            req.phase_number,
        )
        .await?;
        return Ok(Json(result));
    }

    let stripe_key = state
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Stripe no está configurado".into()))?;

    let result = PaymentService::initiate_payment(
        &state.pool,
        &state.http_client,
        stripe_key,
        order_id,
        caller_id,
        req.phase_number,
        user_email.as_deref(),
    )
    .await?;

    Ok(Json(result))
}

/* [166A-2] Crear PaymentIntent de checkout directo (sin orden previa).
 * [20CA-1] Ya NO requiere autenticación: acepta email en el body para crear
 * el PaymentIntent con metadata. El usuario se crea DESPUÉS del pago exitoso
 * en el webhook (handle_checkout_payment_succeeded). Esto evita cuentas
 * huérfanas cuando el usuario abandona el checkout de Stripe. */
pub async fn create_checkout_intent(
    State(state): State<AppState>,
    Json(req): Json<CreateCheckoutIntentRequest>,
) -> Result<Json<CheckoutIntentResponse>, AppError> {
    let stripe_key = state
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Stripe no está configurado".into()))?;

    /* [20CA-1] El email es obligatorio para checkout sin auth */
    let email = req
        .email
        .as_deref()
        .filter(|e| !e.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("Email es requerido para checkout".into()))?;

    /* Validar formato básico de email */
    if !email.contains('@') || email.len() < 5 {
        return Err(AppError::BadRequest("Email inválido".into()));
    }

    let result = PaymentService::create_checkout_intent(
        &state.pool,
        &state.http_client,
        stripe_key,
        email,
        &req.service_slug,
        &req.plan_slug,
        req.payment_mode,
    )
    .await?;

    Ok(Json(result))
}

/// Historial de pagos de una orden
#[utoipa::path(
    get,
    path = "/api/orders/{order_id}/payments",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    responses(
        (status = 200, description = "Historial de pagos", body = Vec<PaymentResponse>),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "Orden no encontrada", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "payments"
)]
pub async fn list_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Vec<PaymentResponse>>, AppError> {
    /* Verificar acceso: dueño, asignado o admin */
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    /* [074A-50] Admin real siempre tiene acceso, effective_role solo afecta UI */
    if auth.role != UserRole::Admin {
        match auth.effective_role {
            UserRole::Admin => {}
            UserRole::Client => {
                if order.client_id != auth.user_id {
                    return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
                }
            }
            UserRole::Employee => {
                if order.assigned_employee_id != Some(auth.user_id) {
                    return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
                }
            }
        }
    }

    let payments = PaymentService::list_payments(&state.pool, order_id).await?;
    Ok(Json(payments))
}
