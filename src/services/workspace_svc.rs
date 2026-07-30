use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::workspace::{WorkspaceRelease, WorkspaceReleasePublic};
use crate::repositories::workspace_repo::WorkspaceRepository;

pub struct WorkspaceService;

impl WorkspaceService {
    /// Obtener el release activo (público).
    pub async fn get_active_release(pool: &PgPool) -> Result<WorkspaceReleasePublic, AppError> {
        WorkspaceRepository::get_latest(pool)
            .await?
            .map(WorkspaceReleasePublic::from)
            .ok_or_else(|| AppError::NotFound("No hay releases publicados".into()))
    }

    /// Obtener un release por versión (público).
    pub async fn get_release_by_version(
        pool: &PgPool,
        version: i32,
    ) -> Result<WorkspaceReleasePublic, AppError> {
        WorkspaceRepository::get_by_version(pool, version)
            .await?
            .map(WorkspaceReleasePublic::from)
            .ok_or_else(|| AppError::NotFound(format!("Release v{version} no encontrado")))
    }

    /// Listar todos los releases (admin — incluye historial).
    pub async fn list_releases(pool: &PgPool) -> Result<Vec<WorkspaceRelease>, AppError> {
        let releases = sqlx::query_as::<_, WorkspaceRelease>(
            "SELECT id, version, tree, published_at, published_by \
             FROM workspace_releases \
             ORDER BY version DESC",
        )
        .fetch_all(pool)
        .await?;
        Ok(releases)
    }

    /// Publicar un nuevo release (admin).
    /// [297A-11 §9.2] Publicación transaccional a release inmutable.
    pub async fn publish(
        pool: &PgPool,
        tree: serde_json::Value,
        published_by: Uuid,
    ) -> Result<WorkspaceRelease, AppError> {
        let mut tx = pool.begin().await?;

        /* Obtener siguiente versión */
        let max_version = WorkspaceRepository::get_max_version(&mut *tx).await?;
        let next_version = max_version + 1;

        /* Crear release inmutable */
        let release = WorkspaceRepository::create(&mut *tx, next_version, &tree, Some(published_by)).await?;

        tx.commit().await?;
        Ok(release)
    }
}
