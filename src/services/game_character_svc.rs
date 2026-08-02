use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::game_character::{
    CreateGameCharacterRequest, GameCharacterDefinition, UpdateGameCharacterRequest,
};
use crate::repositories::game_character_repo::GameCharacterRepository;

pub struct GameCharacterService;

impl GameCharacterService {
    pub async fn list_active(pool: &PgPool) -> Result<Vec<GameCharacterDefinition>, AppError> {
        Ok(GameCharacterRepository::list_active(pool).await?)
    }

    /// Listado completo para el panel admin (activas e inactivas).
    pub async fn list_all(pool: &PgPool) -> Result<Vec<GameCharacterDefinition>, AppError> {
        Ok(GameCharacterRepository::list_all(pool).await?)
    }

    /// Alta de una nueva opción allowlisted. La autorización ya fue resuelta
    /// por el extractor `AdminUser` del handler; aquí solo se valida el input.
    pub async fn create(
        pool: &PgPool,
        request: CreateGameCharacterRequest,
    ) -> Result<GameCharacterDefinition, AppError> {
        let display_name = validate_fields(&request.id, &request.display_name, &request.body_tone)?;

        match GameCharacterRepository::create(pool, &request.id, &display_name, &request.body_tone)
            .await
        {
            Ok(character) => Ok(character),
            Err(error) if is_unique_violation(&error) => Err(AppError::Conflict(
                "Ya existe un personaje con ese id".into(),
            )),
            Err(error) => Err(error.into()),
        }
    }

    /// Actualización completa de una opción, incluyendo desactivación.
    pub async fn update(
        pool: &PgPool,
        id: &str,
        request: UpdateGameCharacterRequest,
    ) -> Result<GameCharacterDefinition, AppError> {
        let display_name = validate_fields(id, &request.display_name, &request.body_tone)?;

        GameCharacterRepository::update(
            pool,
            id,
            &display_name,
            &request.body_tone,
            request.is_active,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Personaje no encontrado".into()))
    }
}

fn validate_fields(id: &str, display_name: &str, body_tone: &str) -> Result<String, AppError> {
    if !GameCharacterDefinition::is_valid_id(id) {
        return Err(AppError::Validation(
            "Identificador de personaje no válido".into(),
        ));
    }
    let display_name = GameCharacterDefinition::validate_display_name(display_name)
        .map_err(|message| AppError::Validation(message.into()))?;
    if !GameCharacterDefinition::is_valid_body_tone(body_tone) {
        return Err(AppError::Validation("Tono de cuerpo no válido".into()));
    }
    Ok(display_name)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation())
}
