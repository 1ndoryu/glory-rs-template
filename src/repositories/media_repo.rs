use sqlx::PgPool;
use uuid::Uuid;

use crate::models::media::Media;

pub struct MediaRepository;

impl MediaRepository {
    pub async fn create(
        pool: &PgPool,
        article_id: Option<Uuid>,
        file_path: &str,
        file_type: &str,
        file_size: i64,
        alt_text: &str,
    ) -> Result<Media, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Media>(
            "INSERT INTO media (id, article_id, file_path, file_type, file_size, alt_text) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING id, article_id, file_path, file_type, file_size, alt_text, created_at, asset_state",
        )
        .bind(id)
        .bind(article_id)
        .bind(file_path)
        .bind(file_type)
        .bind(file_size)
        .bind(alt_text)
        .fetch_one(pool)
        .await
    }

    pub async fn list(
        pool: &PgPool,
        file_type: Option<&str>,
        article_id: Option<Uuid>,
    ) -> Result<Vec<Media>, sqlx::Error> {
        let cols = "id, article_id, file_path, file_type, file_size, alt_text, created_at, asset_state";
        match (file_type, article_id) {
            (Some(ft), Some(aid)) => {
                sqlx::query_as::<_, Media>(&format!(
                    "SELECT {cols} FROM media WHERE file_type = $1 AND article_id = $2 ORDER BY created_at DESC"
                ))
                .bind(ft)
                .bind(aid)
                .fetch_all(pool)
                .await
            }
            (Some(ft), None) => {
                sqlx::query_as::<_, Media>(&format!(
                    "SELECT {cols} FROM media WHERE file_type = $1 ORDER BY created_at DESC"
                ))
                .bind(ft)
                .fetch_all(pool)
                .await
            }
            (None, Some(aid)) => {
                sqlx::query_as::<_, Media>(&format!(
                    "SELECT {cols} FROM media WHERE article_id = $1 ORDER BY created_at DESC"
                ))
                .bind(aid)
                .fetch_all(pool)
                .await
            }
            (None, None) => {
                sqlx::query_as::<_, Media>(&format!(
                    "SELECT {cols} FROM media ORDER BY created_at DESC"
                ))
                .fetch_all(pool)
                .await
            }
        }
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM media WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
