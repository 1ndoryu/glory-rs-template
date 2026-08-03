use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::game_asset::{
    CreateGameAssetRequest, GameAssetDefinition, UpdateGameAssetRequest,
};
use crate::models::game_audit::{ACTION_ASSET_CREATED, ACTION_ASSET_UPDATED};
use crate::repositories::game_asset_repo::GameAssetRepository;
use crate::services::game_audit_svc::GameAuditService;

pub struct GameAssetService;

impl GameAssetService {
    pub async fn list_active(pool: &PgPool) -> Result<Vec<GameAssetDefinition>, AppError> {
        Ok(GameAssetRepository::list_active(pool).await?)
    }

    /// Listado completo para el panel admin (activas e inactivas).
    pub async fn list_all(pool: &PgPool) -> Result<Vec<GameAssetDefinition>, AppError> {
        Ok(GameAssetRepository::list_all(pool).await?)
    }

    /// Alta de un nuevo asset allowlisted. La autorización ya fue resuelta por
    /// el extractor `AdminUser` del handler; aquí solo se valida el input.
    /// [297A-60] La creación y su evento de auditoría comparten transacción.
    pub async fn create(
        pool: &PgPool,
        actor_id: uuid::Uuid,
        request: CreateGameAssetRequest,
    ) -> Result<GameAssetDefinition, AppError> {
        let display_name = validate_fields(&request.id, &request.display_name, &request.category)?;
        let mut tx = pool.begin().await?;

        let asset = match GameAssetRepository::create(
            &mut tx,
            &request.id,
            &display_name,
            &request.category,
        )
        .await
        {
            Ok(asset) => asset,
            Err(error) if is_unique_violation(&error) => {
                return Err(AppError::Conflict("Ya existe un asset con ese id".into()));
            }
            Err(error) => return Err(error.into()),
        };

        let payload = serde_json::json!({
            "displayName": asset.display_name,
            "category": asset.category,
            "isActive": asset.is_active,
        });
        GameAuditService::record_asset_change(
            &mut tx,
            actor_id,
            ACTION_ASSET_CREATED,
            &asset.id,
            &payload,
        )
        .await?;
        tx.commit().await?;
        Ok(asset)
    }

    /// Actualización completa de un asset, incluyendo desactivación.
    /// [297A-60] La actualización y su evento de auditoría comparten transacción.
    pub async fn update(
        pool: &PgPool,
        actor_id: uuid::Uuid,
        id: &str,
        request: UpdateGameAssetRequest,
    ) -> Result<GameAssetDefinition, AppError> {
        let display_name = validate_fields(id, &request.display_name, &request.category)?;
        let mut tx = pool.begin().await?;

        let asset = GameAssetRepository::update(
            &mut tx,
            id,
            &display_name,
            &request.category,
            request.is_active,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Asset no encontrado".into()))?;

        let payload = serde_json::json!({
            "displayName": asset.display_name,
            "category": asset.category,
            "isActive": asset.is_active,
        });
        GameAuditService::record_asset_change(
            &mut tx,
            actor_id,
            ACTION_ASSET_UPDATED,
            &asset.id,
            &payload,
        )
        .await?;
        tx.commit().await?;
        Ok(asset)
    }
}

fn validate_fields(id: &str, display_name: &str, category: &str) -> Result<String, AppError> {
    if !GameAssetDefinition::is_valid_id(id) {
        return Err(AppError::Validation(
            "Identificador de asset no válido".into(),
        ));
    }
    let display_name = GameAssetDefinition::validate_display_name(display_name)
        .map_err(|message| AppError::Validation(message.into()))?;
    if !GameAssetDefinition::is_valid_category(category) {
        return Err(AppError::Validation("Categoría de asset no válida".into()));
    }
    Ok(display_name)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation())
}
