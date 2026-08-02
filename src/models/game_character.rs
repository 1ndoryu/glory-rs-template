use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

pub const GAME_CHARACTER_ID_MAX_CHARS: usize = 32;

/// Opción visual publicada por el catálogo; no incluye storage keys ni scripts.
#[derive(Debug, Clone, FromRow)]
pub struct GameCharacterDefinition {
    pub id: String,
    pub display_name: String,
    pub body_tone: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameCharacterPublicResponse {
    pub id: String,
    pub display_name: String,
    pub body_tone: String,
}

impl From<GameCharacterDefinition> for GameCharacterPublicResponse {
    fn from(character: GameCharacterDefinition) -> Self {
        Self {
            id: character.id,
            display_name: character.display_name,
            body_tone: character.body_tone,
        }
    }
}

impl GameCharacterDefinition {
    /// El ID de una opción del catálogo solo contiene minúsculas ASCII, dígitos y guiones.
    #[must_use]
    pub fn is_valid_id(id: &str) -> bool {
        !id.is_empty()
            && id.chars().count() <= GAME_CHARACTER_ID_MAX_CHARS
            && id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }
}
