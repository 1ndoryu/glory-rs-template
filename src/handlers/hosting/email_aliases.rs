/* [265A-11] Handlers de alias de correo para hosting (Opcion A: Cloudflare Email Routing).
 * CRUD de aliases/reenvios: listar, crear, eliminar.
 * Preparado para Fase 2 (buzones) en estructura pero sin exponer endpoints. */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateEmailAliasRequest, EmailAliasResponse, EmailMailboxResponse, HostingEmailInfoResponse,
    UserRole,
};
use crate::repositories::HostingRepository;
use crate::AppState;

fn ensure_subscription_access(
    auth: &AuthUser,
    subscription_user_id: Option<Uuid>,
) -> Result<(), AppError> {
    if auth.effective_role == UserRole::Client && subscription_user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden(
            "Sin permisos para gestionar el correo de esta suscripcion".into(),
        ));
    }
    Ok(())
}

/* [265A-11] Obtener informacion completa de correo de una suscripcion: aliases activos + preparacion buzones.
 * GET /hosting/subscriptions/:id/email */
#[utoipa::path(
    get,
    path = "/api/hosting/subscriptions/{id}/email",
    responses(
        (status = 200, description = "Informacion de correo de la suscripcion", body = HostingEmailInfoResponse),
        (status = 401, description = "No autorizado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripcion no encontrada"),
    ),
    params(
        ("id" = Uuid, Path, description = "ID de la suscripcion de hosting"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub async fn get_email_info(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<HostingEmailInfoResponse>, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Suscripcion no encontrada".into()))?;

    ensure_subscription_access(&auth, sub.user_id)?;

    let aliases = HostingRepository::list_aliases(&state.pool, id).await?;
    let aliases_limit =
        HostingRepository::get_plan_included_aliases(&state.pool, &sub.plan).await?;
    let aliases_count = HostingRepository::count_active_aliases(&state.pool, id).await?;

    /* [265A-12] Buzones preparados (vacios hasta Fase 2) */
    let mailboxes: Vec<EmailMailboxResponse> = Vec::new();
    let mailboxes_limit =
        HostingRepository::get_plan_included_mailboxes(&state.pool, &sub.plan).await?;

    Ok(Json(HostingEmailInfoResponse {
        aliases: aliases.into_iter().map(Into::into).collect(),
        aliases_limit,
        aliases_used: aliases_count as i32,
        mailboxes,
        mailboxes_limit,
        mailboxes_used: 0,
    }))
}

/* [265A-11] Crear alias de correo (forwarding).
 * POST /hosting/subscriptions/:id/email/aliases */
#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/email/aliases",
    responses(
        (status = 201, description = "Alias creado", body = EmailAliasResponse),
        (status = 400, description = "Limite alcanzado o datos invalidos"),
        (status = 401, description = "No autorizado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Suscripcion no encontrada"),
        (status = 409, description = "El alias ya existe"),
    ),
    params(
        ("id" = Uuid, Path, description = "ID de la suscripcion de hosting"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub async fn create_alias(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateEmailAliasRequest>,
) -> Result<(StatusCode, Json<EmailAliasResponse>), AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Suscripcion no encontrada".into()))?;

    ensure_subscription_access(&auth, sub.user_id)?;

    /* Verificar limite del plan */
    let limit = HostingRepository::get_plan_included_aliases(&state.pool, &sub.plan).await?;
    let used = HostingRepository::count_active_aliases(&state.pool, id).await?;
    if used >= limit as i64 && limit > 0 {
        return Err(AppError::Validation(format!(
            "Limite de {} alias(es) alcanzado para el plan {}",
            limit, sub.plan
        )));
    }

    /* Si el plan no incluye aliases (basico), prohibir creacion */
    if limit == 0 {
        return Err(AppError::Validation(
            "El plan actual no incluye alias de correo".into(),
        ));
    }

    let alias = HostingRepository::create_alias(
        &state.pool,
        id,
        &req.alias.to_ascii_lowercase(),
        &req.domain.to_ascii_lowercase(),
        &req.destination,
    )
    .await
    .map_err(|e| {
        /* Mapear unique violation de PostgreSQL (23505) a 409 */
        match &e {
            AppError::Database(db_err) if db_err.to_string().contains("23505") => {
                return AppError::Conflict("El alias ya existe en esta suscripcion".into());
            }
            _ => {}
        }
        e
    })?;

    /* Registrar evento */
    let details = serde_json::json!({
        "action": "alias_created",
        "alias": format!("{}@{}", req.alias, req.domain),
        "destination": req.destination,
    });
    let _ =
        HostingRepository::add_event(&state.pool, id, "email_alias_created", Some(details)).await;

    Ok((StatusCode::CREATED, Json(EmailAliasResponse::from(alias))))
}

/* [265A-11] Eliminar alias de correo.
 * DELETE /hosting/subscriptions/:id/email/aliases/:alias_id */
#[utoipa::path(
    delete,
    path = "/api/hosting/subscriptions/{id}/email/aliases/{alias_id}",
    responses(
        (status = 204, description = "Alias eliminado"),
        (status = 401, description = "No autorizado"),
        (status = 403, description = "Sin permisos"),
        (status = 404, description = "Alias no encontrado"),
    ),
    params(
        ("id" = Uuid, Path, description = "ID de la suscripcion de hosting"),
        ("alias_id" = Uuid, Path, description = "ID del alias a eliminar"),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub async fn delete_alias(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, alias_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Suscripcion no encontrada".into()))?;

    ensure_subscription_access(&auth, sub.user_id)?;

    let alias = HostingRepository::get_alias_by_id(&state.pool, alias_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Alias no encontrado".into()))?;

    /* Verificar que el alias pertenece a esta suscripcion */
    if alias.subscription_id != id {
        return Err(AppError::Forbidden(
            "El alias no pertenece a esta suscripcion".into(),
        ));
    }

    HostingRepository::delete_alias(&state.pool, alias_id).await?;

    /* Registrar evento */
    let details = serde_json::json!({
        "action": "alias_deleted",
        "alias": format!("{}@{}", alias.alias, alias.domain),
    });
    let _ =
        HostingRepository::add_event(&state.pool, id, "email_alias_deleted", Some(details)).await;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_auth(user_id: Uuid, effective_role: UserRole) -> AuthUser {
        AuthUser {
            user_id,
            role: effective_role,
            effective_role,
            impersonator: None,
        }
    }

    #[test]
    fn ensure_access_client_own_subscription_allowed() {
        let uid = Uuid::new_v4();
        let auth = make_auth(uid, UserRole::Client);
        assert!(ensure_subscription_access(&auth, Some(uid)).is_ok());
    }

    #[test]
    fn ensure_access_client_other_subscription_forbidden() {
        let auth = make_auth(Uuid::new_v4(), UserRole::Client);
        let other_uid = Uuid::new_v4();
        let result = ensure_subscription_access(&auth, Some(other_uid));
        match result {
            Err(AppError::Forbidden(msg)) => {
                assert!(msg.contains("Sin permisos"));
            }
            other => panic!("expected Forbidden, got: {other:?}"),
        }
    }

    #[test]
    fn ensure_access_client_subscription_without_user_forbidden() {
        let auth = make_auth(Uuid::new_v4(), UserRole::Client);
        let result = ensure_subscription_access(&auth, None);
        match result {
            Err(AppError::Forbidden(_)) => {} /* esperado */
            other => panic!("expected Forbidden, got: {other:?}"),
        }
    }

    #[test]
    fn ensure_access_admin_any_subscription_allowed() {
        let auth = make_auth(Uuid::new_v4(), UserRole::Admin);
        let other_uid = Uuid::new_v4();
        assert!(ensure_subscription_access(&auth, Some(other_uid)).is_ok());
    }

    #[test]
    fn ensure_access_employee_any_subscription_allowed() {
        let auth = make_auth(Uuid::new_v4(), UserRole::Employee);
        let other_uid = Uuid::new_v4();
        assert!(ensure_subscription_access(&auth, Some(other_uid)).is_ok());
    }
}
