/* [259A-4c] Avisos de órdenes al cliente (extraído de email.rs).
 * Todas son non-fatal: ante fallo SMTP solo se loguea. El envío real
 * (`EmailService::send`) y los helpers viven en el hub `email`. */

use sqlx::PgPool;

use crate::repositories::EmailLogRepository;

use super::email::{EmailConfig, EmailService};

impl EmailService {
    /// Genera y envía email de confirmación de pedido.
    pub async fn send_order_confirmation(
        config: &EmailConfig,
        pool: &PgPool,
        to_email: &str,
        client_name: &str,
        order_number: i32,
        service_title: &str,
        plan_name: &str,
        price_display: &str,
    ) {
        let subject = format!("¡Pedido #{order_number} recibido! — Nakomi Studio");

        let html = super::email_templates::render_order_confirmation(
            client_name,
            order_number,
            service_title,
            plan_name,
            price_display,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            to_email,
            &subject,
            "order_confirmation",
            Some("order"),
            None,
            status,
            error_msg,
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email de confirmación orden #{order_number}: {e}");
        }
    }

    /* [311A-1] Email al cliente notificando que su orden fue completada.
     * Se dispara cuando el admin completa una orden. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_order_completed_client(
        config: &EmailConfig,
        pool: &PgPool,
        to_email: &str,
        client_name: &str,
        order_number: i32,
        _site_url: &str,
        order_id: uuid::Uuid,
    ) {
        let subject = format!("✅ Orden #{order_number} completada — Nakomi Studio");

        let html =
            super::email_templates::render_order_completed_client(client_name, order_number, "");

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            to_email,
            &subject,
            "order_completed_client",
            Some("order"),
            Some(order_id),
            status,
            error_msg,
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email orden completada a {to_email}: {e}");
        }
    }

    /* [311A-1] Email al cliente notificando que su orden fue cancelada.
     * Se dispara cuando el admin cancela una orden. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_order_cancelled_client(
        config: &EmailConfig,
        pool: &PgPool,
        to_email: &str,
        client_name: &str,
        order_number: i32,
        reason: &str,
        order_id: uuid::Uuid,
    ) {
        let subject = format!("❌ Orden #{order_number} cancelada — Nakomi Studio");

        let html = super::email_templates::render_order_cancelled_client(
            client_name,
            order_number,
            reason,
        );

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            to_email,
            &subject,
            "order_cancelled_client",
            Some("order"),
            Some(order_id),
            status,
            error_msg,
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email orden cancelada a {to_email}: {e}");
        }
    }

    /* [311A-1] Email al cliente notificando que una fase fue entregada.
     * Se dispara cuando el admin sube archivos de una fase. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_phase_delivered_client(
        config: &EmailConfig,
        pool: &PgPool,
        to_email: &str,
        client_name: &str,
        order_number: i32,
        phase_title: &str,
        site_url: &str,
        order_id: uuid::Uuid,
    ) {
        let subject = format!("📦 Fase entregada — Orden #{order_number} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel?seccion=ordenes&id={order_id}");

        let html = super::email_templates::render_phase_delivered_client(
            client_name,
            order_number,
            phase_title,
            &panel_link,
        );

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            to_email,
            &subject,
            "phase_delivered_client",
            Some("order"),
            Some(order_id),
            status,
            error_msg,
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email fase entregada a {to_email}: {e}");
        }
    }

    /* [311A-1] Email al cliente notificando que se reportó un problema en su orden.
     * Se dispara cuando el cliente o admin reporta un problema. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_problem_reported_client(
        config: &EmailConfig,
        pool: &PgPool,
        to_email: &str,
        client_name: &str,
        order_number: i32,
        _problem_title: &str,
        problem_description: &str,
        _site_url: &str,
        order_id: uuid::Uuid,
    ) {
        let subject = format!("⚠️ Problema reportado — Orden #{order_number} — Nakomi Studio");

        let html = super::email_templates::render_problem_reported_client(
            client_name,
            order_number,
            problem_description,
        );

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            to_email,
            &subject,
            "problem_reported_client",
            Some("order"),
            Some(order_id),
            status,
            error_msg,
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email problema reportado a {to_email}: {e}");
        }
    }
}
