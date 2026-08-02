use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::game_profile::GameProfile;

pub struct GameProfileRepository;

impl GameProfileRepository {
    pub async fn get(pool: &PgPool, user_id: Uuid) -> Result<Option<GameProfile>, sqlx::Error> {
        sqlx::query_as::<_, GameProfile>(
            "SELECT user_id, display_name, revision, updated_at
             FROM user_game_profiles
             WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    /// Crea o actualiza el perfil solo si la revisión esperada coincide.
    /// La inserción inicial y la actualización ocurren bajo la misma transacción
    /// para que dos dispositivos no puedan ganar la revisión cero a la vez.
    pub async fn update_if_revision(
        pool: &PgPool,
        user_id: Uuid,
        display_name: &str,
        expected_revision: i32,
    ) -> Result<Option<GameProfile>, sqlx::Error> {
        let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

        sqlx::query(
            "INSERT INTO user_game_profiles (user_id, display_name, revision)
             VALUES ($1, 'Jugador', 0)
             ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        let updated = sqlx::query_as::<_, GameProfile>(
            "UPDATE user_game_profiles
             SET display_name = $1, revision = revision + 1, updated_at = NOW()
             WHERE user_id = $2 AND revision = $3
             RETURNING user_id, display_name, revision, updated_at",
        )
        .bind(display_name)
        .bind(user_id)
        .bind(expected_revision)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated)
    }
}
