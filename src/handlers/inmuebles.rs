use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    AddFotoRequest, CreateInmuebleRequest, EstadoRequest, FotoPublica, Inmueble,
    PaginatedInmuebles, PaginationParams, PublicacionRequest, UpdateInmuebleRequest,
};
use crate::services::InmuebleService;
use crate::AppState;

/* [159A-1] CRUD admin del catálogo. Requiere JWT; la visibilidad
 * pública la decide el backend con `publicado` (PATCH publicacion). */

/// Crear un inmueble (admite payload vacío: borrador)
#[utoipa::path(
    post,
    path = "/api/admin/inmuebles",
    request_body = CreateInmuebleRequest,
    responses(
        (status = 201, description = "Inmueble creado", body = Inmueble),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_inmueble(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CreateInmuebleRequest>,
) -> Result<(StatusCode, Json<Inmueble>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let inmueble = InmuebleService::create(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(inmueble)))
}

/// Listar todos los inmuebles (publicados o no) con paginación
#[utoipa::path(
    get,
    path = "/api/admin/inmuebles",
    params(PaginationParams),
    responses(
        (status = 200, description = "Lista del catálogo", body = PaginatedInmuebles),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_inmuebles(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedInmuebles>, AppError> {
    let lista = InmuebleService::list_admin(&state.pool, params.page, params.per_page).await?;
    Ok(Json(lista))
}

/// Ver un inmueble por ID (incluye no publicados)
#[utoipa::path(
    get,
    path = "/api/admin/inmuebles/{id}",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    responses(
        (status = 200, description = "Inmueble encontrado", body = Inmueble),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_inmueble(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Inmueble>, AppError> {
    let inmueble = InmuebleService::get_admin(&state.pool, id).await?;
    Ok(Json(inmueble))
}

/// Actualización parcial de un inmueble
#[utoipa::path(
    put,
    path = "/api/admin/inmuebles/{id}",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    request_body = UpdateInmuebleRequest,
    responses(
        (status = 200, description = "Inmueble actualizado", body = Inmueble),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_inmueble(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateInmuebleRequest>,
) -> Result<Json<Inmueble>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let inmueble = InmuebleService::update(&state.pool, id, req).await?;
    Ok(Json(inmueble))
}

/// Publicar o despublicar un inmueble
#[utoipa::path(
    patch,
    path = "/api/admin/inmuebles/{id}/publicacion",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    request_body = PublicacionRequest,
    responses(
        (status = 200, description = "Visibilidad actualizada", body = Inmueble),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_publicacion(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PublicacionRequest>,
) -> Result<Json<Inmueble>, AppError> {
    let inmueble = InmuebleService::set_publicado(&state.pool, id, req.publicado).await?;
    Ok(Json(inmueble))
}

/// Cambiar el estado (`vendido`/`alquilado` despublican en la misma operación)
#[utoipa::path(
    patch,
    path = "/api/admin/inmuebles/{id}/estado",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    request_body = EstadoRequest,
    responses(
        (status = 200, description = "Estado actualizado", body = Inmueble),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_estado(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<EstadoRequest>,
) -> Result<Json<Inmueble>, AppError> {
    let inmueble = InmuebleService::set_estado(&state.pool, id, &req.estado).await?;
    Ok(Json(inmueble))
}

/// Eliminar un inmueble (borra sus fotos en cascada)
#[utoipa::path(
    delete,
    path = "/api/admin/inmuebles/{id}",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    responses(
        (status = 204, description = "Inmueble eliminado"),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_inmueble(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    InmuebleService::delete(&state.pool, &state.upload_dir, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Añadir una foto a un inmueble
#[utoipa::path(
    post,
    path = "/api/admin/inmuebles/{id}/fotos",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    request_body = AddFotoRequest,
    responses(
        (status = 201, description = "Foto añadida", body = FotoPublica),
        (status = 404, description = "Inmueble no encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_foto(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AddFotoRequest>,
) -> Result<(StatusCode, Json<FotoPublica>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let foto = InmuebleService::add_foto(&state.pool, id, req).await?;
    Ok((StatusCode::CREATED, Json(foto)))
}

/// Eliminar una foto
#[utoipa::path(
    delete,
    path = "/api/admin/fotos/{id}",
    params(("id" = Uuid, Path, description = "ID de la foto")),
    responses(
        (status = 204, description = "Foto eliminada"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_foto(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    InmuebleService::delete_foto(&state.pool, &state.upload_dir, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/inmuebles", post(create_inmueble).get(list_inmuebles))
        .route(
            "/inmuebles/:id",
            get(get_inmueble)
                .put(update_inmueble)
                .delete(delete_inmueble),
        )
        .route("/inmuebles/:id/publicacion", patch(set_publicacion))
        .route("/inmuebles/:id/estado", patch(set_estado))
        .route("/inmuebles/:id/fotos", post(add_foto))
        .route("/fotos/:id", delete(delete_foto))
}
