use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::collections::HashMap;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::settings::{AnalyticsStats, TrackEventsRequest, UpdateSettingsRequest};
use crate::services::settings_svc::{AnalyticsService, SettingsService};
use crate::AppState;

/// Obtener todos los settings (publico para temas/fonts)
pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, String>>, AppError> {
    let settings = SettingsService::get_all(&state.pool).await?;
    Ok(Json(settings))
}

/// Actualizar settings (admin)
pub async fn update_settings(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<StatusCode, AppError> {
    SettingsService::update_batch(&state.pool, &req.settings).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Registrar eventos de analytics (publico)
pub async fn track_events(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<TrackEventsRequest>,
) -> Result<StatusCode, AppError> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    for event in &req.events {
        event
            .validate()
            .map_err(|error| AppError::Validation(error.to_string()))?;
    }
    let ip_hash = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|ip| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            ip.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        });

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    AnalyticsService::track_events(
        &state.pool,
        &req.events,
        ip_hash.as_deref(),
        user_agent.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Obtener estadisticas (admin)
pub async fn get_analytics_stats(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<AnalyticsStats>, AppError> {
    let analytics = AnalyticsService::get_stats(&state.pool).await?;
    Ok(Json(analytics))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        /* Públicos */
        .route("/settings", get(get_settings))
        .route("/analytics/events", post(track_events))
        /* Admin */
        .route("/admin/settings", post(update_settings))
        .route("/admin/analytics/stats", get(get_analytics_stats))
}
