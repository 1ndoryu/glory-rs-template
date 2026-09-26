/* [174A-2] Handlers de lifecycle de ordenes. Extraido de orders.rs para SRP.
 * POST /api/auth/switch-role
 * POST /api/orders/:id/cancel
 * POST /api/orders/:id/phases/:n/approve
 * POST /api/orders/:id/phases/:n/revision
 * PATCH /api/orders/:id/ai-intermediary
 * GET /api/orders/:id/activity */

use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateNotification, OrderResponse, OrderStatus, SwitchRoleRequest, ToggleAiIntermediaryRequest,
    UserRole, NOTIF_ORDER_CANCELLED, NOTIF_ORDER_COMPLETED, NOTIF_PAYMENT_RELEASED,
    NOTIF_PHASE_APPROVED, NOTIF_REVISION_REQUESTED,
};
use crate::repositories::{
    ActivityLogRepository, OrderRepository, UserRepository, WalletRepository,
};
use crate::services::{AuthService, OrderService, PaymentService};
use crate::AppState;

/* ============================================================
CAMBIO DE ROL (admin)
============================================================ */

/// Cambiar de vista impersonando un usuario real con el rol objetivo.
/// [084A-1] Si target es client/employee, busca un usuario real con ese rol y genera
/// token impersonado. Si target es admin, restaura el token del admin original.
#[utoipa::path(
    post,
    path = "/api/auth/switch-role",
    request_body = SwitchRoleRequest,
    responses(
        (status = 200, description = "Rol cambiado, nuevo token"),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Solo admins pueden cambiar rol", body = crate::errors::ErrorResponse),
        (status = 404, description = "No hay usuario con ese rol", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn switch_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SwitchRoleRequest>,
) -> Result<Json<crate::models::AuthResponse>, AppError> {
    /* [035A-21] Un JWT impersonado puede sobrevivir a reseeds locales y quedar
     * apuntando a un admin que ya no existe. Antes eso devolvía 500 al volver
     * a admin; ahora se trata como sesión inválida y el frontend puede limpiarla. */
    let admin_id = if auth.role == UserRole::Admin && auth.impersonator.is_none() {
        auth.user_id
    } else if let Some(imp) = auth.impersonator {
        imp
    } else {
        return Err(AppError::Forbidden(
            "Solo administradores pueden cambiar de rol".into(),
        ));
    };

    let original_admin = UserRepository::find_by_id(&state.pool, admin_id)
        .await?
        .filter(|user| user.role == UserRole::Admin)
        .ok_or_else(|| {
            AppError::Forbidden(
                "La sesión de impersonación ya no es válida. Inicia sesión de nuevo.".into(),
            )
        })?;

    match req.role {
        UserRole::Admin => {
            /* Restaurar sesión del admin original */
            let effective = original_admin.effective_role();
            let token = AuthService::generate_token(
                original_admin.id,
                original_admin.role,
                effective,
                None,
                &state.jwt_secret,
            )?;
            Ok(Json(crate::models::AuthResponse {
                token,
                user_id: original_admin.id,
                email: original_admin.email.clone(),
                role: original_admin.role,
                effective_role: effective,
                impersonating: false,
                needs_password: false,
            }))
        }
        target_role => {
            /* Impersonar un usuario real con el rol objetivo */
            let target = UserRepository::find_first_by_role(&state.pool, target_role)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("No hay usuario activo con rol {target_role}"))
                })?;
            let token = AuthService::generate_token(
                target.id,
                target.role,
                target.role,
                Some(original_admin.id),
                &state.jwt_secret,
            )?;
            Ok(Json(crate::models::AuthResponse {
                token,
                user_id: target.id,
                email: target.email.clone(),
                role: target.role,
                effective_role: target.role,
                impersonating: true,
                needs_password: false,
            }))
        }
    }
}

