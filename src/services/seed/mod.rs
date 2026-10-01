/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: seed usa runtime queries
 * porque ejecuta SQL dinámico con datos de prueba y tipos genéricos. */
/* [064A-62] Servicio de seed: recrea datos de prueba suplementarios desde el panel admin.
 * [084A-25] Limpieza: usuarios, órdenes, proyectos, equipo, hosting y pagos ahora
 * se gestionan con fixtures TOML en content/ (glory-rs ContentManager).
 * [025B-2] Fixtures nuevos: order_problems y vps_subscriptions.
 * El seed suplementario crea: notificaciones, reviews, chat, activity, hosting_events,
 * wallet con balance de prueba y withdrawal_requests en diferentes estados. */

mod content;
mod hosting;
mod wallet;

use sqlx::PgPool;
use uuid::Uuid;

/* [084A-25] Emails de test usados para identificar datos de fixture y seed */
const TEST_EMAILS: [&str; 5] = [
    "cliente@test.com",
    "empleado@test.com",
    "test@test.com",
    "testfase2@test.com",
    "employee1@test.com",
];

pub struct SeedService;

impl SeedService {
    /// Elimina todos los datos generados por el seed (usuarios test + sus órdenes/fases/chat)
    pub async fn delete_test_data(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = ANY($1)")
            .bind(&TEST_EMAILS[..])
            .fetch_all(pool)
            .await?;

        if ids.is_empty() {
            return Ok(0);
        }

        /* Borrar en orden por FKs: refunds → payments → deliverables → reviews → fases → órdenes → chat → notificaciones → hosting → activity → usuarios */
        sqlx::query(
            "DELETE FROM order_refunds WHERE order_id IN (SELECT id FROM orders WHERE client_id = ANY($1))",
        )
        .bind(&ids)
        .execute(pool)
        .await?;

        sqlx::query(
            "DELETE FROM order_payments WHERE order_id IN (SELECT id FROM orders WHERE client_id = ANY($1))",
        )
        .bind(&ids)
        .execute(pool)
        .await?;

        sqlx::query(
            "DELETE FROM phase_deliverables WHERE phase_id IN (SELECT id FROM order_phases WHERE order_id IN (SELECT id FROM orders WHERE client_id = ANY($1)))",
        )
        .bind(&ids)
        .execute(pool)
        .await?;

        sqlx::query(
            "DELETE FROM order_reviews WHERE order_id IN (SELECT id FROM orders WHERE client_id = ANY($1))",
        )
        .bind(&ids)
        .execute(pool)
        .await?;

        sqlx::query(
            "DELETE FROM order_delegations WHERE order_id IN (SELECT id FROM orders WHERE client_id = ANY($1))",
        )
        .bind(&ids)
        .execute(pool)
        .await?;

        sqlx::query(
            "DELETE FROM order_phases WHERE order_id IN (SELECT id FROM orders WHERE client_id = ANY($1))",
        )
        .bind(&ids)
        .execute(pool)
        .await?;

        /* Limpiar chat_session_id de orders antes de borrar chat_sessions (FK) */
        sqlx::query("UPDATE orders SET chat_session_id = NULL WHERE client_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        /* Chat depende de orders (order_id FK) y users (user_id FK) — borrar antes de orders */
        sqlx::query("DELETE FROM chat_messages WHERE session_id IN (SELECT id FROM chat_sessions WHERE user_id = ANY($1) OR order_id IN (SELECT id FROM orders WHERE client_id = ANY($1)))")
            .bind(&ids)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM chat_sessions WHERE user_id = ANY($1) OR order_id IN (SELECT id FROM orders WHERE client_id = ANY($1))")
            .bind(&ids)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM orders WHERE client_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        /* Hosting: borrar eventos primero, luego suscripciones */
        sqlx::query(
            "DELETE FROM hosting_events WHERE subscription_id IN (SELECT id FROM hosting_subscriptions WHERE user_id = ANY($1))",
        )
        .bind(&ids)
        .execute(pool)
        .await?;

        sqlx::query("DELETE FROM hosting_subscriptions WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM notifications WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM activity_log WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        let result = sqlx::query("DELETE FROM users WHERE id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        /* [084A-25] Proyectos y team_members ahora son gestionados por fixtures TOML.
         * No se borran aquí — el fixture sync los re-crea al reiniciar. */

        Ok(result.rows_affected())
    }

    /* [084A-25] Recrea datos suplementarios de prueba (no gestionados por fixtures).
     * Usuarios, órdenes, hosting, proyectos y equipo los gestiona content/ vía glory-rs.
     * Aquí solo creamos notificaciones, reviews, chat, activity y hosting_events. */
    pub async fn recreate_test_data(
        pool: &PgPool,
        seed_requester_id: Uuid,
    ) -> Result<String, sqlx::Error> {
        Self::delete_supplemental_data(pool).await?;

        /* [045A-1] La wallet del panel consulta SIEMPRE al user_id autenticado.
         * Sembrar solo cliente@test.com / empleado@test.com deja la vista admin en $0.00.
         * Por eso también limpiamos y recreamos movimientos/retiros para quien ejecuta el seed. */
        Self::delete_wallet_seed_for_user(pool, seed_requester_id).await?;

        let client_id = Self::find_user(pool, "cliente@test.com").await?;
        let employee_id = Self::find_user(pool, "empleado@test.com").await?;

        let (Some(client_id), Some(employee_id)) = (client_id, employee_id) else {
            return Ok(
                "No se encontraron usuarios de prueba (cliente@test.com / empleado@test.com). \
                Reiniciar el servidor para que los fixtures de content/ se sincronicen primero."
                    .into(),
            );
        };

        let notif_count = Self::create_seed_notifications(pool, client_id, employee_id).await?;
        let review_count = Self::create_seed_reviews(pool, client_id, employee_id).await?;
        let chat_count = Self::create_seed_chat(pool, client_id, employee_id).await?;
        let activity_count = Self::create_seed_activity(pool, client_id, employee_id).await?;
        let events_count = Self::create_seed_hosting_events(pool, client_id).await?;
        let (wallet_balance, withdrawal_count) =
            Self::create_seed_wallet(pool, client_id, employee_id, seed_requester_id).await?;

        Ok(format!(
            "Seed completado: {notif_count} notificaciones + {review_count} reviews + \
             {chat_count} mensajes chat + {activity_count} activity log + \
             {events_count} hosting events + wallet ${:.2} + {withdrawal_count} retiros. \
             Incluye wallet demo para la sesión actual. Credenciales: cliente@test.com/cliente, \
             empleado@test.com/empleado",
            f64::from(wallet_balance) / 100.0
        ))
    }

    /* [084A-25] Busca un usuario por email. Retorna None si no existe. */
    async fn find_user(pool: &PgPool, email: &str) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
    }

    /* [084A-25] Borra solo datos suplementarios (no gestionados por fixtures).
     * No toca users, orders, phases, payments, hosting, projects, team_members. */
    async fn delete_supplemental_data(pool: &PgPool) -> Result<(), sqlx::Error> {
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = ANY($1)")
            .bind(&TEST_EMAILS[..])
            .fetch_all(pool)
            .await?;

        if ids.is_empty() {
            return Ok(());
        }

        /* Reviews de test users */
        sqlx::query("DELETE FROM order_reviews WHERE client_id = ANY($1) OR employee_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        /* Chat: desvincular sesiones de órdenes antes de borrar */
        sqlx::query("UPDATE orders SET chat_session_id = NULL WHERE client_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM chat_messages WHERE session_id IN (SELECT id FROM chat_sessions WHERE user_id = ANY($1))")
            .bind(&ids)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM chat_sessions WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        /* Notificaciones y activity log */
        sqlx::query("DELETE FROM notifications WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM activity_log WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        /* Hosting events (las suscripciones son fixture-managed, pero los eventos no) */
        sqlx::query("DELETE FROM hosting_events WHERE subscription_id IN (SELECT id FROM hosting_subscriptions WHERE user_id = ANY($1))")
            .bind(&ids)
            .execute(pool)
            .await?;

        /* [025B-2] Withdrawal requests y wallet transactions son suplementales */
        sqlx::query("DELETE FROM withdrawal_requests WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM wallet_transactions WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM user_wallets WHERE user_id = ANY($1)")
            .bind(&ids)
            .execute(pool)
            .await?;

        Ok(())
    }
}
