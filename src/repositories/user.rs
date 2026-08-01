use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

pub struct UserRepository;

impl UserRepository {
    pub async fn create_unverified(
        conn: &mut sqlx::PgConnection,
        email: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash, email_verified_at)
             VALUES (gen_random_uuid(), $1, $2, NULL)
             RETURNING id, email, password_hash, role, status, created_at",
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(&mut *conn)
        .await
    }

    /// Crea un usuario con rol 'user' y estado 'active'.
    /// El rol NUNCA se acepta del request — siempre se asigna server-side.
    pub async fn create(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash, email_verified_at) \
             VALUES ($1, $2, $3, NOW()) \
             RETURNING id, email, password_hash, role, status, created_at",
        )
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .fetch_one(pool)
        .await
    }

    /// Busca un usuario por email (solo activos)
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, role, status, created_at \
             FROM users WHERE email = $1 AND status = 'active'",
        )
        .bind(email)
        .fetch_optional(pool)
        .await
    }

    /// Busca un usuario por ID (solo activos)
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, role, status, created_at \
             FROM users WHERE id = $1 AND status = 'active'",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn is_email_verified(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let row: Option<(Option<chrono::DateTime<chrono::Utc>>,)> = sqlx::query_as(
            "SELECT email_verified_at FROM users WHERE id = $1 AND status = 'active'",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(row.and_then(|value| value.0).is_some())
    }

    pub async fn mark_email_verified(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE users SET email_verified_at = COALESCE(email_verified_at, NOW())
             WHERE id = $1 AND status = 'active'",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_password(
        pool: &PgPool,
        id: Uuid,
        password_hash: &str,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1 AND status = 'active'")
                .bind(id)
                .bind(password_hash)
                .execute(pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }
}
