/* [154A-15c] Servicio de email con SMTP (lettre).
 * Configuración vía env vars: SMTP_HOST o GLORY_SMTP_HOST, SMTP_PORT o GLORY_SMTP_PORT,
 * SMTP_USER o GLORY_SMTP_USER, SMTP_PASS o GLORY_SMTP_PASSWORD, SMTP_FROM.
 * Se aceptan ambos prefijos para compatibilidad con .env local (GLORY_SMTP_*)
 * y posibles configuraciones legacy (SMTP_*).
 * Non-fatal: si SMTP no está configurado, los emails se loguean y se omiten. */

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use sqlx::PgPool;

use crate::repositories::EmailLogRepository;

#[derive(Clone)]
pub struct EmailConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub from_name: String,
    pub from_email: String,
    pub bcc_email: Option<String>,
}

impl EmailConfig {
    /// Intenta crear config desde env vars. Retorna None si faltan variables.
    /// Acepta `SMTP_*` y `GLORY_SMTP_*` como nombres de variables (compat local/prod).
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("SMTP_HOST")
            .or_else(|_| std::env::var("GLORY_SMTP_HOST"))
            .ok()?;
        let user = std::env::var("SMTP_USER")
            .or_else(|_| std::env::var("GLORY_SMTP_USER"))
            .ok()?;
        let pass = std::env::var("SMTP_PASS")
            .or_else(|_| std::env::var("GLORY_SMTP_PASSWORD"))
            .ok()?;
        let from_email = std::env::var("SMTP_FROM").unwrap_or_else(|_| user.clone());
        let from_name =
            std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Nakomi Studio".to_string());
        let port = std::env::var("SMTP_PORT")
            .or_else(|_| std::env::var("GLORY_SMTP_PORT"))
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587);
        let bcc_email = std::env::var("SMTP_BCC").ok();

        Some(Self {
            host,
            port,
            user,
            pass,
            from_name,
            from_email,
            bcc_email,
        })
    }
}

pub struct EmailService;

impl EmailService {
    /// Envía un email HTML. Non-fatal: loguea error si falla.
    pub async fn send(
        config: &EmailConfig,
        to_email: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        let from = format!("{} <{}>", config.from_name, config.from_email);
        /* [311A-1] BCC configurable via SMTP_BCC para que el admin reciba copia
         * de TODO correo enviado desde la plataforma. Non-fatal: si la dirección
         * es inválida o no está configurada, el email se envía sin BCC. */
        let mut builder = Message::builder()
            .from(from.parse().map_err(|e| format!("From inválido: {e}"))?)
            .to(to_email.parse().map_err(|e| format!("To inválido: {e}"))?)
            .subject(subject)
            .header(ContentType::TEXT_HTML);
        if let Some(ref bcc_email) = config.bcc_email {
            if let Ok(bcc_addr) = bcc_email.parse() {
                builder = builder.bcc(bcc_addr);
            }
        }
        let email = builder
            .body(html_body.to_string())
            .map_err(|e| format!("Error construyendo email: {e}"))?;

        let creds = Credentials::new(config.user.clone(), config.pass.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
            .map_err(|e| format!("Error conectando SMTP: {e}"))?
            .port(config.port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| format!("Error enviando email: {e}"))?;

        Ok(())
    }

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
            client_name, order_number, service_title, plan_name, price_display,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, to_email, &subject, "order_confirmation",
            Some("order"), None, status, error_msg,
        ).await {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email de confirmación orden #{order_number}: {e}");
        }
    }
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[allow(dead_code)]
pub(crate) fn recipient_label(display_name: Option<&str>, email: &str) -> String {
    let raw = display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(email);
    html_escape(raw)
}

