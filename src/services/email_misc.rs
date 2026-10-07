/* [259A-4c] Avisos de facturas de chat, VPS, perfil y continuación (extraído de email.rs).
 * Todas son non-fatal: ante fallo SMTP solo se loguea. El envío real
 * (`EmailService::send`) y los helpers viven en el hub `email`. */

use sqlx::PgPool;

use crate::repositories::{EmailLogRepository, NuevoEmailLog};

use super::email::{EmailConfig, EmailService};

impl EmailService {
    /* [124A-INV] Email al cliente notificando que su factura fue pagada y
     * que puede registrarse con el email de pago para acceder al panel. */
    pub async fn send_chat_invoice_paid_client(
        config: &EmailConfig,
        pool: &PgPool,
        client_email: &str,
        amount_usd: f64,
        _site_url: &str,
        _register_url: &str,
    ) {
        let subject = "Tu pago fue recibido — Nakomi Studio".to_string();
        let amount_display = format!("${:.2} USD", amount_usd);

        let html = super::email_templates::render_chat_invoice_paid_client(
            client_email,
            &amount_display,
            "",
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, client_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            NuevoEmailLog {
                to_email: client_email,
                subject: &subject,
                template: "chat_invoice_paid_client",
                reference_type: Some("chat_invoice"),
                reference_id: None,
                status,
                error_msg,
            },
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email pago factura chat a {client_email}: {e}");
        } else {
            tracing::info!("Email pago chat invoice enviado a {client_email}");
        }
    }

    /* [124A-INV] Email a admins notificando que una factura de chat fue pagada. */
    pub async fn send_chat_invoice_paid_admin(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        client_email: &str,
        amount_usd: f64,
        session_id: uuid::Uuid,
        site_url: &str,
    ) {
        let subject = format!("Factura pagada: {client_email} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel/chat?session={session_id}");
        let amount_display = format!("${:.2} USD", amount_usd);

        let html = super::email_templates::render_chat_invoice_paid_admin(
            client_email,
            &amount_display,
            &session_id.to_string(),
            &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                NuevoEmailLog {
                    to_email: email,
                    subject: &subject,
                    template: "chat_invoice_paid_admin",
                    reference_type: Some("chat_session"),
                    reference_id: Some(session_id),
                    status,
                    error_msg,
                },
            )
            .await
            {
                tracing::warn!("Error registrando email_log: {log_err}");
            }

            if let Err(e) = result {
                tracing::error!("Error enviando email pago chat admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!(
                "Email pago chat invoice enviado a {} admins para sesión {session_id}",
                admin_emails.len()
            );
        }
    }

