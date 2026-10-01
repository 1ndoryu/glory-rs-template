/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro:
 * display-name/AI/lookup queries usan runtime queries (ver comentarios inline). */
//! Metadata de orden: display info, nombres, IA intermediaria y lookups puntuales.

use sqlx::PgPool;
use uuid::Uuid;

use super::OrderRepository;

impl OrderRepository {
    /* ============================================================
    [035A-12] HELPERS DE DISPLAY: info legible de órdenes
    ============================================================ */

    /* [035A-12] Info legible de orden para client/chat: descripcion, servicio, plan y estado. */
    pub async fn get_order_display_info(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<(String, String, String, String)>, sqlx::Error> {
        let row = sqlx::query!(
            r#"SELECT o.project_description, s.name as service_name,
                      sp.name as plan_name, o.status::text as status
               FROM orders o
               JOIN services s ON s.id = o.service_id
               JOIN service_plans sp ON sp.id = o.plan_id
               WHERE o.id = $1"#,
            order_id,
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|r| {
            (
                r.project_description.unwrap_or_default(),
                r.service_name.unwrap_or_default(),
                r.plan_name.unwrap_or_default(),
                r.status.unwrap_or_default(),
            )
        }))
    }

    /* [035A-12] Nombre display de un empleado por id. */
    pub async fn get_employee_display_name(
        pool: &PgPool,
        employee_id: Uuid,
    ) -> Result<String, sqlx::Error> {
        let display: Option<String> = sqlx::query_scalar(
            "SELECT display_name FROM users WHERE id = $1",
        )
        .bind(employee_id)
        .fetch_optional(pool)
        .await?;
        Ok(display.unwrap_or_else(|| "Empleado".to_string()))
    }

    /* [035A-12] Nombre display de un cliente por id. */
    pub async fn get_client_display_name(
        pool: &PgPool,
        client_id: Uuid,
    ) -> Result<String, sqlx::Error> {
        let display: Option<String> = sqlx::query_scalar(
            "SELECT display_name FROM users WHERE id = $1",
        )
        .bind(client_id)
        .fetch_optional(pool)
        .await?;
        Ok(display.unwrap_or_else(|| "Cliente".to_string()))
    }

    /* [T-10] Activa/desactiva IA intermediaria para una orden */
    pub async fn toggle_ai_intermediary(
        pool: &PgPool,
        order_id: Uuid,
        enabled: bool,
    ) -> Result<crate::models::Order, sqlx::Error> {
        sqlx::query_as::<_, crate::models::Order>(
            "UPDATE orders SET ai_intermediary_enabled = $2, updated_at = NOW() \
             WHERE id = $1 \
             RETURNING id, order_number, client_id, service_id, plan_id, \
             payment_mode, base_price_cents, discount_percent, final_price_cents, currency, \
             status, assigned_employee_id, assigned_at, auto_assign_deadline, current_phase, \
               started_at, completed_at, cancelled_at, project_description, client_notes, internal_notes, \
             created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees",
        )
        .bind(order_id)
        .bind(enabled)
        .fetch_one(pool)
        .await
    }

    /* [T-10] Actualiza el resumen IA de una orden (reemplaza, no acumula) */
    pub async fn update_ai_summary(
        pool: &PgPool,
        order_id: Uuid,
        summary: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, crate::models::Order>(
            "UPDATE orders SET ai_summary = $2, updated_at = NOW() WHERE id = $1 \
             RETURNING id, order_number, client_id, service_id, plan_id, \
             payment_mode, base_price_cents, discount_percent, final_price_cents, currency, \
             status, assigned_employee_id, assigned_at, auto_assign_deadline, current_phase, \
               started_at, completed_at, cancelled_at, project_description, client_notes, internal_notes, \
             created_at, updated_at, ai_intermediary_enabled, ai_summary, open_to_employees",
        )
        .bind(order_id)
        .bind(summary)
        .fetch_one(pool)
        .await?;
        Ok(())
    }

    /* [124A-SENT-R1] order_number de una orden — usado en chat/rest.rs.
     * runtime query (sin macro). */
    pub async fn order_number_by_id(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<i32>, sqlx::Error> {
        sqlx::query_scalar::<_, i32>("SELECT order_number FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(pool)
            .await
    }

    /* [124A-SENT-R1] client_id de una orden — usado en chat/rest_messages.rs.
     * runtime query (sin macro). */
    pub async fn client_id_by_id(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>("SELECT client_id FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(pool)
            .await
    }

    /* [01AA-3] Conteo de órdenes de un cliente — usado en
     * handlers/orders.rs (first-order-discount). runtime query (sin macro). */
    pub async fn count_for_client(pool: &PgPool, client_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders WHERE client_id = $1")
            .bind(client_id)
            .fetch_one(pool)
            .await
    }

    /* [124A-SENT-R1] Participantes de una orden (client_id + assigned_employee_id).
     * Devuelve (client_id, assigned_employee_id) para verificar acceso en chat.
     * runtime query (sin macro). */
    pub async fn get_order_participants(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Option<(Uuid, Option<Uuid>)>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            client_id: Uuid,
            assigned_employee_id: Option<Uuid>,
        }
        let row = sqlx::query_as::<_, Row>(
            "SELECT client_id, assigned_employee_id FROM orders WHERE id = $1",
        )
        .bind(order_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|r| (r.client_id, r.assigned_employee_id)))
    }
}
