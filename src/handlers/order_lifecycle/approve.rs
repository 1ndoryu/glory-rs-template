/* [01AA-4-F2] Aprobación de fases y flujo de orden completada
 * (era parte de order_lifecycle.rs). */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateNotification, Order, OrderStatus, UserRole, NOTIF_ORDER_COMPLETED,
    NOTIF_PAYMENT_RELEASED, NOTIF_PHASE_APPROVED,
};
use crate::repositories::{
    ActivityLogRepository, OrderRepository, UserRepository, WalletRepository,
};
use crate::services::{OrderService, PaymentService};
use crate::AppState;

/// Cliente aprueba una fase entregada
#[utoipa::path(
    put,
    path = "/api/orders/{order_id}/phases/{phase_number}/approve",
    params(
        ("order_id" = Uuid, Path, description = "ID de la orden"),
        ("phase_number" = i32, Path, description = "Número de fase"),
    ),
    responses(
        (status = 200, description = "Fase aprobada", body = crate::models::OrderPhaseResponse),
        (status = 400, description = "Estado no permite aprobación", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Solo el cliente dueño", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn approve_phase(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, phase_number)): Path<(Uuid, i32)>,
) -> Result<Json<crate::models::OrderPhaseResponse>, AppError> {
    auth.require_role(&[UserRole::Client, UserRole::Admin])?;
    let phase =
        OrderService::approve_phase(&state.pool, order_id, phase_number, auth.user_id).await?;

    /* [154A-15d] Registrar aprobación de fase en activity_log */
    let _ = ActivityLogRepository::log(
        &state.pool,
        auth.user_id,
        "phase_approved",
        "order",
        order_id,
        Some(serde_json::json!({"phase_number": phase_number})),
    )
    .await;

    /* [044A-38 Fase 3] Si la orden se completó, capturar los pagos retenidos en Stripe */
    if let Some(order) = OrderRepository::find_order_by_id(&state.pool, order_id).await? {
        notify_phase_approved(&state, &order, phase_number, auth.user_id).await;
        if order.status == OrderStatus::Completed {
            handle_order_completed(&state, &order, order_id, auth.user_id).await;
        }
    }

    Ok(Json(crate::models::OrderPhaseResponse::from(phase)))
}

/* [01AA-4-F2] Aviso a admins de fase aprobada. */
async fn notify_phase_approved(
    state: &AppState,
    order: &Order,
    phase_number: i32,
    actor_id: Uuid,
) {
    /* [20CA-10] Notificar a admins que una fase fue aprobada */
    if let Ok(admin_ids) = UserRepository::admin_ids(&state.pool).await {
        let admins_filtered: Vec<Uuid> = admin_ids
            .into_iter()
            .filter(|id| *id != actor_id)
            .collect();
        if !admins_filtered.is_empty() {
            let base = CreateNotification {
                user_id: Uuid::nil(),
                notification_type: NOTIF_PHASE_APPROVED.to_string(),
                title: format!(
                    "Orden #{} — Fase {} aprobada",
                    order.order_number, phase_number
                ),
                body: Some("El cliente aprobó la entrega".to_string()),
                link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
                reference_type: Some("order".to_string()),
                reference_id: Some(order.id),
            };
            let _ = state
                .notification_hub
                .notify_many(&admins_filtered, &base)
                .await;
        }
    }
}

/* [01AA-4-F2] Orquestador de orden completada: log + notifs + emails + captura + comisión. */
async fn handle_order_completed(
    state: &AppState,
    order: &Order,
    order_id: Uuid,
    actor_id: Uuid,
) {
    /* [154A-15d] Registrar orden completada */
    let _ = ActivityLogRepository::log(
        &state.pool,
        actor_id,
        "order_completed",
        "order",
        order_id,
        None,
    )
    .await;

    notify_order_completed(state, order).await;
    email_order_completed(state, order).await;
    capture_and_release(state, order, order_id).await;

    /* [204A-12] Comisiones: 90% al empleado, 10% a Nakomi (primer admin).
     * Solo aplica si hay empleado asignado y precio final > 0. */
    if let Some(emp_id) = order.assigned_employee_id {
        if order.final_price_cents > 0 {
            credit_employee_and_commission(state, order, emp_id, order_id).await;
        }
    }
}

/* [01AA-4-F2] Notifs de completado (partes + admins). */
async fn notify_order_completed(state: &AppState, order: &Order) {
    /* [104A-38] Notificar cliente y empleado sobre orden completada */
    let mut recipients = vec![order.client_id];
    if let Some(emp) = order.assigned_employee_id {
        recipients.push(emp);
    }
    let base = CreateNotification {
        user_id: Uuid::nil(),
        notification_type: NOTIF_ORDER_COMPLETED.to_string(),
        title: format!("Orden #{} completada", order.order_number),
        body: Some("Todas las fases han sido aprobadas".to_string()),
        link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
        reference_type: Some("order".to_string()),
        reference_id: Some(order.id),
    };
    let _ = state.notification_hub.notify_many(&recipients, &base).await;

    /* [20CA-10] Notificar a admins que la orden fue completada */
    if let Ok(admin_ids) = UserRepository::admin_ids(&state.pool).await {
        let admins_filtered: Vec<Uuid> = admin_ids
            .into_iter()
            .filter(|id| !recipients.contains(id))
            .collect();
        if !admins_filtered.is_empty() {
            let admin_base = CreateNotification {
                user_id: Uuid::nil(),
                notification_type: NOTIF_ORDER_COMPLETED.to_string(),
                title: format!("Orden #{} completada", order.order_number),
                body: Some(format!(
                    "Precio final: ${:.2}",
                    order.final_price_cents as f64 / 100.0
                )),
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
}

/* [01AA-4-F2] Emails de completado (cliente + admins, non-fatal). */
async fn email_order_completed(state: &AppState, order: &Order) {
    /* [311A-1] Email al cliente notificando orden completada (non-fatal) */
    if let Some(ref email_cfg) = state.email_config {
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
            let site_url = std::env::var("SITE_URL")
                .unwrap_or_else(|_| "https://nakomi.studio".to_string());
            tokio::spawn(async move {
                crate::services::EmailService::send_order_completed_client(
                    &cfg,
                    &pool,
                    &client_email,
                    &cname,
                    onum,
                    &site_url,
                    oid,
                )
                .await;
            });
        }
    }

    /* [011A-1] Email a admins notificando orden completada (non-fatal) */
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
                let site_url = std::env::var("SITE_URL")
                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                tokio::spawn(async move {
                    crate::services::EmailService::send_order_completed_admin(
                        &cfg,
                        &pool,
                        &admin_emails,
                        &cname,
                        &cemail,
                        onum,
                        oid,
                        &site_url,
                    )
                    .await;
                });
            }
        }
    }
}

