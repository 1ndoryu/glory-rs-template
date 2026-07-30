use sqlx::PgPool;
use uuid::Uuid;

use crate::models::resource::{CreateResourceParams, EditorialState, Resource, VisibilityState};

pub struct ResourceRepository;

impl ResourceRepository {
    /// Insertar un recurso envelope (llamar dentro de transacción).
    pub async fn create(
        tx: &mut sqlx::PgConnection,
        params: CreateResourceParams<'_>,
    ) -> Result<Resource, sqlx::Error> {
        sqlx::query_as::<_, Resource>(
            "INSERT INTO resources (id, kind, title, editorial, visibility, lifecycle) \
             VALUES ($1, $2, $3, $4, $5, 'active') \
             RETURNING id, kind, title, editorial, visibility, lifecycle, deleted_at, created_at, updated_at",
        )
        .bind(params.id)
        .bind(params.kind)
        .bind(params.title)
        .bind(params.editorial)
        .bind(params.visibility)
        .fetch_one(&mut *tx)
        .await
    }

    /// Buscar recurso por ID.
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Resource>, sqlx::Error> {
        sqlx::query_as::<_, Resource>(
            "SELECT id, kind, title, editorial, visibility, lifecycle, deleted_at, created_at, updated_at \
             FROM resources WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Actualizar editorial/visibility de un recurso.
    pub async fn update_state(
        pool: &PgPool,
        id: Uuid,
        editorial: Option<EditorialState>,
        visibility: Option<VisibilityState>,
    ) -> Result<Option<Resource>, sqlx::Error> {
        sqlx::query_as::<_, Resource>(
            "UPDATE resources SET \
                editorial = COALESCE($1, editorial), \
                visibility = COALESCE($2, visibility), \
                updated_at = NOW() \
             WHERE id = $3 \
             RETURNING id, kind, title, editorial, visibility, lifecycle, deleted_at, created_at, updated_at",
        )
        .bind(editorial)
        .bind(visibility)
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Soft delete: mover a trashed.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE resources SET lifecycle = 'trashed', deleted_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND lifecycle = 'active'",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Restaurar de trashed a active.
    pub async fn restore(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE resources SET lifecycle = 'active', deleted_at = NULL, updated_at = NOW() \
             WHERE id = $1 AND lifecycle = 'trashed'",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
