use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::game_map::{
    document_content_hash, document_json_bytes, GameMapVersionPublic, MapVersion,
    MAP_VERSION_MAX_JSON_BYTES,
};
use crate::repositories::game_map_repo::GameMapRepository;

pub struct GameMapService;

impl GameMapService {
    /// Obtiene el mapa activo y vuelve a validar el JSON antes de servirlo.
    /// Un snapshot corrupto nunca se convierte en una respuesta parcial.
    pub async fn get_active(pool: &PgPool, map_id: &str) -> Result<GameMapVersionPublic, AppError> {
        if map_id.trim().is_empty() || map_id.chars().count() > 128 {
            return Err(AppError::BadRequest(
                "Identificador de mapa no válido".into(),
            ));
        }

        let row = GameMapRepository::get_active(pool, map_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Mapa publicado no encontrado".into()))?;

        let document_size = usize::try_from(row.document_bytes).map_err(|_| {
            AppError::Internal("El tamaño del snapshot del mapa no es válido".into())
        })?;
        if document_size > MAP_VERSION_MAX_JSON_BYTES {
            return Err(AppError::Internal(
                "El snapshot del mapa supera el tamaño permitido".into(),
            ));
        }
        let document_bytes = document_json_bytes(&row.document)
            .ok_or_else(|| AppError::Internal("El snapshot del mapa no es serializable".into()))?;
        let document: MapVersion =
            MapVersion::from_bounded_json(&document_bytes, MAP_VERSION_MAX_JSON_BYTES).map_err(
                |_| AppError::Internal("El snapshot publicado del mapa no es válido".into()),
            )?;
        let computed_hash = document_content_hash(&row.document)
            .ok_or_else(|| AppError::Internal("No se pudo verificar el hash del mapa".into()))?;
        if computed_hash != row.content_hash {
            return Err(AppError::Internal(
                "La integridad del snapshot del mapa no se pudo verificar".into(),
            ));
        }

        let document_id = document.id.clone();
        if document_id != row.map_id || i32::from(document.schema_version) != row.schema_version {
            return Err(AppError::Internal(
                "La metadata del snapshot no coincide con su documento".into(),
            ));
        }

        Ok(GameMapVersionPublic {
            map_id: row.map_id,
            version: row.version,
            schema_version: row.schema_version,
            content_hash: row.content_hash,
            published_at: row.published_at,
            document: row.document,
        })
    }
}
