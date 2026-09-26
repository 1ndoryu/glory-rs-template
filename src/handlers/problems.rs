/* [104A-28] Handlers de problemas reportados en órdenes.
 * POST /orders/:id/report-problem — cliente o empleado reporta problema con razón.
 * GET /admin/problems — admin lista todos los problemas.
 * GET /orders/:id/problems — problemas de una orden específica.
 * PATCH /admin/problems/:id/resolve — admin resuelve/descarta un problema. */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateNotification, ProblemAction, ProblemResponse, ProblemStatus, ReportProblemRequest,
    ResolveProblemRequest, UserRole,
};
use crate::repositories::{OrderRepository, ProblemRepository, UserRepository};
use crate::AppState;

/* ============================================================
REPORTAR PROBLEMA
============================================================ */

/// Reportar un problema en una orden (cliente o empleado vinculado)
#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/report-problem",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    request_body = ReportProblemRequest,
    responses(
        (status = 201, description = "Problema reportado", body = ProblemResponse),
        (status = 400, description = "Datos inválidos"),
        (status = 403, description = "Sin permisos"),
    ),
    security(("bearer_auth" = [])),
    tag = "problems"
)]
pub async fn report_problem(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<ReportProblemRequest>,
) -> Result<(StatusCode, Json<ProblemResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    /* Verificar que el usuario tiene relación con la orden */
    let role_str = match auth.effective_role {
        UserRole::Client => {
            if order.client_id != auth.user_id {
                return Err(AppError::Forbidden(
                    "No eres el cliente de esta orden".into(),
                ));
            }
            "client"
        }
        UserRole::Employee => {
            if order.assigned_employee_id != Some(auth.user_id) {
                return Err(AppError::Forbidden("No estás asignado a esta orden".into()));
            }
            "employee"
        }
        UserRole::Admin => "admin",
    };

    let problem =
        ProblemRepository::create(&state.pool, order_id, auth.user_id, role_str, &req.reason)
            .await?;

    /* Notificar a todos los admins */
    let admins = UserRepository::admin_ids(&state.pool)
        .await
        .unwrap_or_default();

    let base_notif = CreateNotification {
        user_id: Uuid::nil(),
        notification_type: "problem_reported".to_string(),
        title: format!("Problema reportado en orden #{}", order.order_number),
        body: Some(req.reason.chars().take(100).collect()),
        link: Some(format!("/panel?seccion=problemas&id={}", problem.id)),
        reference_type: Some("order_problem".to_string()),
        reference_id: Some(problem.id),
    };
    let _ = state
        .notification_hub
        .notify_many(&admins, &base_notif)
        .await;

    let reporter_name = UserRepository::display_name_or_email(&state.pool, auth.user_id)
        .await
        .unwrap_or_else(|_| "Desconocido".to_string());

    /* [311A-1] Email al cliente notificando problema reportado (non-fatal) */
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
            let ptitle = req.reason.chars().take(80).collect::<String>();
            let pdesc = req.reason.clone();
            let site_url =
                std::env::var("SITE_URL").unwrap_or_else(|_| "https://nakomi.studio".to_string());
            tokio::spawn(async move {
                crate::services::EmailService::send_problem_reported_client(
                    &cfg,
                    &pool,
                    &client_email,
                    &cname,
                    onum,
                    &ptitle,
                    &pdesc,
                    &site_url,
                    oid,
                )
                .await;
            });
        }
    }
    /* [011A-1] Email a admins notificando problema reportado (non-fatal) */
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
                let ptitle = req.reason.chars().take(80).collect::<String>();
                let pdesc = req.reason.clone();
                let site_url = std::env::var("SITE_URL")
                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                tokio::spawn(async move {
                    crate::services::EmailService::send_problem_reported_admin(
                        &cfg,
                        &pool,
                        &admin_emails,
                        &cname,
                        &cemail,
                        onum,
                        &ptitle,
                        &pdesc,
                        oid,
                        &site_url,
                    )
                    .await;
                });
            }
        }
    }
    let resp = ProblemResponse {
        id: problem.id,
        order_id: problem.order_id,
        order_number: order.order_number,
        reporter_id: problem.reporter_id,
        reporter_name,
        reporter_role: problem.reporter_role,
        reason: problem.reason,
        status: problem.status,
        admin_response: problem.admin_response,
        resolved_by: problem.resolved_by,
        resolved_at: problem.resolved_at.map(|dt| dt.to_rfc3339()),
        created_at: problem.created_at.to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(resp)))
}

/* ============================================================
LISTAR PROBLEMAS (admin)
============================================================ */

