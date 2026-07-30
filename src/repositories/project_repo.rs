use sqlx::PgPool;
use uuid::Uuid;

use crate::models::project::Project;

pub struct ProjectRepository;

impl ProjectRepository {
    /// [297A-10] Crear proyecto dentro de una transacción.
    pub async fn create(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        title: &str,
        description: &str,
        url: Option<&str>,
        sort_order: i32,
    ) -> Result<Project, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            "INSERT INTO projects (id, title, description, url, sort_order) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, title, description, url, sort_order, is_visible, created_at",
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .bind(url)
        .bind(sort_order)
        .fetch_one(&mut *conn)
        .await
    }

    pub async fn list_all(pool: &PgPool) -> Result<Vec<Project>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            "SELECT id, title, description, url, sort_order, is_visible, created_at \
             FROM projects ORDER BY sort_order ASC, created_at DESC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_visible(pool: &PgPool) -> Result<Vec<Project>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            "SELECT id, title, description, url, sort_order, is_visible, created_at \
             FROM projects WHERE is_visible = true ORDER BY sort_order ASC, created_at DESC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        url: Option<&str>,
        sort_order: Option<i32>,
        is_visible: Option<bool>,
    ) -> Result<Option<Project>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            "UPDATE projects SET \
                title = COALESCE($1, title), \
                description = COALESCE($2, description), \
                url = COALESCE($3, url), \
                sort_order = COALESCE($4, sort_order), \
                is_visible = COALESCE($5, is_visible) \
             WHERE id = $6 \
             RETURNING id, title, description, url, sort_order, is_visible, created_at",
        )
        .bind(title)
        .bind(description)
        .bind(url)
        .bind(sort_order)
        .bind(is_visible)
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
