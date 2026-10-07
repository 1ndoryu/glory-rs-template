/* [07AA-14] Lectura de suscripciones VPS extraída de `vps.rs`: lista (admin/empleado
 * ven todos, cliente solo lo suyo) y detalle con chequeo de ownership. */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{UserRole, VpsSubscriptionResponse};
use crate::repositories::VpsRepository;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/vps/subscriptions",
    responses(
        (status = 200, description = "Lista de suscripciones VPS", body = Vec<VpsSubscriptionResponse>),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<VpsSubscriptionResponse>>, AppError> {
    let subscriptions =
        if auth.effective_role == UserRole::Admin || auth.effective_role == UserRole::Employee {
            VpsRepository::list_all(&state.pool).await?
        } else {
            VpsRepository::list_by_user_id(&state.pool, auth.user_id).await?
        };

    Ok(Json(subscriptions.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/api/vps/subscriptions/{id}",
    params(("id" = Uuid, Path, description = "ID de la suscripción VPS")),
    responses(
        (status = 200, description = "Suscripción VPS", body = VpsSubscriptionResponse),
        (status = 404, description = "No encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "vps"
)]
pub async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<VpsSubscriptionResponse>, AppError> {
    let subscription = VpsRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción VPS no encontrada".into()))?;

    if auth.effective_role == UserRole::Client && subscription.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden(
            "Sin permisos para ver esta suscripción VPS".into(),
        ));
    }

    Ok(Json(subscription.into()))
}
