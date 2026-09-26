/* [174A-2] Handlers de cancellation requests. Extraido de wallet.rs para SRP.
 * POST /api/orders/:order_id/cancel-request
 * POST /api/orders/:order_id/cancel-request/:request_id/respond
 * GET /api/orders/:order_id/cancel-request */

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CancellationRequestResponse, CreateCancellationRequest, CreateNotification, Order,
    RespondCancellationRequest, UserRole, NOTIF_ORDER_CANCELLED,
};
use crate::repositories::{
    ActivityLogRepository, CancellationRequestRepository, OrderRepository, UserRepository,
    WalletRepository,
};
use crate::AppState;

/* ============================================================
POST /api/orders/:order_id/cancel-request â€” Cliente o empleado solicita cancelaciÃ³n
[204A-13] Ambas partes pueden solicitar. La otra parte responde.
============================================================ */

#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/cancel-request",
    request_body = CreateCancellationRequest,
    responses(
        (status = 201, description = "Solicitud de cancelaciÃ³n creada", body = CancellationRequestResponse),
        (status = 400, description = "Ya existe solicitud pendiente"),
        (status = 403, description = "Solo cliente o responsable pueden solicitar"),
        (status = 404, description = "Orden no encontrada"),
    ),
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    security(("bearer" = []))
)]
pub async fn create_cancellation_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
    Json(body): Json<CreateCancellationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    /* [204A-13] Cliente o empleado asignado pueden solicitar cancelaciÃ³n.
     * Admin usa cancel directo. Cada parte notifica a la otra. */
    let is_employee = order.assigned_employee_id == Some(auth.user_id);
    let is_client = order.client_id == auth.user_id;
    if !is_employee && !is_client {
        return Err(AppError::Forbidden(
            "Solo el cliente o el responsable pueden solicitar cancelaciÃ³n".into(),
        ));
    }

    /* Verificar que no haya solicitud pendiente */
    if let Some(_pending) =
        CancellationRequestRepository::find_pending_by_order(&state.pool, order_id).await?
    {
        return Err(AppError::BadRequest(
            "Ya existe una solicitud de cancelaciÃ³n pendiente".into(),
        ));
    }

    /* Crear solicitud */
    let req =
        CancellationRequestRepository::create(&state.pool, order_id, auth.user_id, &body.reason)
            .await?;

    /* [204A-13] Notificar a la otra parte (cliente→empleado o empleado→cliente).
     * [20CA-6] Si el empleado solicitó, notificar también a admins. */
    let (notify_id, notify_body) = if is_client {
        let emp = order.assigned_employee_id.unwrap_or(Uuid::nil());
        (
            emp,
            format!(
                "El cliente solicita cancelar tu orden. Motivo: {}",
                truncate_str(&body.reason, 120)
            ),
        )
    } else {
        (
            order.client_id,
            format!(
                "El responsable solicita cancelar tu orden. Motivo: {}",
                truncate_str(&body.reason, 120)
            ),
        )
    };
    if notify_id != Uuid::nil() {
        let notif = CreateNotification {
            user_id: notify_id,
            notification_type: "cancellation_requested".to_string(),
            title: format!("Solicitud de cancelación Orden #{}", order.order_number),
            body: Some(notify_body),
            link: Some(format!("/panel?seccion=proyectos&orden={order_id}")),
            reference_type: Some("order".to_string()),
            reference_id: Some(order_id),
        };
        let _ = state.notification_hub.notify(notif).await;
    }

    /* [20CA-6] Si el empleado solicitó cancelación, notificar a admins */
    if is_employee {
        let admin_ids = UserRepository::admin_ids(&state.pool)
            .await
            .unwrap_or_default();
        if !admin_ids.is_empty() {
            let admin_notif = CreateNotification {
                user_id: Uuid::nil(),
                notification_type: "cancellation_requested".to_string(),
                title: format!(
                    "Solicitud de cancelaci\u{f3}n \u{2014} Orden #{}",
                    order.order_number
                ),
                body: Some(format!(
                    "El freelancer solicita cancelar la orden. Motivo: {}",
                    truncate_str(&body.reason, 120)
                )),
                link: Some(format!("/panel?seccion=proyectos&orden={order_id}")),
                reference_type: Some("order".to_string()),
                reference_id: Some(order_id),
            };
            let _ = state
                .notification_hub
                .notify_many(&admin_ids, &admin_notif)
                .await;
        }
    }

    /* Registrar en activity_log */
    let _ = ActivityLogRepository::log(
        &state.pool,
        auth.user_id,
        "cancellation_requested",
        "order",
        order_id,
        Some(serde_json::json!({ "reason": body.reason })),
    )
    .await;

    let resp = CancellationRequestResponse {
        id: req.id,
        order_id: req.order_id,
        order_number: Some(order.order_number),
        requested_by: req.requested_by,
        requester_name: None,
        reason: req.reason,
        status: req.status,
        resolved_by: req.resolved_by,
        resolved_at: req.resolved_at.map(|t| t.to_rfc3339()),
        created_at: req.created_at.to_rfc3339(),
    };
    Ok((StatusCode::CREATED, Json(resp)))
}