/// Lista todos los problemas reportados (solo admin)
#[utoipa::path(
    get,
    path = "/api/admin/problems",
    responses(
        (status = 200, description = "Lista de problemas", body = Vec<ProblemResponse>),
        (status = 403, description = "Solo admin"),
    ),
    security(("bearer_auth" = [])),
    tag = "problems"
)]
pub async fn list_problems(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ProblemResponse>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let rows = ProblemRepository::list_all(&state.pool).await?;
    let responses: Vec<ProblemResponse> = rows
        .into_iter()
        .map(|r| ProblemResponse {
            id: r.id,
            order_id: r.order_id,
            order_number: r.order_number,
            reporter_id: r.reporter_id,
            reporter_name: r.reporter_name,
            reporter_role: r.reporter_role,
            reason: r.reason,
            status: r.status,
            admin_response: r.admin_response,
            resolved_by: r.resolved_by,
            resolved_at: r.resolved_at.map(|dt| dt.to_rfc3339()),
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(responses))
}

/// Lista problemas de una orden específica
#[utoipa::path(
    get,
    path = "/api/orders/{order_id}/problems",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    responses(
        (status = 200, description = "Problemas de la orden", body = Vec<ProblemResponse>),
    ),
    security(("bearer_auth" = [])),
    tag = "problems"
)]
pub async fn list_order_problems(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Vec<ProblemResponse>>, AppError> {
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    /* Solo participantes de la orden o admin */
    let is_participant = order.client_id == auth.user_id
        || order.assigned_employee_id == Some(auth.user_id)
        || auth.effective_role == UserRole::Admin;
    if !is_participant {
        return Err(AppError::Forbidden("Sin acceso a esta orden".into()));
    }

    let rows = ProblemRepository::list_by_order(&state.pool, order_id).await?;
    let responses: Vec<ProblemResponse> = rows
        .into_iter()
        .map(|r| ProblemResponse {
            id: r.id,
            order_id: r.order_id,
            order_number: r.order_number,
            reporter_id: r.reporter_id,
            reporter_name: r.reporter_name,
            reporter_role: r.reporter_role,
            reason: r.reason,
            status: r.status,
            admin_response: r.admin_response,
            resolved_by: r.resolved_by,
            resolved_at: r.resolved_at.map(|dt| dt.to_rfc3339()),
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(responses))
}

/* ============================================================
RESOLVER PROBLEMA (admin)
============================================================ */

/// Admin resuelve o descarta un problema
#[utoipa::path(
    patch,
    path = "/api/admin/problems/{problem_id}/resolve",
    params(("problem_id" = Uuid, Path, description = "ID del problema")),
    request_body = ResolveProblemRequest,
    responses(
        (status = 200, description = "Problema actualizado", body = ProblemResponse),
        (status = 403, description = "Solo admin"),
        (status = 404, description = "Problema no encontrado"),
    ),
    security(("bearer_auth" = [])),
    tag = "problems"
)]
pub async fn resolve_problem(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(problem_id): Path<Uuid>,
    Json(req): Json<ResolveProblemRequest>,
) -> Result<Json<ProblemResponse>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let existing = ProblemRepository::find_by_id(&state.pool, problem_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Problema no encontrado".into()))?;

    if existing.status == ProblemStatus::Resolved || existing.status == ProblemStatus::Dismissed {
        return Err(AppError::BadRequest("Este problema ya fue resuelto".into()));
    }

    let new_status = match req.action {
        ProblemAction::Resolve => ProblemStatus::Resolved,
        ProblemAction::Dismiss => ProblemStatus::Dismissed,
    };

    let updated = ProblemRepository::resolve(
        &state.pool,
        problem_id,
        new_status,
        auth.user_id,
        req.response.as_deref(),
    )
    .await?;

    /* Notificar al reporter que su problema fue atendido */
    let action_label = match req.action {
        ProblemAction::Resolve => "resuelto",
        ProblemAction::Dismiss => "descartado",
    };

    let order_number = OrderRepository::find_order_by_id(&state.pool, existing.order_id)
        .await?
        .map_or(0, |o| o.order_number);

    let notif = CreateNotification {
        user_id: existing.reporter_id,
        notification_type: "problem_resolved".to_string(),
        title: format!("Tu problema de la orden #{order_number} fue {action_label}"),
        body: req.response.clone(),
        link: Some(format!(
            "/panel?seccion=proyectos&orden={}",
            existing.order_id
        )),
        reference_type: Some("order_problem".to_string()),
        reference_id: Some(problem_id),
    };
    let _ = state.notification_hub.notify(notif).await;

    let reporter_name = UserRepository::display_name_or_email(&state.pool, existing.reporter_id)
        .await
        .unwrap_or_else(|_| "Desconocido".to_string());

    let resp = ProblemResponse {
        id: updated.id,
        order_id: updated.order_id,
        order_number,
        reporter_id: updated.reporter_id,
        reporter_name,
        reporter_role: updated.reporter_role,
        reason: updated.reason,
        status: updated.status,
        admin_response: updated.admin_response,
        resolved_by: updated.resolved_by,
        resolved_at: updated.resolved_at.map(|dt| dt.to_rfc3339()),
        created_at: updated.created_at.to_rfc3339(),
    };

    Ok(Json(resp))
}

/* ============================================================
RUTAS
============================================================ */

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders/:order_id/report-problem", post(report_problem))
        .route("/orders/:order_id/problems", get(list_order_problems))
        .route("/admin/problems", get(list_problems))
        .route(
            "/admin/problems/:problem_id/resolve",
            patch(resolve_problem),
        )
}
