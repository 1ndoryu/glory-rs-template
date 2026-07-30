use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::article::{
    Article, CreateArticleRequest, PaginatedArticles, UpdateArticleRequest,
};
use crate::repositories::article::{CreateArticleParams, UpdateArticleParams};
use crate::repositories::ArticleRepository;

pub struct ArticleService;

impl ArticleService {
    pub async fn create(pool: &PgPool, req: CreateArticleRequest) -> Result<Article, AppError> {
        let slug = Self::generate_slug(pool, &req.title).await?;

        let article = ArticleRepository::create(
            pool,
            CreateArticleParams {
                title: &req.title,
                slug: &slug,
                content: &req.content,
                excerpt: &req.excerpt,
                cover_image: req.cover_image.as_deref(),
                status: &req.status,
                is_pinned: req.is_pinned,
            },
        )
        .await?;

        Ok(article)
    }

    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Article, AppError> {
        ArticleRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Articulo no encontrado".into()))
    }

    pub async fn get_by_slug(pool: &PgPool, slug: &str) -> Result<Article, AppError> {
        ArticleRepository::find_by_slug(pool, slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Articulo no encontrado".into()))
    }

    pub async fn list(
        pool: &PgPool,
        status: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<PaginatedArticles, AppError> {
        let per_page = per_page.clamp(1, 100);
        let page = page.max(1);
        let (articles, total) = ArticleRepository::list(pool, status, page, per_page).await?;

        Ok(PaginatedArticles {
            items: articles,
            total,
            page,
            per_page,
        })
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        req: UpdateArticleRequest,
    ) -> Result<Article, AppError> {
        ArticleRepository::update(
            pool,
            id,
            UpdateArticleParams {
                title: req.title.as_deref(),
                content: req.content.as_ref(),
                excerpt: req.excerpt.as_deref(),
                cover_image: req.cover_image.as_deref(),
                status: req.status.as_deref(),
                is_pinned: req.is_pinned,
            },
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Articulo no encontrado".into()))
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !ArticleRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Articulo no encontrado".into()));
        }
        Ok(())
    }

    /// Genera un slug URL-safe unico a partir del titulo
    async fn generate_slug(pool: &PgPool, title: &str) -> Result<String, AppError> {
        let base = title
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        let mut slug = base.clone();
        let mut counter = 1;

        while ArticleRepository::slug_exists(pool, &slug).await? {
            slug = format!("{base}-{counter}");
            counter += 1;
        }

        Ok(slug)
    }
}
