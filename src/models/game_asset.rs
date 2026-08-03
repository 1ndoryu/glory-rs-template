use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

pub const GAME_ASSET_ID_MAX_CHARS: usize = 48;
pub const GAME_ASSET_DISPLAY_NAME_MAX_CHARS: usize = 64;

/// Categorías del catálogo, alineadas con `AssetCategory` del contrato de mapa
/// (`terrain`, `tree`, `rock`, `water`, `character`, `generic`).
pub const GAME_ASSET_CATEGORIES: [&str; 6] =
    ["terrain", "tree", "rock", "water", "character", "generic"];

/// Asset del catálogo; no incluye storage keys ni scripts. `category`,
/// `is_active` y `created_at` son metadata administrativa interna.
#[derive(Debug, Clone, FromRow)]
pub struct GameAssetDefinition {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Contrato público: solo lo necesario para colocar el asset en el editor.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameAssetPublicResponse {
    pub id: String,
    pub display_name: String,
    pub category: String,
}

impl From<GameAssetDefinition> for GameAssetPublicResponse {
    fn from(asset: GameAssetDefinition) -> Self {
        Self {
            id: asset.id,
            display_name: asset.display_name,
            category: asset.category,
        }
    }
}

/// Contrato administrativo: incluye el estado y la fecha para gestionar el catálogo.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameAssetAdminResponse {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<GameAssetDefinition> for GameAssetAdminResponse {
    fn from(asset: GameAssetDefinition) -> Self {
        Self {
            id: asset.id,
            display_name: asset.display_name,
            category: asset.category,
            is_active: asset.is_active,
            created_at: asset.created_at,
        }
    }
}

/// Alta de un nuevo asset allowlisted del catálogo (admin).
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateGameAssetRequest {
    pub id: String,
    pub display_name: String,
    pub category: String,
}

/// Actualización completa de un asset del catálogo (admin).
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateGameAssetRequest {
    pub display_name: String,
    pub category: String,
    pub is_active: bool,
}

impl GameAssetDefinition {
    /// El ID de un asset solo contiene minúsculas ASCII, dígitos y guiones.
    #[must_use]
    pub fn is_valid_id(id: &str) -> bool {
        !id.is_empty()
            && id.chars().count() <= GAME_ASSET_ID_MAX_CHARS
            && id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }

    /// Etiqueta visible de un asset: sin controles, entre 1 y 64 caracteres.
    pub fn validate_display_name(value: &str) -> Result<String, &'static str> {
        let trimmed = value.trim();
        let count = trimmed.chars().count();
        if trimmed.is_empty() || count > GAME_ASSET_DISPLAY_NAME_MAX_CHARS {
            return Err("La etiqueta debe tener entre 1 y 64 caracteres");
        }
        if trimmed.chars().any(char::is_control) {
            return Err("La etiqueta contiene caracteres no permitidos");
        }
        Ok(trimmed.to_string())
    }

    /// Categoría permitida por el contrato del mapa (allowlisted).
    #[must_use]
    pub fn is_valid_category(value: &str) -> bool {
        GAME_ASSET_CATEGORIES.contains(&value)
    }
}
