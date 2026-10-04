/* sentinel-disable-file sqlx-query-sin-macro: fixtures.rs usa queries dinámicas para
 * setup inicial (detección de _glory_fixtures y cascade de seed legacy con
 * formatos generados en runtime, sin macro tipada posible). */

/* [01AA-4-F3r] Extraído de `main.rs`: conexión a BD + fixtures + limpieza de
 * seed legacy. Sin cambio de comportamiento. */

use std::time::Duration;

use argon2::password_hash::rand_core::OsRng;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use glory_rs::fixtures::ContentManager;
use sqlx::PgPool;

/* [096A-1] Pool con max_lifetime + idle_timeout para evitar conexiones
 * zombie que provocan CLOSE_WAIT y deadlock del event loop. */
pub async fn connect_pool(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .max_lifetime(Duration::from_mins(30))
        .idle_timeout(Duration::from_mins(5))
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}

/* [250A-1] Extraído de main() para cumplir límite de 100 líneas.
 * Configura password hasher, content manager, limpia seed legacy y sincroniza
 * content/ TOMLs si FIXTURES_SYNC=true. */
#[allow(clippy::too_many_lines)]
pub async fn setup_and_run_fixtures(pool: &sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let password_hasher: glory_rs::fixtures::PasswordHasher = Box::new(|plain| {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(plain.as_bytes(), &salt)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?
            .to_string();
        Ok(hash)
    });
    let fixture_manager =
        ContentManager::new(pool.clone(), "content").with_password_hasher(password_hasher);

    cleanup_legacy_seed(pool).await;

    let fixtures_sync =
        std::env::var("FIXTURES_SYNC").is_ok_and(|v| v.eq_ignore_ascii_case("true") || v == "1");

    if fixtures_sync {
        match fixture_manager.sync_all().await {
            Ok(report) => {
                tracing::info!("[fixtures] {}", report.summary());
                for err in &report.errors {
                    tracing::error!("[fixtures] {err}");
                }
            }
            Err(e) => tracing::error!("[fixtures] Error syncing: {e}"),
        }
    } else {
        tracing::info!("[fixtures] Sync desactivado (FIXTURES_SYNC != true)");
    }

    Ok(())
}

/* [074A-23] Limpia datos de seed legacy que ahora son manejados por fixtures.
 * Borra órdenes (con cascade FK completo) y hosting de test emails conocidos
 * que NO están rastreados en _glory_fixtures. Es no-op si no hay datos legacy. */
async fn cleanup_legacy_seed(pool: &sqlx::PgPool) {
    let test_emails = &["cliente@test.com", "empleado@test.com"];
    let tables_exist: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = '_glory_fixtures')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !tables_exist {
        return;
    }

    /* Subquery: IDs de órdenes legacy (no fixture-tracked) de test users */
    let legacy_orders_subquery = "SELECT o.id FROM orders o
         JOIN users u ON o.client_id = u.id
         WHERE u.email = ANY($1)
         AND NOT EXISTS (
             SELECT 1 FROM _glory_fixtures gf
             WHERE gf.table_name = 'orders' AND gf.db_id = o.id::text
         )";

    /* Romper FK circular orders↔chat_sessions */
    let _ = sqlx::query(&format!(
        "UPDATE orders SET chat_session_id = NULL WHERE id IN ({legacy_orders_subquery})"
    ))
    .bind(test_emails)
    .execute(pool)
    .await;

    /* Cascade completo: chat → reviews → refunds → delegations → payments → deliverables → phases → orders */
    let cascade_tables = [
        (
            "chat_messages",
            "session_id IN (SELECT id FROM chat_sessions WHERE order_id IN ({q}))",
        ),
        (
            "chat_session_notes",
            "session_id IN (SELECT id FROM chat_sessions WHERE order_id IN ({q}))",
        ),
        ("chat_sessions", "order_id IN ({q})"),
        ("order_reviews", "order_id IN ({q})"),
        ("order_refunds", "order_id IN ({q})"),
        ("order_delegations", "order_id IN ({q})"),
        ("order_payments", "order_id IN ({q})"),
        (
            "phase_deliverables",
            "phase_id IN (SELECT id FROM order_phases WHERE order_id IN ({q}))",
        ),
        ("order_phases", "order_id IN ({q})"),
    ];

    let mut total_deleted = 0u64;
    for (table, condition_tpl) in &cascade_tables {
        let condition = condition_tpl.replace("{q}", legacy_orders_subquery);
        let sql = format!("DELETE FROM {table} WHERE {condition}");
        if let Ok(r) = sqlx::query(&sql).bind(test_emails).execute(pool).await {
            total_deleted += r.rows_affected();
        }
    }

    /* Borrar órdenes legacy */
    let sql = format!("DELETE FROM orders WHERE id IN ({legacy_orders_subquery})");
    if let Ok(r) = sqlx::query(&sql).bind(test_emails).execute(pool).await {
        total_deleted += r.rows_affected();
    }

    /* Borrar hosting legacy (no fixture-tracked) */
    let legacy_hosting_subquery = "SELECT hs.id FROM hosting_subscriptions hs
         JOIN users u ON hs.user_id = u.id
         WHERE u.email = ANY($1)
         AND NOT EXISTS (
             SELECT 1 FROM _glory_fixtures gf
             WHERE gf.table_name = 'hosting_subscriptions' AND gf.db_id = hs.id::text
         )";

    let _ = sqlx::query(&format!(
        "DELETE FROM hosting_events WHERE subscription_id IN ({legacy_hosting_subquery})"
    ))
    .bind(test_emails)
    .execute(pool)
    .await
    .map(|r| total_deleted += r.rows_affected());

    let _ = sqlx::query(&format!(
        "DELETE FROM hosting_subscriptions WHERE id IN ({legacy_hosting_subquery})"
    ))
    .bind(test_emails)
    .execute(pool)
    .await
    .map(|r| total_deleted += r.rows_affected());

    if total_deleted > 0 {
        tracing::info!("[cleanup] Legacy seed: {total_deleted} records deleted");
    }
}
