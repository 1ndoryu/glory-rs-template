/* [044A-38] Handlers de órdenes — CRUD con autenticación.
 * Cliente crea órdenes, admin/employee ven según rol, admin asigna. */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateNotification, CreateOrderRequest, OrderResponse, UpdateOrderPhaseDefinitionRequest,
    UpdateOrderProjectDescriptionRequest, UserRole, NOTIF_ORDER_ASSIGNED,
};
use crate::repositories::{ActivityLogRepository, OrderRepository};
use crate::services::OrderService;
use crate::AppState;

/* ============================================================
ÓRDENES
============================================================ */

/// Crear una nueva orden (solo clientes o admin operando como cliente)
#[allow(clippy::too_many_lines)]
#[utoipa::path(
    post,
    path = "/api/orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Orden creada", body = OrderResponse),
        (status = 400, description = "Datos inválidos", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Sin permisos", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn create_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), AppError> {
    auth.require_role(&[UserRole::Client, UserRole::Admin])?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let order = OrderService::create_order(&state.pool, auth.user_id, req).await?;

    /* [166A-1] Flujo pago-primero: la orden se crea en pending_payment.
     * La auto-asignación, chat y emails se ejecutan en el webhook de pago exitoso
     * (handle_payment_success en payments.rs). Esto garantiza que:
     * 1. El pedido NO se activa sin pago confirmado.
     * 2. El cliente puede cancelar antes de pagar.
     * 3. Stripe funciona correctamente (resolve_payment_amount acepta pending_payment). */

    /* [154A-15d] Registrar en activity_log */
    let _ = ActivityLogRepository::log(
        &state.pool,
        auth.user_id,
        "order_created",
        "order",
        order.id,
        Some(serde_json::json!({
            "service": order.service_title,
            "plan": order.plan_name,
            "payment_mode": format!("{:?}", order.payment_mode),
        })),
    )
    .await;

    Ok((StatusCode::CREATED, Json(order)))
}

