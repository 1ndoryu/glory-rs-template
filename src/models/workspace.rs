use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Release inmutable del layout del escritorio.
/// [297A-11 §9.2] Cada release es versionado e inmutable.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct WorkspaceRelease {
    pub id: Uuid,
    pub version: i32,
    pub tree: JsonValue,
    pub published_at: DateTime<Utc>,
    pub published_by: Option<Uuid>,
}

/// Request para publicar un nuevo release.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishReleaseRequest {
    /// Árbol del workspace a publicar (formato JSON del WorkspaceTree frontend).
    pub tree: JsonValue,
}

/// Response pública del release activo.
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceReleasePublic {
    pub version: i32,
    pub tree: JsonValue,
    pub published_at: DateTime<Utc>,
}

impl From<WorkspaceRelease> for WorkspaceReleasePublic {
    fn from(r: WorkspaceRelease) -> Self {
        Self {
            version: r.version,
            tree: r.tree,
            published_at: r.published_at,
        }
    }
}
