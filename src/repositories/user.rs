use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

pub struct UserRepository;

impl UserRepository {
    /// Crea un usuario con rol 'user' y estado 'active'.
    /// El rol NUNCA se acepta del request — siempre se asigna server-side.
    pub async fn create(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash) \
             VALUES ($1, $2, $3) \
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
}
