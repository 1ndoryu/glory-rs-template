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

    /* [07AA-7] Revert de F3f: la firma por order_id usaba `s.name`
     * (columna inexistente en services: solo title/slug) y ningún caller
     * la adoptó (11+8 callers siguen la forma vieja). Se restaura el
     * contrato original (service_id, plan_id) con queries con caché offline. */
    /// Obtiene título, slug del servicio y nombre del plan para un order
    pub async fn get_order_display_info(
        pool: &PgPool,
        service_id: Uuid,
        plan_id: Uuid,
    ) -> Result<(String, String, String), sqlx::Error> {
        let (service_title, service_slug): (String, String) = {
            let row = sqlx::query!(
                r#"SELECT title, slug FROM services WHERE id = $1"#,
                service_id,
            )
            .fetch_one(pool)
            .await?;
            (row.title, row.slug)
        };

        let plan_name: String =
            sqlx::query_scalar!(r#"SELECT name FROM service_plans WHERE id = $1"#, plan_id,)
                .fetch_one(pool)
                .await?;

        Ok((service_title, service_slug, plan_name))
    }

    /* [064A-30] Obtiene display_name del empleado asignado a una orden.
     * Retorna None si employee_id es None o si el usuario no tiene display_name.
     * Usa query_scalar sin macro para no depender del cache offline. */
    pub async fn get_employee_display_name(
        pool: &PgPool,
        employee_id: Option<Uuid>,
    ) -> Result<Option<String>, sqlx::Error> {
        let Some(eid) = employee_id else {
            return Ok(None);
        };
        let name: Option<String> =
            sqlx::query_scalar("SELECT display_name FROM users WHERE id = $1")
                .bind(eid)
                .fetch_optional(pool)
                .await?
                .flatten();
        Ok(name)
    }

    /* [074A-53] Obtener nombre del cliente para la respuesta de órdenes */
    pub async fn get_client_display_name(
        pool: &PgPool,
        client_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        let name: Option<String> =
            sqlx::query_scalar("SELECT display_name FROM users WHERE id = $1")
                .bind(client_id)
                .fetch_optional(pool)
                .await?
                .flatten();
        Ok(name)
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
