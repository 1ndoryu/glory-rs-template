use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::project::{
    CreateProjectRequest, Project, ProjectUrlUpdate, UpdateProjectRequest,
};
use crate::models::resource::{
    CreateResourceParams, EditorialState, ResourceKind, VisibilityState,
};
use crate::repositories::project_repo::{ProjectRepository, ProjectUpdateParams};
use crate::repositories::resource_repo::ResourceRepository;

pub struct ProjectService;

impl ProjectService {
    /// [297A-10] Crear proyecto con resource envelope en transacción. Defaults: draft, private.
    #[allow(clippy::explicit_auto_deref)]
    pub async fn create(pool: &PgPool, req: CreateProjectRequest) -> Result<Project, AppError> {
        let id = uuid::Uuid::new_v4();

        let mut tx = pool.begin().await?;

        /* 1. Insertar resource envelope */
        ResourceRepository::create(
            &mut *tx,
            CreateResourceParams {
                id,
                kind: ResourceKind::Project,
                title: &req.title,
                editorial: EditorialState::Draft,
                visibility: VisibilityState::Private,
            },
        )
        .await?;

        /* 2. Insertar proyecto */
        let project = ProjectRepository::create(
            &mut *tx,
            id,
            &req.title,
            &req.description,
            req.url.as_deref(),
            req.sort_order,
        )
        .await?;

        tx.commit().await?;
        Ok(project)
    }

    pub async fn list_all(pool: &PgPool) -> Result<Vec<Project>, AppError> {
        Ok(ProjectRepository::list_all(pool).await?)
    }

    pub async fn list_visible(pool: &PgPool) -> Result<Vec<Project>, AppError> {
        Ok(ProjectRepository::list_visible(pool).await?)
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        req: UpdateProjectRequest,
    ) -> Result<Project, AppError> {
        let (url, clear_url) = match &req.url {
            ProjectUrlUpdate::Unchanged => (None, false),
            ProjectUrlUpdate::Clear => (None, true),
            ProjectUrlUpdate::Set(value) => (Some(value.as_str()), false),
        };

        ProjectRepository::update(
            pool,
            ProjectUpdateParams {
                id,
                title: req.title.as_deref(),
                description: req.description.as_deref(),
                url,
                clear_url,
                sort_order: req.sort_order,
                is_visible: req.is_visible,
            },
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
