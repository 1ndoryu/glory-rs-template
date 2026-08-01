use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::project::{
    CreateProjectRequest, Project, ProjectUrlUpdate, UpdateProjectRequest,
};
use crate::models::resource::{
    CreateResourceParams, EditorialState, ResourceKind, VisibilityState,
};
use crate::repositories::project_repo::{
    ProjectCreateParams, ProjectRepository, ProjectUpdateParams,
};
use crate::repositories::resource_repo::ResourceRepository;

pub struct ProjectService;

impl ProjectService {
    /// Crear proyecto y resource envelope en una transacción.
    pub async fn create(pool: &PgPool, req: CreateProjectRequest) -> Result<Project, AppError> {
        let id = Uuid::new_v4();
        /* Editorial y visibilidad son estados independientes. La publicación
         * editorial tendrá su propio comando; is_visible solo proyecta acceso. */
        let editorial = EditorialState::Draft;
        let visibility = if req.is_visible {
            VisibilityState::Public
        } else {
            VisibilityState::Private
        };

        let mut tx = pool.begin().await?;

        ResourceRepository::create(
            &mut tx,
            CreateResourceParams {
                id,
                kind: ResourceKind::Project,
                title: &req.title,
                editorial,
                visibility,
            },
        )
        .await?;

        let project = ProjectRepository::create(
            &mut tx,
            ProjectCreateParams {
                id,
                title: &req.title,
                description: &req.description,
                url: req.url.as_deref(),
                sort_order: req.sort_order,
                is_visible: req.is_visible,
            },
        )
        .await?;

        tx.commit().await?;
        Ok(project)
    }

    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Project, AppError> {
        ProjectRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Proyecto no encontrado".into()))
    }

    pub async fn list_all(pool: &PgPool) -> Result<Vec<Project>, AppError> {
        Ok(ProjectRepository::list_all(pool).await?)
    }

    pub async fn list_visible(pool: &PgPool) -> Result<Vec<Project>, AppError> {
        Ok(ProjectRepository::list_visible(pool).await?)
    }

    /// Actualiza proyecto y envelope juntos para evitar estados divergentes.
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

        let mut tx = pool.begin().await?;
        let project = ProjectRepository::update(
            &mut tx,
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
        .ok_or_else(|| AppError::NotFound("Proyecto no encontrado".into()))?;

        let envelope_updated = ResourceRepository::update_project_state(
            &mut tx,
            id,
            req.title.as_deref(),
            req.is_visible,
        )
        .await?;
        if !envelope_updated {
            return Err(AppError::NotFound(
                "Envelope de proyecto no encontrado".into(),
            ));
        }

        tx.commit().await?;
        Ok(project)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        let mut tx = pool.begin().await?;
        let trashed = ResourceRepository::soft_delete_tx(&mut tx, id).await?;
        if !trashed {
            return Err(AppError::NotFound("Proyecto no encontrado".into()));
        }
        tx.commit().await?;
        Ok(())
    }
}
