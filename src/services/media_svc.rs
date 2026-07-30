use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::media::{CreateMediaRequest, Media};
use crate::repositories::media_repo::MediaRepository;

pub struct MediaService;

impl MediaService {
    pub async fn create(pool: &PgPool, req: CreateMediaRequest) -> Result<Media, AppError> {
        let media = MediaRepository::create(
            pool,
            req.article_id,
            &req.file_path,
            &req.file_type,
            req.file_size,
            &req.alt_text,
        )
        .await?;
        Ok(media)
    }

    pub async fn list(
        pool: &PgPool,
        file_type: Option<&str>,
        article_id: Option<Uuid>,
    ) -> Result<Vec<Media>, AppError> {
        let media = MediaRepository::list(pool, file_type, article_id).await?;
        Ok(media)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !MediaRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Media no encontrado".into()));
        }
        Ok(())
    }
}