/* ============================================================
[044A-38 Fase 2] CANCELAR, ENTREGAR, APROBAR, REVISIÓN
============================================================ */

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

    /* [104A-38] Notificar a las partes afectadas por la cancelación */
    let mut recipients = Vec::new();
    if auth.user_id != order.client_id {
        recipients.push(order.client_id);
    }
    if let Some(emp) = order.assigned_employee_id {
        if emp != auth.user_id {
            recipients.push(emp);
        }
    }
    let base = CreateNotification {
        user_id: Uuid::nil(),
        notification_type: NOTIF_ORDER_CANCELLED.to_string(),
        title: format!("Orden #{} cancelada", order.order_number),
        body: reason.as_deref().map(|r| r.chars().take(100).collect()),
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
                    .as_deref()
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
        auth.user_id,
        "order_cancelled",
        "order",
        order_id,
        Some(serde_json::json!({"reason": reason})),
    )
    .await;

    /* [311A-1] Email al cliente notificando cancelación (non-fatal) */
    if let Some(ref email_cfg) = state.email_config {
        let reason_clone = reason
            .clone()
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
                    .clone()
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

    Ok(Json(serde_json::json!({ "status": order.status })))
}

/* [044A-38 Fase 6] deliver_phase movido a handlers/deliverables.rs con soporte multipart.
 * La ruta POST /orders/:id/phases/:n/deliver ahora acepta archivos + notas. */

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
        /* [20CA-10] Notificar a admins que una fase fue aprobada */
        if let Ok(admin_ids) = UserRepository::admin_ids(&state.pool).await {
            let admins_filtered: Vec<Uuid> = admin_ids
                .into_iter()
                .filter(|id| *id != auth.user_id)
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
        if order.status == OrderStatus::Completed {
            /* [154A-15d] Registrar orden completada */
            let _ = ActivityLogRepository::log(
                &state.pool,
                auth.user_id,
                "order_completed",
                "order",
                order_id,
                None,
            )
            .await;

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

            /* [204A-12] Comisiones: 90% al empleado, 10% a Nakomi (primer admin).
             * Solo aplica si hay empleado asignado y precio final > 0. */
            if let Some(emp_id) = order.assigned_employee_id {
                if order.final_price_cents > 0 {
                    credit_employee_and_commission(&state, &order, emp_id, order_id).await;
                }
            }
        }
    }

    Ok(Json(crate::models::OrderPhaseResponse::from(phase)))
}

