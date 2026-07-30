use sqlx::PgPool;
use uuid::Uuid;

use crate::models::article::Article;

pub struct ArticleRepository;

impl ArticleRepository {
    pub async fn create(
        pool: &PgPool,
        title: &str,
        slug: &str,
        content: &serde_json::Value,
        excerpt: &str,
        cover_image: Option<&str>,
        status: &str,
        is_pinned: bool,
    ) -> Result<Article, sqlx::Error> {
        let id = Uuid::new_v4();
        let published_at = if status == "published" {
            Some(chrono::Utc::now())
        } else {
            None
        };

        sqlx::query_as::<_, Article>(
            "INSERT INTO articles (id, title, slug, content, excerpt, cover_image, status, is_pinned, published_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING id, title, slug, content, excerpt, cover_image, status, is_pinned, published_at, created_at, updated_at",
        )
        .bind(id)
        .bind(title)
        .bind(slug)
        .bind(content)
        .bind(excerpt)
        .bind(cover_image)
        .bind(status)
        .bind(is_pinned)
        .bind(published_at)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            "SELECT id, title, slug, content, excerpt, cover_image, status, is_pinned, published_at, created_at, updated_at \
             FROM articles WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            "SELECT id, title, slug, content, excerpt, cover_image, status, is_pinned, published_at, created_at, updated_at \
             FROM articles WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn list(
        pool: &PgPool,
        status: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Article>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;

        let articles = if let Some(status_filter) = status {
            sqlx::query_as::<_, Article>(
                "SELECT id, title, slug, content, excerpt, cover_image, status, is_pinned, published_at, created_at, updated_at \
                 FROM articles WHERE status = $1 \
                 ORDER BY is_pinned DESC, COALESCE(published_at, created_at) DESC \
                 LIMIT $2 OFFSET $3",
            )
            .bind(status_filter)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, Article>(
                "SELECT id, title, slug, content, excerpt, cover_image, status, is_pinned, published_at, created_at, updated_at \
                 FROM articles \
                 ORDER BY is_pinned DESC, COALESCE(published_at, created_at) DESC \
                 LIMIT $1 OFFSET $2",
            )
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await?
        };

        let (total,): (i64,) = if let Some(status_filter) = status {
            sqlx::query_as("SELECT COUNT(*) FROM articles WHERE status = $1")
                .bind(status_filter)
                .fetch_one(pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM articles")
                .fetch_one(pool)
                .await?
        };

        Ok((articles, total))
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        title: Option<&str>,
        content: Option<&serde_json::Value>,
        excerpt: Option<&str>,
        cover_image: Option<&str>,
        status: Option<&str>,
        is_pinned: Option<bool>,
    ) -> Result<Option<Article>, sqlx::Error> {
        /* Si se cambia a published y no tenia published_at, setearlo */
        let current = Self::find_by_id(pool, id).await?;
        let current = match current {
            Some(a) => a,
            None => return Ok(None),
        };

        let new_status = status.unwrap_or(&current.status);
        let published_at = if new_status == "published" && current.published_at.is_none() {
            Some(chrono::Utc::now())
        } else {
            current.published_at
        };

        sqlx::query_as::<_, Article>(
            "UPDATE articles SET \
                title = COALESCE($1, title), \
                content = COALESCE($2, content), \
                excerpt = COALESCE($3, excerpt), \
                cover_image = COALESCE($4, cover_image), \
                status = COALESCE($5, status), \
                is_pinned = COALESCE($6, is_pinned), \
                published_at = $7, \
                updated_at = NOW() \
             WHERE id = $8 \
             RETURNING id, title, slug, content, excerpt, cover_image, status, is_pinned, published_at, created_at, updated_at",
        )
        .bind(title)
        .bind(content)
        .bind(excerpt)
        .bind(cover_image)
        .bind(status)
        .bind(is_pinned)
        .bind(published_at)
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM articles WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn slug_exists(pool: &PgPool, slug: &str) -> Result<bool, sqlx::Error> {
        let (exists,): (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM articles WHERE slug = $1)")
                .bind(slug)
                .fetch_one(pool)
                .await?;
        Ok(exists)
    }
}
