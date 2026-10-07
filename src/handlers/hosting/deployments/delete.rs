/* [01AA-4-f3o] Borrado de despliegues huérfanos (extraido de deployments.rs).
 * [165A-4] Solo borra stacks sin vínculo en BD para evitar desalinear
 * suscripciones reales. [245A-8] Resuelve el runtime antes de borrar para
 * soportar coexistencia entre Coolify legacy y runtime lightweight. */

use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::super::deployment_helpers::{
    invalidate_deployments_cache, locate_runtime_deployment, runtime_link_key,
};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::repositories::HostingRepository;
use crate::services::{HostingRuntimeKind, HostingRuntimeService};
use crate::AppState;

/* [165A-4] Permite limpiar despliegues huérfanos desde el panel admin.
 * Solo borra stacks sin vínculo en BD para evitar desalinear suscripciones reales.
 * [245A-8] Ahora resuelve el runtime antes de borrar para soportar coexistencia
 * entre Coolify legacy y runtime lightweight sin mezclar identidades. */
#[utoipa::path(
    delete,
    path = "/api/hosting/deployments/{deployment_uuid}",
    params(("deployment_uuid" = String, Path, description = "UUID del despliegue en Coolify")),
    responses(
        (status = 204, description = "Despliegue eliminado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Despliegue no encontrado"),
        (status = 409, description = "El despliegue ya está vinculado a una suscripción"),
        (status = 503, description = "Runtime no configurado o no disponible"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
#[allow(clippy::too_many_lines)]
pub(crate) async fn delete_deployment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(deployment_uuid): Path<String>,
) -> Result<StatusCode, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let subscriptions = HostingRepository::list_all(&state.pool).await?;
    let located = locate_runtime_deployment(&state, &deployment_uuid).await?;

    let can_link_by_name = located
        .deployment_name_counts
        .get(&runtime_link_key(
            located.runtime_kind,
            &located.target_name,
        ))
        .copied()
        .unwrap_or(0)
        <= 1;
    let linked_subscription = subscriptions.iter().find(|subscription| {
        HostingRuntimeKind::from_persisted(&subscription.runtime_kind) == located.runtime_kind
            && (subscription.deployment_id_or_legacy() == Some(deployment_uuid.as_str())
                || (can_link_by_name
                    && subscription.coolify_site_name.as_deref()
                        == Some(located.target_name.as_str())))
    });

    if let Some(subscription) = linked_subscription {
        return Err(AppError::Conflict(format!(
            "El despliegue {} ya está vinculado a la suscripción {}. Elimínalo desde la suscripción para no dejar datos huérfanos.",
            located.target_name, subscription.id
        )));
    }

    HostingRuntimeService::delete_deployment(
        &state.http_client,
        located.target_config,
        Some(located.runtime_kind),
        &deployment_uuid,
        true,
    )
    .await?;
    invalidate_deployments_cache(Some(&deployment_uuid)).await;
    tracing::info!(
        "[deployments] Despliegue huérfano {} ({}) eliminado desde el panel admin.",
        located.target_name,
        deployment_uuid
    );

    Ok(StatusCode::NO_CONTENT)
}
