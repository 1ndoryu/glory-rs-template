use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Estado de procesamiento de un asset multimedia
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(sqlx::Type)]
#[sqlx(type_name = "asset_processing_state", rename_all = "lowercase")]
pub enum AssetProcessingState {
    Processing,
    Clean,
    Rejected,
}

/// Archivo multimedia (imagen, audio, video). [297A-10] Añade `asset_state`.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Media {
    pub id: Uuid,
    pub article_id: Option<Uuid>,
    pub file_path: String,
    pub file_type: String,
    pub file_size: i64,
    pub alt_text: String,
    pub created_at: DateTime<Utc>,
    /// [297A-10] Estado de procesamiento del asset.
    pub asset_state: AssetProcessingState,
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
