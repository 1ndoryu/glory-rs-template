/* [154A-15c] Servicio de email con SMTP (lettre).
 * Configuración vía env vars: SMTP_HOST o GLORY_SMTP_HOST, SMTP_PORT o GLORY_SMTP_PORT,
 * SMTP_USER o GLORY_SMTP_USER, SMTP_PASS o GLORY_SMTP_PASSWORD, SMTP_FROM.
 * Se aceptan ambos prefijos para compatibilidad con .env local (GLORY_SMTP_*)
 * y posibles configuraciones legacy (SMTP_*).
 * Non-fatal: si SMTP no está configurado, los emails se loguean y se omiten.
 *
 * [259A-4c] Hub: `EmailConfig`, transporte `send` y helpers `pub(crate)`
 * (`html_escape` lo usa `email_templates`; `format_usd_cents` lo usa `email_misc`).
 * Los 22 avisos viven en módulos hermanos que comparten este `impl` inherente
 * dentro del mismo crate (patrón ya usado en `hosting_runtime_*`):
 * - `email_orders`: confirmación, completada, cancelada, fase y problema (cliente).
 * - `email_admin`: nueva orden, escalación, pago, completada, cancelada,
 *   problema, reembolso y nuevo usuario (admins).
 * - `email_misc`: facturas de chat, VPS, perfil y continuación de chat. */

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

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
         * de cada correo enviado desde la plataforma. Non-fatal: si la dirección
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
