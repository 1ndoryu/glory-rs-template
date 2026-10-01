/* [01AA-4-F2] Handlers menores del lifecycle (eran parte de order_lifecycle.rs):
 * switch-role, revisiones, AI-intermediary y activity. */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateNotification, SwitchRoleRequest, ToggleAiIntermediaryRequest, UserRole,
    NOTIF_REVISION_REQUESTED,
};
use crate::repositories::{ActivityLogRepository, OrderRepository, UserRepository};
use crate::services::{AuthService, OrderService};
use crate::AppState;

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

/* [T-10] Toggle IA intermediaria por orden (solo admin/employee asignado) */
#[utoipa::path(
    put,
    path = "/api/orders/{order_id}/ai-intermediary",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    request_body = ToggleAiIntermediaryRequest,
    responses(
        (status = 200, description = "Toggle actualizado", body = crate::models::OrderResponse),
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
) -> Result<Json<crate::models::OrderResponse>, AppError> {
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

    Ok(Json(crate::models::OrderResponse {
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