pub(crate) fn format_usd_cents(amount_cents: i32) -> String {
    format!("${:.2} USD", f64::from(amount_cents) / 100.0)
}

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
            client_name, client_email, order_number, service_title, plan_name, price_display, payment_mode, &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "new_order_admin",
                Some("order"), Some(order_id), status, error_msg,
            ).await {
                tracing::warn!("Error registrando email_log: {log_err}");
            }

            if let Err(e) = result {
                tracing::error!("Error enviando email nueva orden admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!("Email nueva orden #{order_number} enviado a {} admins", admin_emails.len());
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

        let html = super::email_templates::render_escalation(
            visitor_name, &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "escalation",
                Some("chat_session"), Some(session_id), status, error_msg,
            ).await {
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
            client_name, order_number, amount_display, &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "payment_received_admin",
                Some("order"), Some(order_id), status, error_msg,
            ).await {
                tracing::warn!("Error registrando email_log: {log_err}");
            }

            if let Err(e) = result {
                tracing::error!("Error enviando email pago recibido admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!("Email pago recibido orden #{order_number} enviado a {} admins", admin_emails.len());
        }
    }

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
            client_email, &amount_display, "",
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, client_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, client_email, &subject, "chat_invoice_paid_client",
            Some("chat_invoice"), None, status, error_msg,
        ).await {
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
            client_email, &amount_display, &session_id.to_string(), &panel_link,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "chat_invoice_paid_admin",
                Some("chat_session"), Some(session_id), status, error_msg,
            ).await {
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
        monthly_price_cents: i32,
    ) {
        let subject = format!("VPS pendiente de aprobación: {tier_name} — Nakomi Studio");
        let amount_display = format_usd_cents(monthly_price_cents);

        let html = super::email_templates::render_vps_pending_approval(
            client_email, tier_name, "", &amount_display,
        );

        for email in admin_emails {
            /* [311A-1] Logging individual por admin para trazabilidad. */
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);

            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "vps_pending_approval",
                Some("vps"), None, status, error_msg,
            ).await {
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
            client_email, tier_name, ip, username, password,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, client_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, client_email, &subject, "vps_approved",
            Some("vps"), None, status, error_msg,
        ).await {
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

        let html = super::email_templates::render_vps_rejected(
            client_email, tier_name, reason,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, client_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, client_email, &subject, "vps_rejected",
            Some("vps"), None, status, error_msg,
        ).await {
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
            old_email, new_email, recipient_name,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, new_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, new_email, subject, "profile_email_changed_new",
            Some("user"), None, status, error_msg,
        ).await {
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
            old_email, new_email, recipient_name,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, old_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, old_email, subject, "profile_email_changed_old",
            Some("user"), None, status, error_msg,
        ).await {
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

        let html = super::email_templates::render_profile_password_changed(
            recipient_name,
        );

        /* [311A-1] Logging del envío en email_logs para trazabilidad. */
        let result = Self::send(config, to_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, to_email, subject, "profile_password_changed",
            Some("user"), None, status, error_msg,
        ).await {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        match result {
            Ok(()) => tracing::info!("Email de cambio de contraseña enviado a {to_email}"),
            Err(error) => tracing::error!(
                "Error enviando email de cambio de contraseña a {to_email}: {error}"
            ),
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

        let html = super::email_templates::render_order_completed_client(
            client_name, order_number, "",
        );

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, to_email, &subject, "order_completed_client",
            Some("order"), Some(order_id), status, error_msg,
        ).await {
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
            client_name, order_number, reason,
        );

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, to_email, &subject, "order_cancelled_client",
            Some("order"), Some(order_id), status, error_msg,
        ).await {
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
            client_name, order_number, phase_title, &panel_link,
        );

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, to_email, &subject, "phase_delivered_client",
            Some("order"), Some(order_id), status, error_msg,
        ).await {
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
            client_name, order_number, problem_description,
        );

        let result = Self::send(config, to_email, &subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, to_email, &subject, "problem_reported_client",
            Some("order"), Some(order_id), status, error_msg,
        ).await {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        if let Err(e) = result {
            tracing::error!("Error enviando email problema reportado a {to_email}: {e}");
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
        let subject = format!("✅ Orden #{order_number} completada — {client_name} — Nakomi Studio");
        let panel_link = format!("{site_url}/panel?seccion=ordenes&id={order_id}");

        let html = super::email_templates::render_order_completed_admin(
            client_name, order_number, "", &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "order_completed_admin",
                Some("order"), Some(order_id), status, error_msg,
            ).await {
                tracing::warn!("Error registrando email_log: {log_err}");
            }
            if let Err(e) = result {
                tracing::error!("Error enviando email orden completada admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!("Email orden completada #{order_number} enviado a {} admins", admin_emails.len());
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
            client_name, order_number, reason, &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "order_cancelled_admin",
                Some("order"), Some(order_id), status, error_msg,
            ).await {
                tracing::warn!("Error registrando email_log: {log_err}");
            }
            if let Err(e) = result {
                tracing::error!("Error enviando email orden cancelada admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!("Email orden cancelada #{order_number} enviado a {} admins", admin_emails.len());
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
            client_name, order_number, problem_description, &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "problem_reported_admin",
                Some("order"), Some(order_id), status, error_msg,
            ).await {
                tracing::warn!("Error registrando email_log: {log_err}");
            }
            if let Err(e) = result {
                tracing::error!("Error enviando email problema reportado admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!("Email problema reportado orden #{order_number} enviado a {} admins", admin_emails.len());
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
            client_name, order_number, amount_display, reason, &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "refund_requested_admin",
                Some("refund"), Some(refund_id), status, error_msg,
            ).await {
                tracing::warn!("Error registrando email_log: {log_err}");
            }
            if let Err(e) = result {
                tracing::error!("Error enviando email reembolso solicitado admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!("Email reembolso solicitado orden #{order_number} enviado a {} admins", admin_emails.len());
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
            user_name, user_email, &panel_link,
        );

        for email in admin_emails {
            let result = Self::send(config, email, &subject, &html).await;
            let status = if result.is_ok() { "sent" } else { "failed" };
            let error_msg = result.as_ref().err().map(String::as_str);
            if let Err(log_err) = EmailLogRepository::insert(
                pool, email, &subject, "new_user_registered_admin",
                Some("user"), Some(user_id), status, error_msg,
            ).await {
                tracing::warn!("Error registrando email_log: {log_err}");
            }
            if let Err(e) = result {
                tracing::error!("Error enviando email nuevo usuario admin a {email}: {e}");
            }
        }
        if !admin_emails.is_empty() {
            tracing::info!("Email nuevo usuario registrado ({user_email}) enviado a {} admins", admin_emails.len());
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
    ) {
        let subject = "Continúa tu conversación — Nakomi Studio";

        let html = super::email_templates::render_chat_continuation(
            visitor_name,
            continuation_url,
        );

        let result = Self::send(config, to_email, subject, &html).await;
        let status = if result.is_ok() { "sent" } else { "failed" };
        let error_msg = result.as_ref().err().map(String::as_str);

        if let Err(log_err) = EmailLogRepository::insert(
            pool, to_email, subject, "chat_continuation",
            Some("chat_session"), Some(session_id), status, error_msg,
        ).await {
            tracing::warn!("Error registrando email_log: {log_err}");
        }

        match result {
            Ok(()) => {
                tracing::info!(%session_id, "Email de continuación enviado a visitante");
            }
            Err(error) => {
                tracing::error!(%session_id, "Error enviando email de continuación: {error}");
            }
        }
    }
}
