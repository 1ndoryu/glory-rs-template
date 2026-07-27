/* [277A-13] Repositorio para SEO settings editables desde el panel admin.
 * La tabla `seo_settings` almacena metadatos SEO de páginas estáticas.
 * El prerender middleware y el audit endpoint leen de aquí en vez de hardcoded. */

use sqlx::PgPool;

/// Fila de la tabla seo_settings
#[derive(sqlx::FromRow, serde::Serialize)]
pub struct SeoSetting {
    pub path: String,
    pub label: String,
    pub title: String,
    pub description: String,
    pub og_image_url: Option<String>,
    pub json_ld_type: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct SeoSettingsRepository;

impl SeoSettingsRepository {
    /// Listar todos los SEO settings ordenados por path
    pub async fn list_all(pool: &PgPool) -> Result<Vec<SeoSetting>, sqlx::Error> {
        sqlx::query_as::<_, SeoSetting>(
            "SELECT path, label, title, description, og_image_url, json_ld_type, updated_at
             FROM seo_settings ORDER BY path",
        )
        .fetch_all(pool)
        .await
    }

    /// Buscar un setting por path exacto
    pub async fn find_by_path(
        pool: &PgPool,
        path: &str,
    ) -> Result<Option<SeoSetting>, sqlx::Error> {
        sqlx::query_as::<_, SeoSetting>(
            "SELECT path, label, title, description, og_image_url, json_ld_type, updated_at
             FROM seo_settings WHERE path = $1",
        )
        .bind(path)
        .fetch_optional(pool)
        .await
    }

    /// Crear o actualizar un SEO setting (upsert por path)
    pub async fn upsert(
        pool: &PgPool,
        path: &str,
        title: &str,
        description: &str,
        og_image_url: Option<&str>,
    ) -> Result<SeoSetting, sqlx::Error> {
        sqlx::query_as::<_, SeoSetting>(
            "INSERT INTO seo_settings (path, label, title, description, og_image_url)
             VALUES ($1, $1, $2, $3, $4)
             ON CONFLICT (path) DO UPDATE SET
                 title = EXCLUDED.title,
                 description = EXCLUDED.description,
                 og_image_url = EXCLUDED.og_image_url,
                 updated_at = NOW()
             RETURNING path, label, title, description, og_image_url, json_ld_type, updated_at",
        )
        .bind(path)
        .bind(title)
        .bind(description)
        .bind(og_image_url)
        .fetch_one(pool)
        .await
    }
}
