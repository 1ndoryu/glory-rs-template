/* [259A-4d] Escalacion y resumen de chat_timing: notificacion + email a admins
 * al escalar ([114A-8]), y resumen de contexto al cerrar sesion ([T-3]). */

use std::fmt::Write;

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateNotification, NOTIF_ESCALATION_NEEDED};
use crate::repositories::UserRepository;
use crate::services::{AiChatConfig, NotificationHub};

use super::ai_providers::{call_ai_api_with_options, ChatApiOptions};

/* [114A-8] Reutilizable: enviar notificación + email de escalación a todos los admins */
pub(crate) async fn send_escalation(
    pool: &PgPool,
    notification_hub: &NotificationHub,
    session_id: Uuid,
    visitor_name: Option<&str>,
    email_config: Option<&crate::services::EmailConfig>,
) {
    let name = visitor_name.unwrap_or("Visitante");

    /* [124A-ESC] Persistir is_escalated = true para que el panel muestre
     * el indicador al recargar sin depender solo del estado WS en memoria. */
    /* UPDATE fire-and-forget; resultado descartado con let _ = */
    // sentinel-disable-next-line sqlx-query-sin-macro
    let _ = sqlx::query(
        "UPDATE chat_sessions SET is_escalated = true, updated_at = NOW() WHERE id = $1",
    )
    .bind(session_id)
    .execute(pool)
    .await;

    if let Ok(admin_ids) = UserRepository::admin_ids(pool).await {
        if !admin_ids.is_empty() {
            let base = CreateNotification {
                user_id: Uuid::nil(),
                notification_type: NOTIF_ESCALATION_NEEDED.to_string(),
                title: format!("Escalación: {name} necesita ayuda"),
                body: Some(
                    "La IA detectó que se requiere intervención humana en la sesión de chat."
                        .to_string(),
                ),
                link: Some(format!("/admin/chat?session={session_id}")),
                reference_type: Some("chat_session".to_string()),
                reference_id: Some(session_id),
            };
            let _ = notification_hub.notify_many(&admin_ids, &base).await;
            tracing::info!(
                "Escalación enviada a {} admins para sesión {session_id}",
                admin_ids.len()
            );
        }
    }

    /* [114A-8] Email de escalación a admins */
    if let Some(cfg) = email_config {
        if let Ok(emails) = UserRepository::admin_emails(pool).await {
            if !emails.is_empty() {
                let site_url = std::env::var("SITE_URL")
                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                crate::services::EmailService::send_escalation_emails(
                    cfg, pool, &emails, name, session_id, &site_url,
                )
                .await;
            }
        }
    }
}

/* [T-3] Genera resumen de la conversación al cerrar sesión.
 * Usa modelo pequeño (llama-3.1-8b-instant) para resumir el historial
 * del visitante y lo guarda en visitor_profiles.context_summary.
 * Se ejecuta como tokio::spawn para no bloquear el cierre de WS. */
pub(crate) async fn generate_context_summary(
    pool: PgPool,
    config: AiChatConfig,
    http_client: reqwest::Client,
    session_id: Uuid,
    visitor_id: String,
) {
    if !config.is_configured() {
        return;
    }

    /* Obtener historial de la sesión */
    let messages =
        match crate::repositories::ChatRepository::list_messages(&pool, session_id, 50, 0).await {
            Ok(msgs) if !msgs.is_empty() => msgs,
            _ => return,
        };

    /* Construir transcript compacto */
    let mut transcript = String::new();
    for msg in &messages {
        /* [084A-46] Agente renombrado a Claudia */
        let role = if msg.sender_type == "ai" {
            "Claudia"
        } else {
            "Cliente"
        };
        let _ = writeln!(transcript, "{role}: {}", msg.content);
    }

    if transcript.len() < 100 {
        return;
    }

    let transcript_truncated = if transcript.len() > 3000 {
        &transcript[..3000]
    } else {
        &transcript
    };

    let Some(summary) = call_summary_api(&config, transcript_truncated, &http_client).await else {
        return;
    };

    /* Cargar resumen previo y concatenar (max 2000 chars total) */
    let existing =
        match crate::repositories::ChatRepository::find_visitor_profile(&pool, &visitor_id).await {
            Ok(Some(p)) => p.context_summary.unwrap_or_default(),
            _ => String::new(),
        };

    let final_summary = if existing.is_empty() {
        summary
    } else {
        let combined = format!("{existing}\n---\n{summary}");
        if combined.len() > 2000 {
            combined[combined.len() - 2000..].to_string()
        } else {
            combined
        }
    };

    if let Err(e) = crate::repositories::ChatRepository::update_context_summary(
        &pool,
        &visitor_id,
        &final_summary,
    )
    .await
    {
        tracing::warn!("Error guardando context summary para {visitor_id}: {e}");
    } else {
        tracing::info!("Context summary actualizado para visitor {visitor_id}");
    }
}

/* [T-3] Llama a la API de Groq con modelo ligero para generar resumen de sesión.
 * Retorna None si la API falla o el resumen está vacío. */
pub(crate) async fn call_summary_api(
    config: &AiChatConfig,
    transcript: &str,
    http_client: &reqwest::Client,
) -> Option<String> {
    let messages = [
        serde_json::json!({
            "role": "system",
            "content": "Genera un resumen conciso (máximo 500 caracteres) de esta conversación \
                 de chat de soporte. Incluye: qué necesitaba el cliente, qué servicios le interesaron, \
                 si se capturó email, si se generó factura, si quedó algo pendiente, y cualquier \
                 preferencia o dato relevante del cliente. Solo el resumen, sin formato especial."
        }),
        serde_json::json!({"role": "user", "content": transcript}),
    ];

    match call_ai_api_with_options(
        config,
        &messages,
        None,
        ChatApiOptions::terse(200),
        Some(http_client),
    )
    .await
    {
        Ok(json) => {
            let s = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        Err(e) => {
            tracing::warn!("Context summary API error: {e}");
            None
        }
    }
}
