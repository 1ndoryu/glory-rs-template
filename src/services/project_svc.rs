use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::project::{Project, CreateProjectRequest, UpdateProjectRequest};
use crate::repositories::project_repo::ProjectRepository;

pub struct ProjectService;

impl ProjectService {
    pub async fn create(pool: &PgPool, req: CreateProjectRequest) -> Result<Project, AppError> {
        let project = ProjectRepository::create(
            pool,
            &req.title,
            &req.description,
            req.url.as_deref(),
            req.sort_order,
        )
        .await?;
        Ok(project)
    }

    pub async fn list_all(pool: &PgPool) -> Result<Vec<Project>, AppError> {
        Ok(ProjectRepository::list_all(pool).await?)
    }

    pub async fn list_visible(pool: &PgPool) -> Result<Vec<Project>, AppError> {
        Ok(ProjectRepository::list_visible(pool).await?)
    }

    pub async fn update(pool: &PgPool, id: Uuid, req: UpdateProjectRequest) -> Result<Project, AppError> {
        ProjectRepository::update(
            pool,
            id,
            req.title.as_deref(),
            req.description.as_deref(),
            req.url.as_deref(),
            req.sort_order,
            req.is_visible,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Proyecto no encontrado".into()))
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !ProjectRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Proyecto no encontrado".into()));
        }
        Ok(())
    }
}
