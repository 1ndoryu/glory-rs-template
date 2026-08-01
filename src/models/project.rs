use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Proyecto del portfolio
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub sort_order: i32,
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub url: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    /// Los proyectos nuevos nacen privados/ocultos salvo publicación explícita.
    #[serde(default)]
    pub is_visible: bool,
}

/// Parche explícito de URL: distingue omitir, limpiar y reemplazar.
#[derive(Debug, Default, ToSchema, PartialEq, Eq)]
pub enum ProjectUrlUpdate {
    #[default]
    Unchanged,
    Clear,
    Set(String),
}

impl<'de> Deserialize<'de> for ProjectUrlUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<String>::deserialize(deserializer)? {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProjectRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    /// Ausente conserva la URL; null la limpia; una cadena la reemplaza.
    #[serde(default)]
    pub url: ProjectUrlUpdate,
    pub sort_order: Option<i32>,
    pub is_visible: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::{ProjectUrlUpdate, UpdateProjectRequest};

    #[test]
    fn distingue_url_omitida_nula_y_con_valor() {
        let omitted: UpdateProjectRequest = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(omitted.url, ProjectUrlUpdate::Unchanged);

        let cleared: UpdateProjectRequest = serde_json::from_str(r#"{"url":null}"#).unwrap();
        assert_eq!(cleared.url, ProjectUrlUpdate::Clear);

        let replaced: UpdateProjectRequest =
            serde_json::from_str(r#"{"url":"https://example.com"}"#).unwrap();
        assert_eq!(
            replaced.url,
            ProjectUrlUpdate::Set("https://example.com".to_string())
        );
    }
}
