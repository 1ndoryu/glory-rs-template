use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Fila interna del snapshot publicado. No se expone directamente desde HTTP.
#[derive(Debug, Clone, FromRow)]
pub struct GameMapVersionRow {
    pub id: Uuid,
    pub map_id: String,
    pub version: i32,
    pub schema_version: i32,
    pub content_hash: String,
    pub document: JsonValue,
    pub document_bytes: i64,
    pub published_at: DateTime<Utc>,
    pub published_by: Option<Uuid>,
    pub is_active: bool,
}

pub struct GameMapRepository;

impl GameMapRepository {
    /// Recupera solo el snapshot activo; drafts e históricos quedan fuera del
    /// contrato público aunque alguien conozca su UUID o versión.
    pub async fn get_active(
        pool: &PgPool,
        map_id: &str,
    ) -> Result<Option<GameMapVersionRow>, sqlx::Error> {
        sqlx::query_as::<_, GameMapVersionRow>(
            "SELECT id, map_id, version, schema_version, content_hash, document,
                    octet_length(document::text)::BIGINT AS document_bytes,
                    published_at, published_by, is_active
             FROM game_map_versions
             WHERE map_id = $1
               AND is_active = TRUE
             LIMIT 1",
        )
        .bind(map_id)
        .fetch_optional(pool)
        .await
    }
}
