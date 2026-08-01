use sqlx::PgPool;
use uuid::Uuid;

use crate::models::resource::{
    CreateResourceParams, EditorialState, Resource, ResourceKind, VisibilityState,
};

pub struct ResourceRepository;

impl ResourceRepository {
    /// Insertar un recurso envelope (llamar dentro de transacción).
    #[allow(clippy::explicit_auto_deref)]
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

    /// Sincronizar título y visibilidad del envelope de un recurso dentro de su transacción.
    /// El estado editorial se modifica únicamente mediante publicación explícita.
    pub async fn update_resource_metadata(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        kind: ResourceKind,
        title: Option<&str>,
        is_visible: Option<bool>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE resources SET \
                title = COALESCE($1, title), \
                visibility = CASE \
                    WHEN $2 IS NULL THEN visibility \
                    WHEN $2 THEN 'public'::visibility_state \
                    ELSE 'private'::visibility_state \
                END, \
                updated_at = NOW() \
             WHERE id = $3 AND kind = $4",
        )
        .bind(title)
        .bind(is_visible)
        .bind(id)
        .bind(kind)
        .execute(&mut *conn)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Soft delete dentro de una transacción: conserva el envelope para restauración.
    pub async fn soft_delete_kind_tx(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        kind: ResourceKind,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE resources SET lifecycle = 'trashed', deleted_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND kind = $2 AND lifecycle = 'active'",
        )
        .bind(id)
        .bind(kind)
        .execute(&mut *conn)
        .await?;
        Ok(result.rows_affected() > 0)
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

    /// [297A-14 F4] Restaurar de trashed a active verificando el kind.
    /// Evita restaurar un recurso de otro tipo mediante el endpoint de media.
    pub async fn restore_kind(
        pool: &PgPool,
        id: Uuid,
        kind: ResourceKind,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE resources SET lifecycle = 'active', deleted_at = NULL, updated_at = NOW() \
             WHERE id = $1 AND kind = $2 AND lifecycle = 'trashed'",
        )
        .bind(id)
        .bind(kind)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
