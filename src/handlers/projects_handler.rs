use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use crate::services::project_svc::ProjectService;
use crate::AppState;

/// Crear proyecto (admin)
pub async fn create_project(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), AppError> {
    let project = ProjectService::create(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

/// Listar proyectos (publico — solo visibles)
pub async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<Project>>, AppError> {
    let projects = ProjectService::list_visible(&state.pool).await?;
    Ok(Json(projects))
}

/// Listar todos los proyectos (admin)
pub async fn list_all_projects(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<Vec<Project>>, AppError> {
    let projects = ProjectService::list_all(&state.pool).await?;
    Ok(Json(projects))
}

/// Actualizar proyecto (admin)
pub async fn update_project(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, AppError> {
    let project = ProjectService::update(&state.pool, id, req).await?;
    Ok(Json(project))
}

/// Eliminar proyecto (admin)
pub async fn delete_project(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    ProjectService::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        /* Públicos */
        .route("/projects", get(list_projects))
        /* Admin */
        .route(
            "/admin/projects",
            get(list_all_projects).post(create_project),
        )
        .route(
            "/admin/projects/{id}",
            axum::routing::put(update_project).delete(delete_project),
        )
}
