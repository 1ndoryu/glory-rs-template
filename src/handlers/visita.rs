use axum::extract::{Path, Query, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    PaginatedVisitas, UpdateEstadoVisita, Visita, VisitasAdminParams, ESTADOS_VISITA,
};
use crate::repositories::VisitaRepository;
use crate::AppState;

/* [E15] Panel de visitas de la IA: el admin lista las `pendiente` y las
 * confirma (con `fecha`: día cerrado con el visitante) o las cancela.
 * Solo se mueve desde `pendiente`: una visita cerrada no se reabre por
 * aquí (si cambia, se agenda otra). Todo requiere JWT. */

/// Listar visitas para el panel (requiere JWT; filtro opcional por estado)
#[utoipa::path(
    get,
    path = "/api/admin/visitas",
    params(VisitasAdminParams),
    responses(
        (status = 200, description = "Visitas agendadas por la IA", body = PaginatedVisitas),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_visitas(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<VisitasAdminParams>,
) -> Result<Json<PaginatedVisitas>, AppError> {
    if let Some(e) = params.estado.as_deref() {
        if !ESTADOS_VISITA.contains(&e) {
            return Err(AppError::Validation("Estado de visita no válido".into()));
        }
    }
    let page = params.page.max(1);
    let per_page = params.per_page.clamp(1, 100);
    let (rows, total) =
        VisitaRepository::list_admin(&state.pool, params.estado.as_deref(), page, per_page).await?;
    Ok(Json(PaginatedVisitas {
        items: rows.into_iter().map(Visita::from_admin).collect(),
        total,
        page,
        per_page,
    }))
}

/// Confirmar (con fecha) o cancelar una visita (requiere JWT)
#[utoipa::path(
    patch,
    path = "/api/admin/visitas/{id}",
    params(("id" = Uuid, Path, description = "ID de la visita")),
    request_body = UpdateEstadoVisita,
    responses(
        (status = 200, description = "Estado actualizado", body = Visita),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 409, description = "Visita ya cerrada", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn revisar_visita(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEstadoVisita>,
) -> Result<Json<Visita>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    /* Confirmar sin día cerrado no sirve: el visitante quedaría esperando */
    if req.estado == "confirmada" && req.fecha.is_none() {
        return Err(AppError::Validation(
            "Confirmar requiere fecha (YYYY-MM-DD)".into(),
        ));
    }
    let actual = VisitaRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Visita no encontrada".into()))?;
    if actual.estado != "pendiente" && actual.estado != req.estado {
        return Err(AppError::Conflict(format!(
            "La visita ya está {}",
            actual.estado
        )));
    }
    let fila = VisitaRepository::cambiar_estado(&state.pool, id, &req.estado, req.fecha)
        .await?
        .ok_or_else(|| AppError::NotFound("Visita no encontrada".into()))?;
    let titulo = VisitaRepository::titulo_de(&state.pool, fila.inmueble_id)
        .await?
        .unwrap_or_default();
    Ok(Json(Visita::from_row(fila, titulo)))
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/visitas", get(list_visitas))
        .route("/visitas/:id", patch(revisar_visita))
}
