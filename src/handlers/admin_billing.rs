/* [026B-1] Admin: gestión de cobros pendientes (billing_items).
 * Lista todos los billing_items con email de usuario y permite marcar pagado/pendiente. */

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{AdminBillingItemResponse, AdminUpdateBillingStatusRequest, UserRole};
use crate::repositories::{AdminBillingItem, BillingRepository};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListBillingQuery {
    pub status: Option<String>,
}

/// Lista todos los `billing_items` (admin only) con email del usuario
#[utoipa::path(
    get,
    path = "/api/admin/billing-items",
    params(
        ("status" = Option<String>, Query, description = "Filtrar: pending, paid, cancelled"),
    ),
    responses(
        (status = 200, description = "Lista de billing items", body = Vec<AdminBillingItemResponse>)
    ),
    security(("bearer_auth" = [])),
    tag = "admin-billing"
)]
pub async fn list_billing_items(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListBillingQuery>,
) -> Result<Json<Vec<AdminBillingItemResponse>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let items =
        BillingRepository::list_all_for_admin(&state.pool, params.status.as_deref()).await?;

    let response = items.into_iter().map(admin_item_to_response).collect();
    Ok(Json(response))
}

/// Cambia el status de un `billing_item` (paid ↔ pending)
#[utoipa::path(
    patch,
    path = "/api/admin/billing-items/{item_id}/status",
    params(("item_id" = Uuid, Path, description = "ID del billing item")),
    request_body = AdminUpdateBillingStatusRequest,
    responses(
        (status = 200, description = "Status actualizado"),
        (status = 400, description = "Status inválido"),
        (status = 404, description = "Billing item no encontrado"),
    ),
    security(("bearer_auth" = [])),
    tag = "admin-billing"
)]
pub async fn update_billing_status(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(item_id): Path<Uuid>,
    Json(req): Json<AdminUpdateBillingStatusRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    if req.status != "paid" && req.status != "pending" {
        return Err(AppError::Validation(
            "Status inválido: solo se permite 'paid' o 'pending'".into(),
        ));
    }

    BillingRepository::admin_update_status(&state.pool, item_id, &req.status).await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "status": req.status,
    })))
}

fn admin_item_to_response(item: AdminBillingItem) -> AdminBillingItemResponse {
    AdminBillingItemResponse {
        id: item.id,
        user_id: item.user_id,
        user_email: item.user_email,
        resource_type: item.resource_type,
        resource_id: item.resource_id,
        title: item.title,
        description: item.description,
        amount_cents: item.amount_cents,
        currency: item.currency,
        billing_period: item.billing_period,
        status: item.status,
        due_at: item.due_at,
        grace_period_ends_at: item.grace_period_ends_at,
        paid_at: item.paid_at,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/billing-items", get(list_billing_items))
        .route(
            "/admin/billing-items/:item_id/status",
            patch(update_billing_status),
        )
}
