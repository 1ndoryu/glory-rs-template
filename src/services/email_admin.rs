/* [259A-4c] Avisos a administradores (extraído de email.rs).
 * Todas son non-fatal: ante fallo SMTP solo se loguea. El envío real
 * (`EmailService::send`) y los helpers viven en el hub `email`. */

use sqlx::PgPool;

use crate::repositories::EmailLogRepository;

use super::email::{EmailConfig, EmailService};

/* [311A-1] Envía email a todos los admins activos notificando una nueva orden.
 * Se dispara desde create_order() en handlers/orders.rs.
 * Non-fatal: si falla, solo se loguea. */
impl EmailService {
    #[allow(clippy::too_many_arguments)]
    pub async fn send_new_order_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        client_email: &str,
        client_name: &str,
        order_number: i32,
        service_title: &str,
        plan_name: &str,
        price_display: &str,
        payment_mode: &str,
        order_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("🆕 Nueva orden #{order_number} — {client_name} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel?seccion=ordenes&id={order_id}");

        let html = super::email_templates::render_new_order_admin(
            client_name,
            client_email,
            order_number,
            service_title,
            plan_name,
            price_display,
            payment_mode,
            &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "new_order_admin",
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
                tracing::error!("Error enviando email nueva orden admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email nueva orden #{order_number} enviado a {} admins",
                admin_emails.len()
            );
        }
    }

    /* [114A-8] Envía email de escalación a todos los admins activos.
     * Se dispara cuando la IA detecta que un visitante necesita asistencia humana.
     * Non-fatal: si SMTP no está configurado o falla, solo se loguea. */
    pub async fn send_escalation_emails(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        visitor_name: &str,
        session_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("⚠ Escalación: {visitor_name} necesita ayuda — Nakomi Studio");
        let panel_link = format!("{site_url}/panel/chat?session={session_id}");

        let html = super::email_templates::render_escalation(visitor_name, &panel_link);

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "escalation",
                Some("chat_session"),
                Some(session_id),
                status,
                error_msg,
            )
            .await
            {
                tracing::warn!("Error registrando email_log: {log_err}");
            }

            if let Err(e) = result {
                tracing::error!("Error enviando email escalación a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email de escalación enviado a {} admins para sesión {session_id}",
                admin_emails.len()
            );
        }
    }

    /* [311A-1] Email a todos los admins activos notificando que un pago de orden fue recibido.
     * Se dispara desde stripe_webhook() en handlers/payments.rs.
     * Non-fatal: si falla, solo se loguea. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_payment_received_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        _client_email: &str,
        client_name: &str,
        order_number: i32,
        amount_display: &str,
        order_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("💰 Pago recibido — Orden #{order_number} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel/orders/{order_id}");

        let html = super::email_templates::render_payment_received_admin(
            client_name,
            order_number,
            amount_display,
            &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "payment_received_admin",
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
                tracing::error!("Error enviando email pago recibido admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email pago recibido orden #{order_number} enviado a {} admins",
                admin_emails.len()
            );
        }
    }

    /* [011A-1] Email a admins notificando que una orden fue completada.
     * Se dispara cuando el cliente aprueba la última fase. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_order_completed_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        client_name: &str,
        _client_email: &str,
        order_number: i32,
        order_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject =
            format!("✅ Orden #{order_number} completada — {client_name} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel?seccion=ordenes&id={order_id}");

        let html = super::email_templates::render_order_completed_admin(
            client_name,
            order_number,
            "",
            &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "order_completed_admin",
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
                tracing::error!("Error enviando email orden completada admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email orden completada #{order_number} enviado a {} admins",
                admin_emails.len()
            );
        }
    }

    /* [011A-1] Email a admins notificando que una orden fue cancelada.
     * Se dispara cuando el admin cancela una orden. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_order_cancelled_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        client_name: &str,
        _client_email: &str,
        order_number: i32,
        reason: &str,
        order_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("❌ Orden #{order_number} cancelada — {client_name} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel?seccion=ordenes&id={order_id}");

        let html = super::email_templates::render_order_cancelled_admin(
            client_name,
            order_number,
            reason,
            &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "order_cancelled_admin",
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
                tracing::error!("Error enviando email orden cancelada admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email orden cancelada #{order_number} enviado a {} admins",
                admin_emails.len()
            );
        }
    }

    /* [011A-1] Email a admins notificando que se reportó un problema.
     * Se dispara cuando un cliente o admin reporta un problema en una orden. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_problem_reported_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        client_name: &str,
        _client_email: &str,
        order_number: i32,
        _problem_title: &str,
        problem_description: &str,
        order_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("⚠️ Problema reportado — Orden #{order_number} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel?seccion=ordenes&id={order_id}");

        let html = super::email_templates::render_problem_reported_admin(
            client_name,
            order_number,
            problem_description,
            &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "problem_reported_admin",
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
                tracing::error!("Error enviando email problema reportado admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email problema reportado orden #{order_number} enviado a {} admins",
                admin_emails.len()
            );
        }
    }

    /* [011A-1] Email a admins notificando solicitud de reembolso.
     * Se dispara cuando un cliente solicita un reembolso. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_refund_requested_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        client_name: &str,
        _client_email: &str,
        order_number: i32,
        amount_display: &str,
        reason: &str,
        refund_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("🔄 Reembolso solicitado — Orden #{order_number} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel?seccion=reembolsos&id={refund_id}");

        let html = super::email_templates::render_refund_requested_admin(
            client_name,
            order_number,
            amount_display,
            reason,
            &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "refund_requested_admin",
                Some("refund"),
                Some(refund_id),
                status,
                error_msg,
            )
            .await
            {
                tracing::warn!("Error registrando email_log: {log_err}");
            }
            if let Err(e) = result {
                tracing::error!("Error enviando email reembolso solicitado admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email reembolso solicitado orden #{order_number} enviado a {} admins",
                admin_emails.len()
            );
        }
    }

    /* [011A-1] Email a admins notificando que un nuevo usuario se registró.
     * Se dispara después de un registro exitoso. Non-fatal. */
    #[allow(clippy::too_many_arguments)]
    pub async fn send_new_user_registered_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        user_email: &str,
        user_name: &str,
        user_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("🆕 Nuevo usuario registrado — {user_email} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel");

        let html = super::email_templates::render_new_user_registered_admin(
            user_name,
            user_email,
            &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                email,
                &subject,
                "new_user_registered_admin",
                Some("user"),
                Some(user_id),
                status,
                error_msg,
            )
            .await
            {
                tracing::warn!("Error registrando email_log: {log_err}");
            }
            if let Err(e) = result {
                tracing::error!("Error enviando email nuevo usuario admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email nuevo usuario registrado ({user_email}) enviado a {} admins",
                admin_emails.len()
            );
        }
    }
}
