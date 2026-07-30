use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Configuracion del sitio (clave-valor)
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct SiteSetting {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

/// Request para actualizar settings
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSettingsRequest {
    pub settings: std::collections::HashMap<String, String>,
}

/// Evento de analytics
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct AnalyticsEvent {
    pub id: Uuid,
    pub event_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub ip_hash: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request batch de eventos
#[derive(Debug, Deserialize, ToSchema)]
pub struct TrackEventsRequest {
    pub events: Vec<TrackEvent>,
}

/// Evento individual
#[derive(Debug, Deserialize, ToSchema)]
pub struct TrackEvent {
    pub event_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

/// Estadisticas agregadas
#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsStats {
    pub total_page_views: i64,
    pub total_clicks: i64,
    pub total_downloads: i64,
    pub total_purchases: i64,
    pub top_articles: Vec<TopArticle>,
    pub recent_events: Vec<RecentEvent>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TopArticle {
    pub id: Uuid,
    pub title: String,
    pub views: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecentEvent {
    pub event_type: String,
    pub target_type: Option<String>,
    pub created_at: DateTime<Utc>,
}
