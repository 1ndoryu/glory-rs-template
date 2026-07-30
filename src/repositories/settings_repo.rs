use std::collections::HashMap;

use sqlx::PgPool;

use crate::models::settings::SiteSetting;

pub struct SettingsRepository;

impl SettingsRepository {
    pub async fn get_all(pool: &PgPool) -> Result<HashMap<String, String>, sqlx::Error> {
        let rows =
            sqlx::query_as::<_, SiteSetting>("SELECT key, value, updated_at FROM site_settings")
                .fetch_all(pool)
                .await?;

        Ok(rows.into_iter().map(|s| (s.key, s.value)).collect())
    }

    pub async fn upsert(pool: &PgPool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO site_settings (key, value, updated_at) VALUES ($1, $2, NOW()) \
             ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_batch(
        pool: &PgPool,
        settings: &HashMap<String, String>,
    ) -> Result<(), sqlx::Error> {
        for (key, value) in settings {
            Self::upsert(pool, key, value).await?;
        }
        Ok(())
    }
}