    pub async fn send_vps_pending_approval(
        config: &EmailConfig,
        pool: &PgPool,
        admin_emails: &[String],
        client_email: &str,
        tier_name: &str,
        _monthly_price_cents: i32,
    ) {
        let subject = format!("VPS pendiente de aprobación: {tier_name} — Nakomi Studio");
        /* [07AA-7] F3h redujo el template a (cliente, plan, panel_link): antes se
         * pasaba el importe como link (hack). Link real al panel vía SITE_URL. */
        let site_url =
            std::env::var("SITE_URL").unwrap_or_else(|_| "https://nakomi.studio".to_string());
        let panel_link = format!("{}/panel", site_url.trim_end_matches('/'));

        let html = super::email_templates::render_vps_pending_approval(
            client_email,
            tier_name,
            &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool,
                NuevoEmailLog {
                    to_email: email,
                    subject: &subject,
                    template: "vps_pending_approval",
                    reference_type: Some("vps"),
                    reference_id: None,
                    status,
                    error_msg,
                },
            )
            .await
            {
                tracing::warn!("Error registrando email_log: {log_err}");
            }

            if let Err(error) = result {
                tracing::error!("Error enviando email VPS pendiente a {email}: {error}");
            }
        }
    }

    pub async fn send_vps_approved(
        config: &EmailConfig,
        pool: &PgPool,
        client_email: &str,
        tier_name: &str,
        public_ip: Option<&str>,
        username: &str,
        password: &str,
    ) {
        let subject = format!("Tu {tier_name} ya está activo — Nakomi Studio");
        let ip = public_ip.unwrap_or("");

        let html = super::email_templates::render_vps_approved(
            client_email,
            tier_name,
            ip,
            username,
            password,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, client_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            NuevoEmailLog {
                to_email: client_email,
                subject: &subject,
                template: "vps_approved",
                reference_type: Some("vps"),
                reference_id: None,
                status,
                error_msg,
            },
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(error) = result {
            tracing::error!("Error enviando email VPS aprobado a {client_email}: {error}");
        }
    }

    pub async fn send_vps_rejected(
        config: &EmailConfig,
        pool: &PgPool,
        client_email: &str,
        tier_name: &str,
        reason: &str,
    ) {
        let subject = format!("Tu solicitud de {tier_name} fue rechazada — Nakomi Studio");

        let html = super::email_templates::render_vps_rejected(client_email, tier_name, reason);

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, client_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            NuevoEmailLog {
                to_email: client_email,
                subject: &subject,
                template: "vps_rejected",
                reference_type: Some("vps"),
                reference_id: None,
                status,
                error_msg,
            },
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(error) = result {
            tracing::error!("Error enviando email VPS rechazado a {client_email}: {error}");
        }
    }

    /* [205A-2] Notificación informativa al correo nuevo tras cambiar el email desde perfil.
     * No verifica ownership; solo confirma que el cambio ya se aplicó y deja rastro en la bandeja. */
    pub async fn send_profile_email_changed_new_address(
        config: &EmailConfig,
        pool: &PgPool,
        new_email: &str,
        display_name: Option<&str>,
        old_email: &str,
    ) {
        let subject = "Tu correo de acceso fue actualizado — Nakomi Studio";
        let recipient_name = display_name.unwrap_or(new_email);

        let html = super::email_templates::render_profile_email_changed_new(
            old_email,
            new_email,
            recipient_name,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, new_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            NuevoEmailLog {
                to_email: new_email,
                subject,
                template: "profile_email_changed_new",
                reference_type: Some("user"),
                reference_id: None,
                status,
                error_msg,
            },
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        match result {
            Ok(()) => {
                tracing::info!("Email de confirmación de cambio de correo enviado a {new_email}");
            }
            Err(error) => {
                tracing::error!("Error enviando email de cambio de correo a {new_email}: {error}");
            }
        }
    }

    /* [205A-2] Notificación defensiva al correo anterior tras un cambio de email.
     * Sirve para alertar al usuario si no reconoce la modificación. */
    pub async fn send_profile_email_changed_old_address(
        config: &EmailConfig,
        pool: &PgPool,
        old_email: &str,
        display_name: Option<&str>,
        new_email: &str,
    ) {
        let subject = "Tu correo de acceso fue reemplazado — Nakomi Studio";
        let recipient_name = display_name.unwrap_or(old_email);

        let html = super::email_templates::render_profile_email_changed_old(
            old_email,
            new_email,
            recipient_name,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, old_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            NuevoEmailLog {
                to_email: old_email,
                subject,
                template: "profile_email_changed_old",
                reference_type: Some("user"),
                reference_id: None,
                status,
                error_msg,
            },
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        match result {
            Ok(()) => tracing::info!("Email de alerta por cambio de correo enviado a {old_email}"),
            Err(error) => {
                tracing::error!(
                    "Error enviando alerta por cambio de correo a {old_email}: {error}"
                );
            }
        }
    }

    /* [205A-2] Notificación informativa tras cambio de contraseña desde perfil.
     * Permite comprobar entrega SMTP y avisar al usuario si el cambio fue inesperado. */
    pub async fn send_profile_password_changed(
        config: &EmailConfig,
        pool: &PgPool,
        to_email: &str,
        display_name: Option<&str>,
    ) {
        let subject = "Tu contraseña fue actualizada — Nakomi Studio";
        let recipient_name = display_name.unwrap_or(to_email);

        let html = super::email_templates::render_profile_password_changed(recipient_name);

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, to_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            NuevoEmailLog {
                to_email,
                subject,
                template: "profile_password_changed",
                reference_type: Some("user"),
                reference_id: None,
                status,
                error_msg,
            },
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        match result {
            Ok(()) => tracing::info!("Email de cambio de contraseña enviado a {to_email}"),
            Err(error) => tracing::error!(
                "Error enviando email de cambio de contraseña a {to_email}: {error}"
            ),
        }
    }

    /* [237A-7j] Email de continuación de conversación al visitante.
     * Se envía cuando el visitante con email conocido se desconecta por más de 2 minutos.
     * Contiene un enlace firmado de un solo uso para reanudar la conversación. */
    pub async fn send_chat_continuation(
        config: &EmailConfig,
        pool: &PgPool,
        to_email: &str,
        visitor_name: &str,
        continuation_url: &str,
        session_id: uuid::Uuid,
    ) -> Result<(), String> {
        let subject = "Continúa tu conversación — Nakomi Studio";

        let html = super::email_templates::render_chat_continuation(visitor_name, continuation_url);

        let result = Self::send(config, to_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool,
            NuevoEmailLog {
                to_email,
                subject,
                template: "chat_continuation",
                reference_type: Some("chat_session"),
                reference_id: Some(session_id),
                status,
                error_msg,
            },
        )
        .await
        {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        match result {
            Ok(()) => {
                tracing::info!(%session_id, "Email de continuación enviado a visitante");
                Ok(())
            }
            Err(error) => {
                tracing::error!(%session_id, "Error enviando email de continuación: {error}");
                Err(error)
            }
        }
    }
}
