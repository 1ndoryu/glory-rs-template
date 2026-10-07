use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{User, UserRole};

/* [054A-1] Row intermedia para list_all con conteo total (paginación offset) */
pub struct UserWithTotal {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub active_role: Option<UserRole>,
    pub email_verified: bool,
    pub status: String,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
    pub username: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub password_set: bool,
    pub total_count: i64,
}

pub struct UserDeleteBlockers {
    pub client_orders: i64,
    pub assigned_orders: i64,
    pub reviews: i64,
    pub refunds: i64,
    pub delegations: i64,
    pub deliverables: i64,
    pub hosting_subscriptions: i64,
    pub chat_sessions: i64,
    pub blog_posts: i64,
}

impl UserDeleteBlockers {
    #[must_use]
    pub fn blocking_references(&self) -> Vec<&'static str> {
        let mut references = Vec::new();

        if self.client_orders > 0 {
            references.push("pedidos como cliente");
        }
        if self.assigned_orders > 0 {
            references.push("pedidos asignados");
        }
        if self.reviews > 0 {
            references.push("reviews");
        }
        if self.refunds > 0 {
            references.push("reembolsos");
        }
        if self.delegations > 0 {
            references.push("delegaciones");
        }
        if self.deliverables > 0 {
            references.push("entregables subidos");
        }
        if self.hosting_subscriptions > 0 {
            references.push("suscripciones de hosting");
        }
        if self.chat_sessions > 0 {
            references.push("sesiones de chat");
        }
        if self.blog_posts > 0 {
            references.push("artículos de blog");
        }

        references
    }
}

pub struct UserRepository;

impl UserRepository {
    /// Crea un usuario y retorna el registro completo
    pub async fn create(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
        password_set: bool,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        let username = email.split('@').next().unwrap_or("user").to_lowercase();
        sqlx::query_as!(
            User,
            r#"INSERT INTO users (id, email, password_hash, username, password_set)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, email, password_hash,
                       role as "role: UserRole", active_role as "active_role: UserRole",
                       email_verified, status, avatar_url, display_name, username, created_at, password_set"#,
            id,
            email,
            password_hash,
            username,
            password_set,
        )
        .fetch_one(pool)
        .await
    }

    /* [015A-1] Crea usuario con rol específico en una sola operación atómica.
     * Genera username con sufijo del UUID para evitar colisiones cuando dos emails
     * comparten el mismo prefijo (ej: john@gmail.com y john@yahoo.com). */
    pub async fn create_with_role(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
        role: UserRole,
        password_set: bool,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        let email_prefix = email.split('@').next().unwrap_or("user").to_lowercase();
        /* Usar los primeros 6 chars del UUID (sin guiones) como sufijo único */
        let id_hex = id.to_string().replace('-', "");
        let username = format!("{email_prefix}_{}", &id_hex[..6]);
        sqlx::query_as!(
            User,
            r#"INSERT INTO users (id, email, password_hash, username, role, password_set)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, email, password_hash,
                       role as "role: UserRole", active_role as "active_role: UserRole",
                       email_verified, status, avatar_url, display_name, username, created_at, password_set"#,
            id,
            email,
            password_hash,
            username,
            role as UserRole,
            password_set,
        )
        .fetch_one(pool)
        .await
    }

