use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
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

    /// Publica una versión dentro de la transacción ya abierta por el servicio
    /// (serializada por mapa mediante advisory lock). `None` significa que
    /// `expected_version` ya no es la versión activa.
    /// [297A-58] La transacción la abre el servicio para que el evento de
    /// auditoría de la publicación se escriba (o se descarte) con el cambio.
    pub async fn publish(
        tx: &mut Transaction<'_, Postgres>,
        map_id: &str,
        expected_version: i32,
        schema_version: i32,
        content_hash: &str,
        document: &JsonValue,
        published_by: Uuid,
    ) -> Result<Option<GameMapVersionRow>, sqlx::Error> {
        /* El lock transaccional evita carreras tanto en la primera publicación
         * como en actualizaciones posteriores, sin mantener locks permanentes. */
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(map_id)
            .execute(&mut **tx)
            .await?;

        let active_version: Option<i32> = sqlx::query_scalar(
            "SELECT version
             FROM game_map_versions
             WHERE map_id = $1 AND is_active = TRUE
             LIMIT 1",
        )
        .bind(map_id)
        .fetch_optional(&mut **tx)
        .await?;

        if active_version.unwrap_or(0) != expected_version {
            return Ok(None);
        }

        let next_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1
             FROM game_map_versions
             WHERE map_id = $1",
        )
        .bind(map_id)
        .fetch_one(&mut **tx)
        .await?;

        sqlx::query(
            "UPDATE game_map_versions
             SET is_active = FALSE
             WHERE map_id = $1 AND is_active = TRUE",
        )
        .bind(map_id)
        .execute(&mut **tx)
        .await?;

        let row = sqlx::query_as::<_, GameMapVersionRow>(
            "INSERT INTO game_map_versions
                (map_id, version, schema_version, content_hash, document, published_by, is_active)
             VALUES ($1, $2, $3, $4, $5, $6, TRUE)
             RETURNING id, map_id, version, schema_version, content_hash, document,
                       octet_length(document::text)::BIGINT AS document_bytes,
                       published_at, published_by, is_active",
        )
        .bind(map_id)
        .bind(next_version)
        .bind(schema_version)
        .bind(content_hash)
        .bind(document)
        .bind(published_by)
        .fetch_one(&mut **tx)
        .await?;

        Ok(Some(row))
    }
}
