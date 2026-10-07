/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: seed usa runtime queries
 * porque ejecuta SQL dinámico con datos de prueba y tipos genéricos. */
//! Seed de contenido social: notificaciones, reviews, chat y activity de prueba.

use sqlx::PgPool;
use uuid::Uuid;

use super::SeedService;

impl SeedService {
    /* [074A-2] Notificaciones de prueba para que la campanita muestre algo */
    pub(crate) async fn create_seed_notifications(
        pool: &PgPool,
        client_id: Uuid,
        employee_id: Uuid,
    ) -> Result<u32, sqlx::Error> {
        let notifs: &[(&str, Uuid, &str, &str, Option<&str>)] = &[
            (
                "order_assigned",
                employee_id,
                "Nueva orden asignada",
                "Se te asignó la orden #1 de Diseño Web Básico.",
                Some("/panel?tab=ordenes"),
            ),
            (
                "payment_received",
                client_id,
                "Pago confirmado",
                "Tu pago de $100.00 fue procesado exitosamente.",
                Some("/panel?tab=ordenes"),
            ),
            (
                "phase_delivered",
                client_id,
                "Entrega lista para revisión",
                "El freelancer entregó la Fase 1 de tu orden.",
                Some("/panel?tab=ordenes"),
            ),
            (
                "message_received",
                client_id,
                "Nuevo mensaje",
                "Empleado te envió un mensaje en el chat de la orden.",
                Some("/panel?tab=mensajes"),
            ),
            (
                "order_completed",
                client_id,
                "Orden completada",
                "Tu orden de Agentes IA ha sido completada. ¡Déjanos una reseña!",
                Some("/panel?tab=ordenes"),
            ),
            (
                "order_completed",
                employee_id,
                "Orden completada",
                "La orden #3 de Agentes IA fue marcada como completada.",
                Some("/panel?tab=ordenes"),
            ),
        ];

        for (ntype, user_id, title, body, link) in notifs {
            sqlx::query(
                "INSERT INTO notifications (user_id, notification_type, title, body, link, read)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(user_id)
            .bind(ntype)
            .bind(title)
            .bind(body)
            .bind(link)
            .bind(ntype == &"order_completed") /* completada = ya leída */
            .execute(pool)
            .await?;
        }

        #[allow(clippy::cast_possible_truncation)]
        Ok(notifs.len() as u32)
    }

    /* [074A-2] Review en la orden completada */
    pub(crate) async fn create_seed_reviews(
        pool: &PgPool,
        client_id: Uuid,
        employee_id: Uuid,
    ) -> Result<u32, sqlx::Error> {
        let completed_order: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM orders WHERE client_id = $1 AND status = 'completed' LIMIT 1",
        )
        .bind(client_id)
        .fetch_optional(pool)
        .await?;

        let Some(order_id) = completed_order else {
            return Ok(0);
        };

        sqlx::query(
            "INSERT INTO order_reviews (order_id, client_id, employee_id, rating, comment, employee_response, employee_responded_at)
             VALUES ($1, $2, $3, 5, 'Excelente trabajo, muy profesional. Entregas puntuales y comunicación clara.', 'Gracias por confiar en nosotros, fue un placer trabajar contigo.', NOW() - INTERVAL '1 day')
             ON CONFLICT (order_id) DO NOTHING",
        )
        .bind(order_id).bind(client_id).bind(employee_id)
        .execute(pool)
        .await?;

        Ok(1)
    }

    /* [074A-2] Chat con mensajes entre cliente y empleado en la orden in_progress */
    pub(crate) async fn create_seed_chat(
        pool: &PgPool,
        client_id: Uuid,
        employee_id: Uuid,
    ) -> Result<u32, sqlx::Error> {
        let in_progress_order: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM orders WHERE client_id = $1 AND status = 'in_progress' LIMIT 1",
        )
        .bind(client_id)
        .fetch_optional(pool)
        .await?;

        let Some(order_id) = in_progress_order else {
            return Ok(0);
        };

        let session_id: Uuid = sqlx::query_scalar(
            "INSERT INTO chat_sessions (user_id, order_id, status, assigned_staff_id, ai_enabled)
             VALUES ($1, $2, 'active', $3, false)
             RETURNING id",
        )
        .bind(client_id)
        .bind(order_id)
        .bind(employee_id)
        .fetch_one(pool)
        .await?;

        /* Vincular sesión a la orden */
        sqlx::query("UPDATE orders SET chat_session_id = $1 WHERE id = $2")
            .bind(session_id)
            .bind(order_id)
            .execute(pool)
            .await?;

        let messages: &[(&str, &str, &str)] = &[
            ("client", &client_id.to_string(), "Hola, quería consultar sobre el avance de mi sitio web."),
            ("employee", &employee_id.to_string(), "¡Hola! Estoy trabajando en el diseño. Te comparto un boceto en las próximas horas."),
            ("client", &client_id.to_string(), "Perfecto, ¿podrías incluir tonos verdes como en el logo?"),
            ("employee", &employee_id.to_string(), "Sí, ya lo tengo en cuenta. La paleta principal será verde oscuro + blanco."),
            ("client", &client_id.to_string(), "Genial, quedo atento. ¡Gracias!"),
        ];

        let mut count = 0u32;
        for (idx, (sender_type, sender_id, content)) in messages.iter().enumerate() {
            sqlx::query(
                "INSERT INTO chat_messages (session_id, sender_type, sender_id, content, created_at)
                 VALUES ($1, $2, $3, $4, NOW() - ($5 || ' hours')::INTERVAL)",
            )
            .bind(session_id).bind(sender_type).bind(sender_id).bind(content)
            .bind((messages.len() - idx).to_string())
            .execute(pool)
            .await?;
            count += 1;
        }

        Ok(count)
    }

    /* [074A-2] Activity log para dashboard admin */
    pub(crate) async fn create_seed_activity(
        pool: &PgPool,
        client_id: Uuid,
        employee_id: Uuid,
    ) -> Result<u32, sqlx::Error> {
        let orders: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM orders WHERE client_id = $1 ORDER BY created_at LIMIT 4",
        )
        .bind(client_id)
        .fetch_all(pool)
        .await?;

        if orders.is_empty() {
            return Ok(0);
        }

        let mut count = 0u32;
        for (idx, order_id) in orders.iter().enumerate() {
            sqlx::query(
                "INSERT INTO activity_log (user_id, action, entity_type, entity_id, details, created_at)
                 VALUES ($1, 'order_created', 'order', $2, $3::jsonb, NOW() - ($4 || ' days')::INTERVAL)",
            )
            .bind(client_id).bind(order_id)
            .bind(serde_json::json!({"source": "seed"}))
            .bind((14 - i32::try_from(idx).unwrap_or(0)).to_string())
            .execute(pool)
            .await?;
            count += 1;

            if idx == 0 {
                sqlx::query(
                    "INSERT INTO activity_log (user_id, action, entity_type, entity_id, details, created_at)
                     VALUES ($1, 'order_assigned', 'order', $2, $3::jsonb, NOW() - '12 days'::INTERVAL)",
                )
                .bind(employee_id).bind(order_id)
                .bind(serde_json::json!({"source": "seed", "employee": "empleado@test.com"}))
                .execute(pool)
                .await?;
                count += 1;
            }
        }

        Ok(count)
    }
}
