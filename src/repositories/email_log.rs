/* [311A-1] Repositorio de email_logs — trazabilidad de todos los correos enviados.
 * Cada vez que EmailService::send() se ejecuta, registra una fila aquí.
 * Non-fatal: si el INSERT falla, el email ya fue entregado. */

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/* [311A-1] Este repositorio usa query_as/query_scalar sin macro (sqlx::query_as::<_, T>)
 * en lugar de query_as! para evitar dependencia de tabla existente en compilación.
 * La tabla email_logs se crea vía migración, no por chequeo compile-time. */

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct EmailLogRow {
    pub id: Uuid,
    pub to_email: String,
    pub subject: String,
    pub template: String,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub status: String,
    pub error_msg: Option<String>,
    pub sent_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl EmailLogRow {
    /// Convierte el campo `template` en una etiqueta amigable para UI.
    pub fn template_label(&self) -> &'static str {
        match self.template.as_str() {
            "order_confirmation" => "Confirmación al cliente",
            "new_order_admin" => "Nueva orden (admin)",
            "payment_received_admin" => "Pago recibido (admin)",
            "order_completed_client" => "Orden completada (cliente)",
            "order_cancelled_client" => "Orden cancelada (cliente)",
            "phase_delivered_client" => "Fase entregada (cliente)",
            "problem_reported_client" => "Problema reportado (cliente)",
            "escalation" => "Escalación de chat",
            "chat_invoice_paid_client" => "Factura chat pagada (cliente)",
            "chat_invoice_paid_admin" => "Factura chat pagada (admin)",
            "vps_pending_approval" => "VPS pendiente (admin)",
            "vps_approved" => "VPS aprobado (cliente)",
            "vps_rejected" => "VPS rechazado (cliente)",
            "profile_email_changed_new" => "Email cambiado (nuevo)",
            "profile_email_changed_old" => "Email cambiado (anterior)",
            "profile_password_changed" => "Contraseña cambiada",
            _ => "Desconocido",
        }
    }

    /// Retorna clase CSS para el badge de estado.
    pub fn status_color(&self) -> &'static str {
        match self.status.as_str() {
            "sent" => "badgeExito",
            "failed" => "badgeError",
            _ => "badgeNeutral",
        }
    }
}

pub struct EmailLogRepository;

impl EmailLogRepository {
    /// Inserta un registro de correo enviado (non-fatal).
    pub async fn insert(
        pool: &PgPool,
        to_email: &str,
        subject: &str,
        template: &str,
        reference_type: Option<&str>,
        reference_id: Option<Uuid>,
        status: &str,
        error_msg: Option<&str>,
    ) -> Result<Uuid, sqlx::Error> {
        let rec = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO email_logs (to_email, subject, template, reference_type, reference_id, status, error_msg)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id"#,
        )
        .bind(to_email)
        .bind(subject)
        .bind(template)
        .bind(reference_type)
        .bind(reference_id)
        .bind(status)
        .bind(error_msg)
        .fetch_one(pool)
        .await?;
        Ok(rec)
    }

    /// Lista paginada con filtro opcional por template.
    pub async fn list(
        pool: &PgPool,
        template: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<EmailLogRow>, sqlx::Error> {
        let limit = std::cmp::min(limit.max(1), 100) as i64;

        if let Some(tpl) = template {
            sqlx::query_as::<_, EmailLogRow>(
                r#"SELECT id, to_email, subject, template, reference_type, reference_id,
                          status, error_msg, sent_at, created_at
                   FROM email_logs
                   WHERE template = $1
                   ORDER BY created_at DESC
                   LIMIT $2 OFFSET $3"#,
            )
            .bind(tpl)
            .bind(limit)
            .bind(offset as i64)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, EmailLogRow>(
                r#"SELECT id, to_email, subject, template, reference_type, reference_id,
                          status, error_msg, sent_at, created_at
                   FROM email_logs
                   ORDER BY created_at DESC
                   LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset as i64)
            .fetch_all(pool)
            .await
        }
    }

    /// Cuenta total con filtro opcional por template.
    pub async fn count(pool: &PgPool, template: Option<&str>) -> Result<i64, sqlx::Error> {
        if let Some(tpl) = template {
            sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM email_logs WHERE template = $1"#)
                .bind(tpl)
                .fetch_one(pool)
                .await
        } else {
            sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM email_logs"#)
                .fetch_one(pool)
                .await
        }
    }
}
