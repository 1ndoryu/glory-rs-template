use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Proyecto del portfolio
#[derive(Debug, Clone, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub sort_order: i32,
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
}

/// [018A-48] Contrato administrativo completo. Los metadatos de orden y
/// visibilidad solo se devuelven tras autorización de administrador.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProjectAdminResponse {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub sort_order: i32,
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
}

/// [018A-48] Contrato público mínimo. El repository ya filtra y ordena los
/// proyectos visibles; no se filtran al visitante detalles de presentación.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProjectPublicResponse {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&Project> for ProjectAdminResponse {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id,
            title: project.title.clone(),
            description: project.description.clone(),
            url: project.url.clone(),
            sort_order: project.sort_order,
            is_visible: project.is_visible,
            created_at: project.created_at,
        }
    }
}

impl From<&Project> for ProjectPublicResponse {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id,
            title: project.title.clone(),
            description: project.description.clone(),
            url: project.url.clone(),
            created_at: project.created_at,
        }
    }
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
    use super::{
        Project, ProjectAdminResponse, ProjectPublicResponse, ProjectUrlUpdate,
        UpdateProjectRequest,
    };
    use chrono::Utc;
    use uuid::Uuid;

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

    #[test]
    fn contrato_publico_no_expone_orden_ni_visibilidad() {
        let project = Project {
            id: Uuid::new_v4(),
            title: "Proyecto".into(),
            description: "Descripción".into(),
            url: Some("https://example.com".into()),
            sort_order: 7,
            is_visible: true,
            created_at: Utc::now(),
        };

        let public = serde_json::to_value(ProjectPublicResponse::from(&project)).unwrap();
        assert!(public.get("sort_order").is_none());
        assert!(public.get("is_visible").is_none());

        let admin = serde_json::to_value(ProjectAdminResponse::from(&project)).unwrap();
        assert_eq!(
            admin.get("sort_order").and_then(|value| value.as_i64()),
            Some(7)
        );
        assert_eq!(
            admin.get("is_visible").and_then(|value| value.as_bool()),
            Some(true)
        );
    }
}