    /// Busca un usuario por email
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT id, email, password_hash,
                      role as "role: UserRole", active_role as "active_role: UserRole",
                      email_verified, status, avatar_url, display_name, username, created_at, password_set
             FROM users WHERE email = $1"#,
            email,
        )
        .fetch_optional(pool)
        .await
    }

    /// Busca un usuario por ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT id, email, password_hash,
                      role as "role: UserRole", active_role as "active_role: UserRole",
                      email_verified, status, avatar_url, display_name, username, created_at, password_set
             FROM users WHERE id = $1"#,
            id,
        )
        .fetch_optional(pool)
        .await
    }

    /* [044A-38] Actualiza el active_role de un admin para cambiar genuinamente de vista */
    pub async fn update_active_role(
        pool: &PgPool,
        user_id: Uuid,
        active_role: Option<UserRole>,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"UPDATE users SET active_role = $2
             WHERE id = $1
             RETURNING id, email, password_hash,
                       role as "role: UserRole", active_role as "active_role: UserRole",
                       email_verified, status, avatar_url, display_name, username, created_at, password_set"#,
            user_id,
            active_role as Option<UserRole>,
        )
        .fetch_one(pool)
        .await
    }

    /* [044A-43] Actualiza la URL del avatar del usuario */
    pub async fn update_avatar(
        pool: &PgPool,
        user_id: Uuid,
        avatar_url: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE users SET avatar_url = $2 WHERE id = $1",
            user_id,
            avatar_url
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [205A-1] Permite que cuentas bootstrap con email temporal adopten el email real.
     * Google OAuth vincula por email, por eso el cambio debe ocurrir antes de sincronizar Google.
     * [205A-2] Sin flujo de re-verificación aún, preservamos email_verified actual. */
    pub async fn update_email(
        pool: &PgPool,
        user_id: Uuid,
        email: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE users SET email = $2 WHERE id = $1", user_id, email)
            .execute(pool)
            .await?;
        Ok(())
    }

    /* [074A-23] Actualiza display_name en users + campos extendidos en user_profiles.
     * Upsert en user_profiles porque puede no existir row para el usuario. */
    pub async fn update_profile(
        pool: &PgPool,
        user_id: Uuid,
        display_name: Option<&str>,
        bio: Option<&str>,
        linkedin: Option<&str>,
        twitter: Option<&str>,
        website: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE users SET display_name = COALESCE($2, display_name) WHERE id = $1",
            user_id,
            display_name,
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            r#"INSERT INTO user_profiles (user_id, bio, linkedin, twitter, website)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id) DO UPDATE SET
               bio = COALESCE($2, user_profiles.bio),
               linkedin = COALESCE($3, user_profiles.linkedin),
               twitter = COALESCE($4, user_profiles.twitter),
               website = COALESCE($5, user_profiles.website),
               updated_at = NOW()"#,
            user_id,
            bio,
            linkedin,
            twitter,
            website,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn stripe_customer_id(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query!(
            "SELECT stripe_customer_id FROM users WHERE id = $1",
            user_id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.and_then(|record| record.stripe_customer_id))
    }

    pub async fn set_stripe_customer_id(
        pool: &PgPool,
        user_id: Uuid,
        stripe_customer_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE users SET stripe_customer_id = $2 WHERE id = $1",
            user_id,
            stripe_customer_id,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /* [054A-1] Lista paginada de usuarios con búsqueda y filtros para panel admin.
     * Usa COUNT(*) OVER() para obtener el total sin query extra.
     * Filtros: role, status, búsqueda por email/display_name (ILIKE). */
    pub async fn list_all(
        pool: &PgPool,
        search: Option<&str>,
        role_filter: Option<UserRole>,
        status_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserWithTotal>, sqlx::Error> {
        let search_pattern = search.map(|s| format!("%{s}%"));
        let status_owned = status_filter.map(str::to_string);

        sqlx::query_as!(
            UserWithTotal,
            r#"SELECT id, email, password_hash,
                      role as "role: UserRole",
                      active_role as "active_role: UserRole",
                      email_verified, status, avatar_url, display_name, username, created_at, password_set,
                      COUNT(*) OVER() as "total_count!: i64"
             FROM users
             WHERE ($1::text IS NULL OR email ILIKE $1 OR display_name ILIKE $1)
               AND ($2::user_role IS NULL OR role = $2)
               AND ($3::text IS NULL OR status = $3)
             ORDER BY created_at DESC
             LIMIT $4 OFFSET $5"#,
            search_pattern as Option<String>,
            role_filter as Option<UserRole>,
            status_owned,
            limit,
            offset,
        )
        .fetch_all(pool)
        .await
    }

    /* [054A-1] Cambia el role real de un usuario (admin action).
     * No permite que un admin se cambie el rol a sí mismo por seguridad. */
    pub async fn update_role(
        pool: &PgPool,
        user_id: Uuid,
        new_role: UserRole,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"UPDATE users SET role = $2, active_role = NULL
             WHERE id = $1
             RETURNING id, email, password_hash,
                       role as "role: UserRole", active_role as "active_role: UserRole",
                       email_verified, status, avatar_url, display_name, username, created_at, password_set"#,
            user_id,
            new_role as UserRole,
        )
        .fetch_one(pool)
        .await
    }

    /* [054A-1] Cambia el status de un usuario (ban/reactivar).
     * status = 'active' | 'banned' | 'suspended' */
    pub async fn update_status(
        pool: &PgPool,
        user_id: Uuid,
        status: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"UPDATE users SET status = $2
             WHERE id = $1
             RETURNING id, email, password_hash,
                       role as "role: UserRole", active_role as "active_role: UserRole",
                       email_verified, status, avatar_url, display_name, username, created_at, password_set"#,
            user_id,
            status,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete_blockers(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<UserDeleteBlockers, sqlx::Error> {
        let row = sqlx::query!(
            r#"SELECT
                (SELECT COUNT(*) FROM orders WHERE client_id = $1) as "client_orders!",
                (SELECT COUNT(*) FROM orders WHERE assigned_employee_id = $1) as "assigned_orders!",
                (SELECT COUNT(*) FROM order_reviews WHERE client_id = $1 OR employee_id = $1) as "reviews!",
                (SELECT COUNT(*) FROM order_refunds WHERE requested_by = $1 OR reviewed_by = $1) as "refunds!",
                (SELECT COUNT(*) FROM order_delegations WHERE from_employee_id = $1 OR to_employee_id = $1) as "delegations!",
                (SELECT COUNT(*) FROM phase_deliverables WHERE uploaded_by = $1) as "deliverables!",
                (SELECT COUNT(*) FROM hosting_subscriptions WHERE user_id = $1) as "hosting_subscriptions!",
                (SELECT COUNT(*) FROM chat_sessions WHERE user_id = $1 OR assigned_staff_id = $1) as "chat_sessions!",
                (SELECT COUNT(*) FROM blog_posts WHERE author_id = $1) as "blog_posts!""#,
            user_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(UserDeleteBlockers {
            client_orders: row.client_orders,
            assigned_orders: row.assigned_orders,
            reviews: row.reviews,
            refunds: row.refunds,
            delegations: row.delegations,
            deliverables: row.deliverables,
            hosting_subscriptions: row.hosting_subscriptions,
            chat_sessions: row.chat_sessions,
            blog_posts: row.blog_posts,
        })
    }

    pub async fn hard_delete(pool: &PgPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /* [084A-1] Busca el primer usuario activo con un rol dado.
     * Usado por switch_role para encontrar un usuario real a impersonar.
     * [074A-59] Prioriza usuarios de fixtures (@test.com) para que switch-role
     * impersone al usuario que tiene datos de prueba, no un registro vacío. */
    pub async fn find_first_by_role(
        pool: &PgPool,
        role: UserRole,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT id, email, password_hash,
                      role as "role: UserRole", active_role as "active_role: UserRole",
                      email_verified, status, avatar_url, display_name, username, created_at, password_set
             FROM users
             WHERE role = $1 AND status = 'active'
             ORDER BY
                 CASE WHEN email LIKE '%@test.com' THEN 0 ELSE 1 END,
                 created_at DESC
             LIMIT 1"#,
            role as UserRole,
        )
        .fetch_optional(pool)
        .await
    }

    /* [T-6] IDs de todos los admins activos para notificaciones de escalación */
    pub async fn admin_ids(pool: &PgPool) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query_scalar!(
            r#"SELECT id FROM users WHERE role = 'admin' AND status = 'active'"#
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [114A-8] Emails de todos los admins activos para email de escalación */
    pub async fn admin_emails(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query_scalar!(
            r#"SELECT email FROM users WHERE role = 'admin' AND status = 'active'"#
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [124A-SENT-R1] Nombre para mostrar de un usuario: display_name o email como fallback.
     * Usado en problems.rs para construir ProblemResponse con nombre legible del reporter.
     * Retorna "Desconocido" si el usuario no existe. runtime query (sin macro). */
    pub async fn display_name_or_email(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<String, sqlx::Error> {
        let name = sqlx::query_scalar::<_, String>(
            "SELECT COALESCE(display_name, email) FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or_else(|| "Desconocido".to_string());
        Ok(name)
    }

    /* [124A-SENT-R1] IDs de empleados con disponibilidad 'available'.
     * Usado al reabrir una orden para notificar empleados que pueden tomarla.
     * runtime query (sin macro) para no requerir sqlx prepare. */
    pub async fn available_employee_ids(pool: &PgPool) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM employee_profiles WHERE availability = 'available'",
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* [07AA-2] Bootstrap Guillermo: aplica perfil cliente a un usuario existente
     * (display_name solo si estaba vacío + rol client + hash nuevo). El SQL vivía
     * en admin_client_bootstrap.rs (handler-accede-bd-rs); el repositorio es su
     * casa (DIP). Semántica COALESCE preservada verbatim. */
    pub async fn apply_bootstrap_credentials(
        pool: &PgPool,
        user_id: Uuid,
        display_name: &str,
        role: UserRole,
        password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE users SET display_name = COALESCE(display_name, $2), role = $3, password_hash = $4 WHERE id = $1"#,
            user_id,
            display_name,
            role as UserRole,
            password_hash,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [07AA-2] Bootstrap Guillermo: fija display_name y deja email sin verificar
     * en el usuario recién creado. Movido desde el handler (DIP). */
    pub async fn mark_bootstrap_created(
        pool: &PgPool,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE users SET display_name = $2, email_verified = false WHERE id = $1"#,
            user_id,
            display_name,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [154A-5] Actualiza la contraseña de un usuario y marca password_set = true.
     * Usado cuando un usuario de quick_register establece su propia contraseña,
     * o cuando usa el endpoint explícito de set-password. */
    pub async fn set_password(
        pool: &PgPool,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"UPDATE users SET password_hash = $2, password_set = true
             WHERE id = $1
             RETURNING id, email, password_hash,
                       role as "role: UserRole", active_role as "active_role: UserRole",
                       email_verified, status, avatar_url, display_name, username, created_at, password_set"#,
            user_id,
            password_hash,
        )
        .fetch_one(pool)
        .await
    }

    /* [124A-R1] Obtener email de un usuario por ID */
    pub async fn get_email(pool: &PgPool, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar!(r#"SELECT email FROM users WHERE id = $1"#, user_id)
            .fetch_optional(pool)
            .await
    }

    /* [124A-R1] Obtener display_name de un usuario por ID.
     * display_name es nullable en la BD → query_scalar devuelve Option<Option<String>>;
     * flatten() colapsa ambos niveles en Option<String>. */
    pub async fn get_display_name(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar!(r#"SELECT display_name FROM users WHERE id = $1"#, user_id)
            .fetch_optional(pool)
            .await
            .map(Option::flatten)
    }

    /* [124A-R1] Primer admin activo (por fecha de creación). Usado para comisiones. */
    pub async fn first_admin_id(pool: &PgPool) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar!(
            r#"SELECT id FROM users WHERE role = 'admin' AND status = 'active'
               ORDER BY created_at ASC LIMIT 1"#
        )
        .fetch_optional(pool)
        .await
    }

    /* [124A-SENT-R1] Info de avatar y nombre de múltiples usuarios por sus IDs.
     * Usado en chat/mod.rs para enriquecer mensajes con datos del sender.
     * runtime query (sin macro) — ANY($1) con &[Uuid] no requiere sqlx prepare. */
    pub async fn info_by_ids(
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, Option<String>, Option<String>)>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            avatar_url: Option<String>,
            display_name: Option<String>,
        }
        /* ANY($1) con &[Uuid] no es compatible con query_as! macro. */
        // sentinel-disable-next-line sqlx-query-as-sin-macro
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, avatar_url, display_name FROM users WHERE id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.id, r.avatar_url, r.display_name))
            .collect())
    }
}
