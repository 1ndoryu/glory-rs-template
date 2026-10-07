/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: seed usa runtime queries
 * porque ejecuta SQL dinámico con datos de prueba y tipos genéricos. */
//! Seed de hosting: eventos del ciclo de vida de suscripciones de prueba.

use sqlx::PgPool;
use uuid::Uuid;

use super::SeedService;

/* [174A-2] Helper reutilizable: inserta un hosting_event con ON CONFLICT DO NOTHING.
 * Extraído de create_seed_hosting_events para reducir repetición de la query SQL. */
async fn insert_hosting_event(
    pool: &PgPool,
    sub_id: &Uuid,
    event_type: &str,
    details: serde_json::Value,
    interval: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO hosting_events (subscription_id, event_type, details, created_at)
         VALUES ($1, $2, $3::jsonb, NOW() - $4::interval)
         ON CONFLICT DO NOTHING",
    )
    .bind(sub_id)
    .bind(event_type)
    .bind(details)
    .bind(interval)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_hosting_event_batch(
    pool: &PgPool,
    sub_id: &Uuid,
    events: Vec<(&str, serde_json::Value, &str)>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for (event_type, details, interval) in events {
        insert_hosting_event(pool, sub_id, event_type, details, interval).await?;
        count += 1;
    }
    Ok(count)
}

async fn create_hosting_status_events(
    pool: &PgPool,
    sub_id: &Uuid,
    status: &str,
    domain_str: &str,
) -> Result<u32, sqlx::Error> {
    match status {
        "active" => {
            insert_hosting_event_batch(
                pool,
                sub_id,
                vec![
                    (
                        "provisioning_completed",
                        serde_json::json!({"message": "Hosting provisionado exitosamente en Coolify", "server_ip": "66.94.100.241", "coolify_site_name": "mitienda-test"}),
                        "29 days 22 hours",
                    ),
                    (
                        "dns_configured",
                        serde_json::json!({"message": format!("Registros DNS de {} apuntan correctamente al servidor", domain_str), "records": ["A @ → 66.94.100.241", "A www → 66.94.100.241"]}),
                        "29 days 20 hours",
                    ),
                    (
                        "ssl_issued",
                        serde_json::json!({"message": format!("Certificado SSL emitido para {}", domain_str), "provider": "Let's Encrypt", "expires": "2026-07-10"}),
                        "29 days 19 hours",
                    ),
                    (
                        "status_changed",
                        serde_json::json!({"from": "provisioning", "to": "active", "message": "Hosting activado y funcionando"}),
                        "29 days 18 hours",
                    ),
                    (
                        "payment_received",
                        serde_json::json!({"message": "Pago mensual procesado: $10.00", "amount_cents": 1000, "source": "stripe_webhook"}),
                        "1 day",
                    ),
                ],
            )
            .await
        }
        "suspended" => {
            insert_hosting_event_batch(
                pool,
                sub_id,
                vec![
                    (
                        "provisioning_completed",
                        serde_json::json!({"message": "Hosting provisionado exitosamente"}),
                        "60 days",
                    ),
                    (
                        "payment_failed",
                        serde_json::json!({"message": "Falló el cobro mensual — tarjeta rechazada", "amount_cents": 1500, "retry_count": 3}),
                        "5 days",
                    ),
                    (
                        "status_changed",
                        serde_json::json!({"from": "active", "to": "suspended", "message": "Hosting suspendido por falta de pago tras 3 intentos fallidos"}),
                        "2 days",
                    ),
                ],
            )
            .await
        }
        _ => {
            insert_hosting_event(
                pool,
                sub_id,
                "status_changed",
                serde_json::json!({
                    "from": "pending", "to": "provisioning",
                    "message": "Servidor asignado, provisioning en curso..."
                }),
                "1 hour",
            )
            .await?;
            Ok(1)
        }
    }
}

impl SeedService {
    /* [104A-seed] Crea hosting_events realistas que simulan el ciclo de vida completo
     * de un hosting comprado: pago → provisioning → DNS → SSL → activo.
     * Para hostings suspendidos agrega evento de suspensión por falta de pago. */
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn create_seed_hosting_events(
        pool: &PgPool,
        client_id: Uuid,
    ) -> Result<u32, sqlx::Error> {
        let sub_ids: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT id, status, domain FROM hosting_subscriptions WHERE user_id = $1",
        )
        .bind(client_id)
        .fetch_all(pool)
        .await?;

        let mut count = 0u32;
        for (sub_id, status, domain) in &sub_ids {
            let domain_str = domain.as_deref().unwrap_or("sin dominio");

            /* Eventos comunes: creación + provisioning iniciado */
            count += insert_hosting_event_batch(
                pool,
                sub_id,
                vec![
                    (
                        "created",
                        serde_json::json!({
                            "source": "stripe_webhook", "plan": "pro", "domain": domain_str,
                            "message": "Suscripción de hosting creada tras confirmación de pago"
                        }),
                        "30 days",
                    ),
                    (
                        "provisioning_started",
                        serde_json::json!({
                            "message": "Servidor VPS asignado, instalando WordPress + SSL",
                            "server": "vps1.nakomi.studio"
                        }),
                        "29 days 23 hours",
                    ),
                ],
            )
            .await?;
            count += create_hosting_status_events(pool, sub_id, status, domain_str).await?;
        }

        Ok(count)
    }
}