/* ============================================================
POST /api/orders/:order_id/cancel-request/:request_id/respond
â€” La parte opuesta acepta o rechaza cancelaciÃ³n
============================================================ */

#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/cancel-request/{request_id}/respond",
    request_body = RespondCancellationRequest,
    responses(
        (status = 200, description = "Respuesta procesada", body = CancellationRequestResponse),
        (status = 403, description = "Solo la otra parte puede responder"),
        (status = 404, description = "Solicitud no encontrada"),
    ),
    params(
        ("order_id" = Uuid, Path, description = "ID de la orden"),
        ("request_id" = Uuid, Path, description = "ID de la solicitud"),
    ),
    security(("bearer" = []))
)]
pub async fn respond_cancellation_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, request_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RespondCancellationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    /* [204A-13] La parte opuesta al solicitante puede responder.
     * Si el empleado solicitÃ³ â†’ responde el cliente.
     * Si el cliente solicitÃ³ â†’ responde el empleado asignado.
     * Admin siempre puede responder. */
    let req = CancellationRequestRepository::find_by_id(&state.pool, request_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Solicitud no encontrada".into()))?;

    let is_admin = auth.effective_role == UserRole::Admin;
    let is_opposing_party = if req.requested_by == order.client_id {
        /* Cliente solicitÃ³ â†’ empleado responde */
        order.assigned_employee_id == Some(auth.user_id)
    } else {
        /* Empleado solicitÃ³ â†’ cliente responde */
        order.client_id == auth.user_id
    };

    if !is_opposing_party && !is_admin {
        return Err(AppError::Forbidden(
            "Solo la otra parte o un administrador puede responder".into(),
        ));
    }

    if req.status != "pending" {
        return Err(AppError::BadRequest(
            "Esta solicitud ya fue resuelta".into(),
        ));
    }

    if req.order_id != order_id {
        return Err(AppError::NotFound(
            "Solicitud no pertenece a esta orden".into(),
        ));
    }

    /* Resolver la solicitud */
    let resolved =
        CancellationRequestRepository::resolve(&state.pool, request_id, auth.user_id, body.accept)
            .await?;

    if body.accept {
        handle_cancellation_accepted(&state, &order, order_id, auth.user_id).await;
    } else {
        handle_cancellation_rejected(&state, &order, order_id, auth.user_id).await;
    }

    let resp = CancellationRequestResponse {
        id: resolved.id,
        order_id: resolved.order_id,
        order_number: Some(order.order_number),
        requested_by: resolved.requested_by,
        requester_name: None,
        reason: resolved.reason,
        status: resolved.status,
        resolved_by: resolved.resolved_by,
        resolved_at: resolved.resolved_at.map(|t| t.to_rfc3339()),
        created_at: resolved.created_at.to_rfc3339(),
    };
    Ok(Json(resp))
}

/* ============================================================
HELPERS - LÃ³gica de aceptaciÃ³n/rechazo de cancelaciÃ³n
============================================================ */