/// Listar órdenes del usuario autenticado (filtrado por rol)
#[utoipa::path(
    get,
    path = "/api/orders",
    responses(
        (status = 200, description = "Lista de órdenes", body = Vec<OrderResponse>),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn list_orders(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<OrderResponse>>, AppError> {
    /* [084A-1] Con impersonación, effective_role refleja el rol real del usuario
     * impersonado. Admin sin impersonar tiene effective_role=admin → ve todo.
     * Impersonando como client → filtra por client_id. Como employee → por assigned. */
    let orders =
        OrderService::list_orders_for_user(&state.pool, auth.user_id, auth.effective_role).await?;
    Ok(Json(orders))
}

/// Detalle de una orden con sus fases
#[utoipa::path(
    get,
    path = "/api/orders/{order_id}",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    responses(
        (status = 200, description = "Detalle de la orden"),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn get_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (client_id, order, phases) = OrderService::get_order(&state.pool, order_id).await?;

    /* [074A-50] Admin real siempre puede ver cualquier orden, incluso con effective_role
     * switcheado a client/employee. El effective_role solo afecta la UI, no el acceso. */
    if auth.role != UserRole::Admin {
        match auth.effective_role {
            UserRole::Admin => { /* imposible si role != admin, pero por completitud */ }
            UserRole::Client => {
                if client_id != auth.user_id {
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

    Ok(Json(serde_json::json!({
        "order": order,
        "phases": phases,
    })))
}

#[utoipa::path(
    patch,
    path = "/api/orders/{order_id}/project-description",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    request_body = UpdateOrderProjectDescriptionRequest,
    responses(
        (status = 200, description = "Descripción del proyecto actualizada", body = OrderResponse),
        (status = 400, description = "Datos inválidos", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Sin permisos", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn update_order_project_description_handler(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<UpdateOrderProjectDescriptionRequest>,
) -> Result<Json<OrderResponse>, AppError> {
    /* [035A-15] La descripción operativa del proyecto deja de ser editable por el cliente.
     * Solo staff del panel puede mutarla desde el detalle de la orden. */
    auth.require_role(&[UserRole::Employee, UserRole::Admin])?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let order = OrderService::update_project_description(
        &state.pool,
        order_id,
        auth.user_id,
        auth.effective_role,
        req.project_description,
    )
    .await?;

    Ok(Json(order))
}

#[utoipa::path(
    patch,
    path = "/api/orders/{order_id}/phases/{phase_number}",
    params(
        ("order_id" = Uuid, Path, description = "ID de la orden"),
        ("phase_number" = i32, Path, description = "Número de fase"),
    ),
    request_body = UpdateOrderPhaseDefinitionRequest,
    responses(
        (status = 200, description = "Fase actualizada", body = crate::models::OrderPhaseResponse),
        (status = 400, description = "Datos inválidos", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Sin permisos", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn update_order_phase_definition_handler(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, phase_number)): Path<(Uuid, i32)>,
    Json(req): Json<UpdateOrderPhaseDefinitionRequest>,
) -> Result<Json<crate::models::OrderPhaseResponse>, AppError> {
    auth.require_role(&[UserRole::Employee, UserRole::Admin])?;

    let phase = OrderService::update_phase_definition(
        &state.pool,
        order_id,
        phase_number,
        auth.user_id,
        auth.effective_role,
        req,
    )
    .await?;

    Ok(Json(phase))
}

/// Asignar empleado a una orden (solo admin)
#[utoipa::path(
    put,
    path = "/api/orders/{order_id}/assign/{employee_id}",
    params(
        ("order_id" = Uuid, Path, description = "ID de la orden"),
        ("employee_id" = Uuid, Path, description = "ID del empleado"),
    ),
    responses(
        (status = 200, description = "Orden asignada"),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Sin permisos", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn assign_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, employee_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let order = OrderService::assign_order(&state.pool, order_id, employee_id).await?;

    /* [104A-38] Notificar al empleado asignado */
    let _ = state
        .notification_hub
        .notify(CreateNotification {
            user_id: employee_id,
            notification_type: NOTIF_ORDER_ASSIGNED.to_string(),
            title: format!("Te asignaron la orden #{}", order.order_number),
            body: None,
            link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
            reference_type: Some("order".to_string()),
            reference_id: Some(order.id),
        })
        .await;

    /* [154A-15d] Registrar asignación en activity_log */
    let _ = ActivityLogRepository::log(
        &state.pool,
        auth.user_id,
        "employee_assigned",
        "order",
        order_id,
        Some(serde_json::json!({"employee_id": employee_id.to_string()})),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "status": order.status, "assigned_employee_id": order.assigned_employee_id }),
    ))
}

/* [016A-5] Desasignar empleado de una orden (admin). Vuelve a awaiting_assignment. */
#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/unassign",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    responses(
        (status = 200, description = "Orden desasignada"),
        (status = 401, description = "No autorizado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "No encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn unassign_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let order = OrderRepository::unassign_order(&state.pool, order_id).await?;

    /* Registrar en activity_log */
    let _ = ActivityLogRepository::log(
        &state.pool,
        auth.user_id,
        "order_unassigned",
        "order",
        order_id,
        Some(serde_json::json!({"previous_employee_id": order.assigned_employee_id})),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "status": order.status, "assigned_employee_id": serde_json::Value::Null }),
    ))
}

/* [035A-30] Agregar una nueva fase bloqueada al final de la orden phased. */
pub async fn add_order_phase_handler(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    auth.require_role(&[UserRole::Employee, UserRole::Admin])?;
    let phase =
        OrderService::add_phase(&state.pool, order_id, auth.user_id, auth.effective_role).await?;
    Ok((StatusCode::CREATED, Json(phase)))
}

/* [035A-30] Eliminar una fase bloqueada de la orden phased. */
pub async fn delete_order_phase_handler(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, phase_number)): Path<(Uuid, i32)>,
) -> Result<StatusCode, AppError> {
    auth.require_role(&[UserRole::Employee, UserRole::Admin])?;
    OrderService::delete_phase(
        &state.pool,
        order_id,
        phase_number,
        auth.user_id,
        auth.effective_role,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/* [277A-10] Endpoint público para verificar si el usuario califica para descuento de primer pedido.
 * El frontend usa esto para mostrar el banner "50% OFF" antes del checkout. */
async fn check_first_order_discount(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Client, UserRole::Admin])?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM orders WHERE client_id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| AppError::Internal(format!("Error verificando órdenes: {e}")))?;

    Ok(Json(serde_json::json!({
        "qualifies": count == 0,
        "discount_percent": if count == 0 { 50 } else { 0 },
    })))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders/first-order-discount", get(check_first_order_discount))
        .route("/orders", post(create_order).get(list_orders))
        .route("/orders/:order_id", get(get_order))
        .route(
            "/orders/:order_id/project-description",
            patch(update_order_project_description_handler),
        )
        .route("/orders/:order_id/assign/:employee_id", put(assign_order))
        .route("/orders/:order_id/unassign", post(unassign_order))
        .route("/orders/:order_id/phases", post(add_order_phase_handler))
        .route(
            "/orders/:order_id/phases/:phase_number",
            patch(update_order_phase_definition_handler).delete(delete_order_phase_handler),
        )
}
