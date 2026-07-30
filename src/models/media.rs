use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Archivo multimedia (imagen, audio, video)
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Media {
    pub id: Uuid,
    pub article_id: Option<Uuid>,
    pub file_path: String,
    pub file_type: String,
    pub file_size: i64,
    pub alt_text: String,
    pub created_at: DateTime<Utc>,
}

/// Request para registrar un archivo media
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMediaRequest {
    pub article_id: Option<Uuid>,
    pub file_path: String,
    pub file_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub alt_text: String,
}

/// Query params para filtrar media
#[derive(Debug, Deserialize)]
pub struct MediaQueryParams {
    pub file_type: Option<String>,
    pub article_id: Option<Uuid>,
}
