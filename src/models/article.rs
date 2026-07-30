use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/// Articulo del blog almacenado en base de datos
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Article {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: JsonValue,
    pub excerpt: String,
    pub cover_image: Option<String>,
    pub status: String,
    pub is_pinned: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request para crear un articulo
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateArticleRequest {
    #[validate(length(
        min = 1,
        max = 500,
        message = "El titulo debe tener entre 1 y 500 caracteres"
    ))]
    pub title: String,
    pub content: JsonValue,
    #[serde(default)]
    pub excerpt: String,
    pub cover_image: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub is_pinned: bool,
}

fn default_status() -> String {
    "draft".to_string()
}

/// Request para actualizar un articulo
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateArticleRequest {
    #[validate(length(min = 1, max = 500))]
    pub title: Option<String>,
    pub content: Option<JsonValue>,
    pub excerpt: Option<String>,
    pub cover_image: Option<String>,
    pub status: Option<String>,
    pub is_pinned: Option<bool>,
}

/// Response paginada de articulos
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedArticles {
    pub items: Vec<Article>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Query params para listar articulos
#[derive(Debug, Deserialize, IntoParams)]
pub struct ArticleQueryParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub status: Option<String>,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}