/* [01AA-4-F2] Captura de pagos retenidos + aviso de liberación. */
async fn capture_and_release(state: &AppState, order: &Order, order_id: Uuid) {
    if let Some(ref stripe_key) = state.stripe_secret_key {
        if let Err(e) = PaymentService::capture_held_payments(
            &state.pool,
            &state.http_client,
            stripe_key,
            order_id,
        )
        .await
        {
            tracing::error!("Error capturando pagos de orden {order_id}: {e}");
        } else {
            /* [20CA-10] Notificar a admins que el pago fue liberado al empleado */
            if let Ok(admin_ids) = UserRepository::admin_ids(&state.pool).await {
                let emp_name = if let Some(eid) = order.assigned_employee_id {
                    UserRepository::get_display_name(&state.pool, eid)
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "Empleado".to_string())
                } else {
                    "Empleado".to_string()
                };
                let base = CreateNotification {
                    user_id: Uuid::nil(),
                    notification_type: NOTIF_PAYMENT_RELEASED.to_string(),
                    title: format!("Pago liberado — Orden #{}", order.order_number),
                    body: Some(format!(
                        "${:.2} liberados a {}",
                        order.final_price_cents as f64 / 100.0,
                        emp_name
                    )),
                    link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
                    reference_type: Some("order".to_string()),
                    reference_id: Some(order.id),
                };
                let _ = state.notification_hub.notify_many(&admin_ids, &base).await;
            }
        }
    }
}

/* [204A-12] Comisiones: al completar orden, 90% al empleado y 10% a Nakomi.
 * Nakomi = primer usuario con role 'admin'. Si no hay admin, se loguea error.
 * El crédito al wallet es atómico (cada credit usa FOR UPDATE en la BD). */
async fn credit_employee_and_commission(
    state: &AppState,
    order: &crate::models::Order,
    employee_id: Uuid,
    order_id: Uuid,
) {
    let total = order.final_price_cents;
    let commission_cents = total / 10; /* 10% para Nakomi */
    let employee_cents = total - commission_cents; /* 90% para empleado */

    /* Pagar al empleado */
    if let Err(e) = WalletRepository::credit(
        &state.pool,
        employee_id,
        employee_cents,
        "order_payment",
        Some("order"),
        Some(order_id),
        Some(&format!("Pago por orden #{} (90%)", order.order_number)),
    )
    .await
    {
        tracing::error!("Error acreditando wallet empleado {employee_id}: {e}");
    }

    /* Comisión a Nakomi (primer admin activo) */
    let admin_id = UserRepository::first_admin_id(&state.pool).await;

    match admin_id {
        Ok(Some(nakomi_id)) => {
            if let Err(e) = WalletRepository::credit(
                &state.pool,
                nakomi_id,
                commission_cents,
                "commission",
                Some("order"),
                Some(order_id),
                Some(&format!("Comisión 10% orden #{}", order.order_number)),
            )
            .await
            {
                tracing::error!("Error acreditando comisión a Nakomi: {e}");
            }
        }
        Ok(None) => {
            tracing::error!("No se encontró admin activo para comisión de orden {order_id}");
        }
        Err(e) => {
            tracing::error!("Error buscando admin para comisión: {e}");
        }
    }
}