async fn handle_cancellation_accepted(
    state: &AppState,
    order: &Order,
    order_id: Uuid,
    resolved_by: Uuid,
) {
    /* Cancelar orden */
    let _ = OrderRepository::cancel_order(&state.pool, order_id).await;

    /* Acreditar wallet del cliente */
    let _ = WalletRepository::credit(
        &state.pool,
        order.client_id,
        order.final_price_cents,
        "refund_credit",
        Some("order"),
        Some(order_id),
        Some(&format!(
            "Reembolso por cancelaciÃ³n de orden #{}",
            order.order_number
        )),
    )
    .await;

    /* Notificar al empleado */
    if let Some(emp_id) = order.assigned_employee_id {
        let notif = CreateNotification {
            user_id: emp_id,
            notification_type: NOTIF_ORDER_CANCELLED.to_string(),
            title: format!("Orden #{} cancelada", order.order_number),
            body: Some("El cliente aceptÃ³ tu solicitud de cancelaciÃ³n.".into()),
            link: None,
            reference_type: Some("order".into()),
            reference_id: Some(order_id),
        };
        let _ = state.notification_hub.notify(notif).await;
    }

    /* Activity log â€” cancelaciÃ³n aceptada */
    let _ = ActivityLogRepository::log(
        &state.pool,
        resolved_by,
        "cancellation_accepted",
        "order",
        order_id,
        Some(serde_json::json!({ "refund_cents": order.final_price_cents })),
    )
    .await;
}

async fn handle_cancellation_rejected(
    state: &AppState,
    order: &Order,
    order_id: Uuid,
    resolved_by: Uuid,
) {
    /* Orden vuelve a awaiting_assignment sin empleado */
    let _ = OrderRepository::reopen_after_rejection(&state.pool, order_id).await;

    /* Notificar al empleado */
    if let Some(emp_id) = order.assigned_employee_id {
        let notif = CreateNotification {
            user_id: emp_id,
            notification_type: "cancellation_rejected".into(),
            title: format!("CancelaciÃ³n rechazada â€” Orden #{}", order.order_number),
            body: Some(
                "El cliente rechazÃ³ tu solicitud. La orden volverÃ¡ a estar disponible.".into(),
            ),
            link: None,
            reference_type: Some("order".into()),
            reference_id: Some(order_id),
        };
        let _ = state.notification_hub.notify(notif).await;
    }

    /* Notificar a empleados disponibles */
    let employees = UserRepository::available_employee_ids(&state.pool)
        .await
        .unwrap_or_default();

    for emp_id in employees {
        let notif = CreateNotification {
            user_id: emp_id,
            notification_type: "order_available".into(),
            title: format!("Orden #{} disponible", order.order_number),
            body: Some("Una orden ha quedado disponible para ser tomada.".into()),
            link: Some("/panel?seccion=disponibles".into()),
            reference_type: Some("order".into()),
            reference_id: Some(order_id),
        };
        let _ = state.notification_hub.notify(notif).await;
    }

    /* Activity log â€” cancelaciÃ³n rechazada */
    let _ = ActivityLogRepository::log(
        &state.pool,
        resolved_by,
        "cancellation_rejected",
        "order",
        order_id,
        None,
    )
    .await;
}

/* ============================================================
GET /api/orders/:order_id/cancel-request â€” Lista solicitudes de cancelaciÃ³n
============================================================ */

#[utoipa::path(
    get,
    path = "/api/orders/{order_id}/cancel-request",
    responses(
        (status = 200, description = "Solicitudes de cancelaciÃ³n", body = Vec<CancellationRequestResponse>),
        (status = 403, description = "Sin permiso"),
        (status = 404, description = "Orden no encontrada"),
    ),
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    security(("bearer" = []))
)]
pub async fn list_cancellation_requests(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    /* Solo cliente, empleado asignado o admin pueden ver solicitudes */
    let is_involved = order.client_id == auth.user_id
        || order.assigned_employee_id == Some(auth.user_id)
        || auth.effective_role == crate::models::UserRole::Admin;
    if !is_involved {
        return Err(AppError::Forbidden(
            "Sin permiso para ver solicitudes".into(),
        ));
    }

    let requests = CancellationRequestRepository::list_by_order(&state.pool, order_id).await?;
    let resp: Vec<CancellationRequestResponse> = requests
        .into_iter()
        .map(|r| CancellationRequestResponse {
            id: r.id,
            order_id: r.order_id,
            order_number: Some(order.order_number),
            requested_by: r.requested_by,
            requester_name: None,
            reason: r.reason,
            status: r.status,
            resolved_by: r.resolved_by,
            resolved_at: r.resolved_at.map(|t| t.to_rfc3339()),
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(resp))
}

/* Helper para truncar strings en notificaciones */
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

pub fn cancellation_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/orders/:order_id/cancel-request",
            get(list_cancellation_requests).post(create_cancellation_request),
        )
        .route(
            "/orders/:order_id/cancel-request/:request_id/respond",
            post(respond_cancellation_request),
        )
}
