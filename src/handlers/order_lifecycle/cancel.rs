/* [01AA-4-F2] Cancelación de órdenes (era parte de order_lifecycle.rs).
 * [044A-38 Fase 2] CANCELAR, ENTREGAR, APROBAR, REVISIÓN */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{CreateNotification, Order, NOTIF_ORDER_CANCELLED};
/* [07AA-7] Import sin uso (warning rustc al validar el bloque). */
use crate::repositories::{ActivityLogRepository, UserRepository};
use crate::services::OrderService;
use crate::AppState;

/// Cancelar una orden (cliente dueño, empleado asignado con razón, o admin)
#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/cancel",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    request_body(content = Option<crate::models::CancelOrderRequest>, description = "Razón opcional (obligatoria para empleados)"),
    responses(
        (status = 200, description = "Orden cancelada"),
        (status = 400, description = "Estado no permite cancelación", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Sin permisos", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn cancel_order_handler(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
    body: Option<Json<crate::models::CancelOrderRequest>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let reason = body.and_then(|b| b.0.reason);
    let order = OrderService::cancel_order(
        &state.pool,
        order_id,
        auth.user_id,
        auth.effective_role,
        reason.as_deref(),
    )
    .await?;

    /* [01AA-4-F2] Notificaciones + activity + emails fuera del handler. */
    notify_order_cancelled(&state, &order, reason.as_deref(), auth.user_id).await;

    Ok(Json(serde_json::json!({ "status": order.status })))
}

/* [01AA-4-F2] Partes + admins + activity_log de cancelación. */
async fn notify_order_cancelled(
    state: &AppState,
    order: &Order,
    reason: Option<&str>,
    actor_id: Uuid,
) {
    /* [104A-38] Notificar a las partes afectadas por la cancelación */
    let mut recipients = Vec::new();
    if actor_id != order.client_id {
        recipients.push(order.client_id);
    }
    if let Some(emp) = order.assigned_employee_id {
        if emp != actor_id {
            recipients.push(emp);
        }
    }
    let base = CreateNotification {
        user_id: Uuid::nil(),
        notification_type: NOTIF_ORDER_CANCELLED.to_string(),
        title: format!("Orden #{} cancelada", order.order_number),
        body: reason.map(|r| r.chars().take(100).collect()),
        link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
        reference_type: Some("order".to_string()),
        reference_id: Some(order.id),
    };
    let _ = state.notification_hub.notify_many(&recipients, &base).await;

    /* [20CA-10] Notificar también a admins sobre cancelación directa */
    if let Ok(admin_ids) = UserRepository::admin_ids(&state.pool).await {
        let admins_filtered: Vec<Uuid> = admin_ids
            .into_iter()
            .filter(|id| !recipients.contains(id))
            .collect();
        if !admins_filtered.is_empty() {
            let admin_base = CreateNotification {
                user_id: Uuid::nil(),
                notification_type: NOTIF_ORDER_CANCELLED.to_string(),
                title: format!("Orden #{} cancelada", order.order_number),
                body: reason
                    .map(|r| format!("Motivo: {}", r.chars().take(100).collect::<String>())),
                link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
                reference_type: Some("order".to_string()),
                reference_id: Some(order.id),
            };
            let _ = state
                .notification_hub
                .notify_many(&admins_filtered, &admin_base)
                .await;
        }
    }

    /* [154A-15d] Registrar cancelación en activity_log */
    let _ = ActivityLogRepository::log(
        &state.pool,
        actor_id,
        "order_cancelled",
        "order",
        order.id,
        Some(serde_json::json!({"reason": reason})),
    )
    .await;

    email_order_cancelled(state, order, reason).await;
}

/* [01AA-4-F2] Emails de cancelación (cliente + admins, non-fatal). */
async fn email_order_cancelled(state: &AppState, order: &Order, reason: Option<&str>) {
    /* [311A-1] Email al cliente notificando cancelación (non-fatal) */
    if let Some(ref email_cfg) = state.email_config {
        let reason_clone = reason
            .map(str::to_string)
            .unwrap_or_else(|| "Sin motivo especificado".to_string());
        if let Ok(Some(client_email)) =
            UserRepository::get_email(&state.pool, order.client_id).await
        {
            let cfg = email_cfg.clone();
            let pool = state.pool.clone();
            let onum = order.order_number;
            let oid = order.id;
            let cname = UserRepository::get_display_name(&state.pool, order.client_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "Cliente".to_string());
            tokio::spawn(async move {
                crate::services::EmailService::send_order_cancelled_client(
                    &cfg,
                    &pool,
                    &client_email,
                    &cname,
                    onum,
                    &reason_clone,
                    oid,
                )
                .await;
            });
        }
    }

    /* [011A-1] Email a admins notificando cancelación (non-fatal) */
    if let Some(ref email_cfg) = state.email_config {
        if let Ok(admin_emails) = UserRepository::admin_emails(&state.pool).await {
            if !admin_emails.is_empty() {
                let cfg = email_cfg.clone();
                let pool = state.pool.clone();
                let onum = order.order_number;
                let oid = order.id;
                let cname = UserRepository::get_display_name(&state.pool, order.client_id)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "Cliente".to_string());
                let cemail = UserRepository::get_email(&state.pool, order.client_id)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "desconocido@email.com".to_string());
                let reason_clone = reason
                    .map(str::to_string)
                    .unwrap_or_else(|| "Sin motivo especificado".to_string());
                let site_url = std::env::var("SITE_URL")
                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                tokio::spawn(async move {
                    crate::services::EmailService::send_order_cancelled_admin(
                        &cfg,
                        &pool,
                        &admin_emails,
                        &cname,
                        &cemail,
                        onum,
                        &reason_clone,
                        oid,
                        &site_url,
                    )
                    .await;
                });
            }
        }
    }
}
