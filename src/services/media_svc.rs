use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::media::{CreateMediaRequest, Media};
use crate::models::resource::{CreateResourceParams, EditorialState, ResourceKind, VisibilityState};
use crate::repositories::media_repo::MediaRepository;
use crate::repositories::resource_repo::ResourceRepository;

pub struct MediaService;

impl MediaService {
    /// [297A-10] Crear media con resource envelope en transacción. Defaults: ready, public.
    pub async fn create(pool: &PgPool, req: CreateMediaRequest) -> Result<Media, AppError> {
        let id = uuid::Uuid::new_v4();

        let mut tx = pool.begin().await?;

        /* 1. Insertar resource envelope (media es ready/public por defecto) */
        ResourceRepository::create(
            &mut *tx,
            CreateResourceParams {
                id,
                kind: ResourceKind::Media,
                title: if req.alt_text.is_empty() { "media file" } else { &req.alt_text },
                editorial: EditorialState::Ready,
                visibility: VisibilityState::Public,
            },
        )
        .await?;

        /* 2. Insertar media */
        let media = MediaRepository::create(
            &mut *tx,
            id,
            req.article_id,
            &req.file_path,
            &req.file_type,
            req.file_size,
            &req.alt_text,
        )
        .await?;

        tx.commit().await?;
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
