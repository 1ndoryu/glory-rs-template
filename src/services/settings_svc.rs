use std::collections::HashMap;

use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::settings::{AnalyticsStats, TrackEvent};
use crate::repositories::analytics_repo::AnalyticsRepository;
use crate::repositories::settings_repo::SettingsRepository;

pub struct SettingsService;

impl SettingsService {
    pub async fn get_all(pool: &PgPool) -> Result<HashMap<String, String>, AppError> {
        Ok(SettingsRepository::get_all(pool).await?)
    }

    pub async fn update_batch(
        pool: &PgPool,
        settings: &HashMap<String, String>,
    ) -> Result<(), AppError> {
        SettingsRepository::upsert_batch(pool, settings).await?;
        Ok(())
    }
}

pub struct AnalyticsService;

impl AnalyticsService {
    pub async fn track_events(
        pool: &PgPool,
        events: &[TrackEvent],
        ip_hash: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), AppError> {
        if events.len() > 50 {
            return Err(AppError::Validation(
                "El lote de analytics no puede superar 50 eventos".into(),
            ));
        }
        AnalyticsRepository::insert_events(pool, events, ip_hash, user_agent).await?;
        Ok(())
    }

    pub async fn get_stats(pool: &PgPool) -> Result<AnalyticsStats, AppError> {
        Ok(AnalyticsRepository::get_stats(pool).await?)
    }
}