/// Cliente solicita revisión de una fase entregada
#[utoipa::path(
    put,
    path = "/api/orders/{order_id}/phases/{phase_number}/revision",
    params(
        ("order_id" = Uuid, Path, description = "ID de la orden"),
        ("phase_number" = i32, Path, description = "Número de fase"),
    ),
    responses(
        (status = 200, description = "Revisión solicitada", body = crate::models::OrderPhaseResponse),
        (status = 400, description = "No se puede pedir revisión", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Solo el cliente dueño", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn request_revision(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, phase_number)): Path<(Uuid, i32)>,
) -> Result<Json<crate::models::OrderPhaseResponse>, AppError> {
    auth.require_role(&[UserRole::Client, UserRole::Admin])?;
    let phase =
        OrderService::request_revision(&state.pool, order_id, phase_number, auth.user_id).await?;

    /* [154A-15d] Registrar revisión solicitada en activity_log */
    let _ = ActivityLogRepository::log(
        &state.pool,
        auth.user_id,
        "revision_requested",
        "order",
        order_id,
        Some(serde_json::json!({"phase_number": phase_number})),
    )
    .await;

    /* [104A-38] Notificar al empleado asignado sobre la revisión solicitada */
    if let Some(order) = OrderRepository::find_order_by_id(&state.pool, order_id).await? {
        if let Some(emp) = order.assigned_employee_id {
            let _ = state
                .notification_hub
                .notify(CreateNotification {
                    user_id: emp,
                    notification_type: NOTIF_REVISION_REQUESTED.to_string(),
                    title: format!(
                        "Revisión solicitada — Orden #{}, Fase {}",
                        order.order_number, phase_number
                    ),
                    body: Some("El cliente solicitó cambios en la entrega".to_string()),
                    link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
                    reference_type: Some("order".to_string()),
                    reference_id: Some(order.id),
                })
                .await;
        }

        /* [20CA-10] Notificar también a todos los admins */
        if let Ok(admin_ids) = crate::repositories::UserRepository::admin_ids(&state.pool).await {
            let admins_filtered: Vec<Uuid> = admin_ids
                .into_iter()
                .filter(|id| Some(*id) != order.assigned_employee_id && *id != auth.user_id)
                .collect();
            if !admins_filtered.is_empty() {
                let rev_base = CreateNotification {
                    user_id: Uuid::nil(),
                    notification_type: NOTIF_REVISION_REQUESTED.to_string(),
                    title: format!(
                        "Revisión solicitada — Orden #{}, Fase {}",
                        order.order_number, phase_number
                    ),
                    body: Some(format!(
                        "El cliente solicitó cambios en la entrega de la orden #{}",
                        order.order_number
                    )),
                    link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
                    reference_type: Some("order".to_string()),
                    reference_id: Some(order.id),
                };
                let _ = state
                    .notification_hub
                    .notify_many(&admins_filtered, &rev_base)
                    .await;
            }
        }
    }

    Ok(Json(crate::models::OrderPhaseResponse::from(phase)))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders/:order_id/cancel", post(cancel_order_handler))
        .route(
            "/orders/:order_id/phases/:phase_number/approve",
            put(approve_phase),
        )
        .route(
            "/orders/:order_id/phases/:phase_number/revision",
            put(request_revision),
        )
        .route(
            "/orders/:order_id/ai-intermediary",
            put(toggle_ai_intermediary),
        )
        .route("/orders/:order_id/activity", get(get_order_activity))
        .route("/auth/switch-role", post(switch_role))
}

/* [T-10] Toggle IA intermediaria por orden (solo admin/employee asignado) */
#[utoipa::path(
    put,
    path = "/api/orders/{order_id}/ai-intermediary",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    request_body = ToggleAiIntermediaryRequest,
    responses(
        (status = 200, description = "Toggle actualizado", body = OrderResponse),
        (status = 401, description = "No autorizado"),
        (status = 403, description = "Sin permisos"),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn toggle_ai_intermediary(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<ToggleAiIntermediaryRequest>,
) -> Result<Json<OrderResponse>, AppError> {
    auth.require_role(&[UserRole::Admin, UserRole::Employee])?;
    let order = OrderRepository::toggle_ai_intermediary(&state.pool, order_id, req.enabled).await?;

    let (svc_title, svc_slug, plan_name) =
        OrderRepository::get_order_display_info(&state.pool, order.service_id, order.plan_id)
            .await?;
    let phases = OrderRepository::list_order_phases(&state.pool, order.id).await?;
    let employee_name =
        OrderRepository::get_employee_display_name(&state.pool, order.assigned_employee_id).await?;

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let total_phases = phases.len() as i32;

    Ok(Json(OrderResponse {
        id: order.id,
        order_number: order.order_number,
        client_id: order.client_id,
        client_name: None,
        service_title: svc_title,
        service_slug: svc_slug,
        plan_name,
        payment_mode: order.payment_mode,
        base_price_cents: order.base_price_cents,
        discount_percent: order.discount_percent,
        final_price_cents: order.final_price_cents,
        currency: order.currency,
        status: order.status,
        assigned_employee_id: order.assigned_employee_id,
        assigned_employee_name: employee_name,
        current_phase: order.current_phase,
        total_phases,
        project_description: order.project_description,
        client_notes: order.client_notes,
        started_at: order.started_at,
        created_at: order.created_at,
        ai_intermediary_enabled: order.ai_intermediary_enabled.unwrap_or(false),
        ai_summary: order.ai_summary,
    }))
}

/* ============================================================
[154A-15d] GET /api/orders/:order_id/activity — timeline de actividad
============================================================ */

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ActivityEntry {
    pub id: uuid::Uuid,
    pub user_id: Option<uuid::Uuid>,
    pub action: String,
    pub details: Option<serde_json::Value>,
    pub created_at: String,
}

#[utoipa::path(
    get,
    path = "/api/orders/{order_id}/activity",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    responses(
        (status = 200, description = "Timeline de actividad", body = Vec<ActivityEntry>),
        (status = 401, description = "No autorizado"),
        (status = 403, description = "Sin permisos"),
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn get_order_activity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Vec<ActivityEntry>>, AppError> {
    /* Verificar que el usuario tiene acceso a la orden */
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    let is_admin = auth.role == UserRole::Admin;
    let is_client = order.client_id == auth.user_id;
    let is_employee = order.assigned_employee_id == Some(auth.user_id);

    if !is_admin && !is_client && !is_employee {
        return Err(AppError::Forbidden("Sin acceso a esta orden".into()));
    }

    let rows = ActivityLogRepository::list_by_entity(&state.pool, "order", order_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error consultando activity_log: {e}")))?;

    let entries: Vec<ActivityEntry> = rows
        .into_iter()
        .map(|r| ActivityEntry {
            id: r.id,
            user_id: r.user_id,
            action: r.action,
            details: r.details,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(entries))
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
