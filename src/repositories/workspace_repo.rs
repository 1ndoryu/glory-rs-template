use sqlx::PgPool;
use uuid::Uuid;

use crate::models::workspace::WorkspaceRelease;

pub struct WorkspaceRepository;

impl WorkspaceRepository {
    /// Listar todos los releases ordenados de más nuevo a más viejo (admin).
    pub async fn list_releases(pool: &PgPool) -> Result<Vec<WorkspaceRelease>, sqlx::Error> {
        sqlx::query_as::<_, WorkspaceRelease>(
            "SELECT id, version, tree, published_at, published_by, summary, diff_from \
             FROM workspace_releases \
             ORDER BY version DESC",
        )
        .fetch_all(pool)
        .await
    }

    /// Obtener el release más reciente (el activo).
    pub async fn get_latest(pool: &PgPool) -> Result<Option<WorkspaceRelease>, sqlx::Error> {
        sqlx::query_as::<_, WorkspaceRelease>(
            "SELECT id, version, tree, published_at, published_by, summary, diff_from \
             FROM workspace_releases \
             ORDER BY version DESC \
             LIMIT 1",
        )
        .fetch_optional(pool)
        .await
    }

    /// Obtener un release por versión específica.
    pub async fn get_by_version(
        pool: &PgPool,
        version: i32,
    ) -> Result<Option<WorkspaceRelease>, sqlx::Error> {
        sqlx::query_as::<_, WorkspaceRelease>(
            "SELECT id, version, tree, published_at, published_by, summary, diff_from \
             FROM workspace_releases \
             WHERE version = $1",
        )
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// Obtener la versión más alta actual (dentro de transacción).
    #[allow(clippy::explicit_auto_deref)]
    pub async fn get_max_version(tx: &mut sqlx::PgConnection) -> Result<i32, sqlx::Error> {
        let row: (i32,) =
            sqlx::query_as("SELECT COALESCE(MAX(version), 0) FROM workspace_releases")
                .fetch_one(&mut *tx)
                .await?;
        Ok(row.0)
    }

    /// Publicar un nuevo release (dentro de transacción).
    /// [028A-11] `summary` es el diff auditable contra la release anterior y
    /// `diff_from` su versión (NULL para la primera release).
    #[allow(clippy::explicit_auto_deref)]
    pub async fn create(
        tx: &mut sqlx::PgConnection,
        version: i32,
        tree: &serde_json::Value,
        published_by: Option<Uuid>,
        summary: &serde_json::Value,
        diff_from: Option<i32>,
    ) -> Result<WorkspaceRelease, sqlx::Error> {
        sqlx::query_as::<_, WorkspaceRelease>(
            "INSERT INTO workspace_releases (version, tree, published_by, summary, diff_from) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, version, tree, published_at, published_by, summary, diff_from",
        )
        .bind(version)
        .bind(tree)
        .bind(published_by)
        .bind(summary)
        .bind(diff_from)
        .fetch_one(&mut *tx)
        .await
    }
}
