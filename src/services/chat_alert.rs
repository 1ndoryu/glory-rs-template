/* [237A-7d] Orquestador de alertas de chat.
 * Garantiza transaccionalidad: mensaje + notificaciones in-app + outbox
 * email/WhatsApp se persisten en una sola transacción PostgreSQL.
 * El broadcast WS ocurre DESPUÉS del commit para evitar notificaciones fantasma.
 *
 * Solo mensajes de cliente/visitor (sender_type = "client") generan alertas.
 * Mensajes de IA, admin, employee NO disparan alertas externas. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AlertEventType, AlertPayload, ChatMessage, ChatSession, CreateNotification, NOTIF_NEW_MESSAGE,
};
use crate::repositories::{
    ChatAlertRepository, ChatRepository, NotificationRepository, UserRepository,
};

/* [257A-1] Las alertas nuevas son fail-closed: una variable ausente, vacía o
 * mal escrita nunca activa una ruta transaccional o una integración externa.
 * Se habilitan por canal, mediante canary, solo con true/1 explícito. */
fn enabled_flag_value(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "true" | "1")
}

fn feature_flag_enabled(key: &str) -> bool {
    std::env::var(key).is_ok_and(|value| enabled_flag_value(&value))
}

fn alerts_enabled() -> bool {
    feature_flag_enabled("CHAT_ALERT_CAPTURE_ENABLED")
}

fn email_delivery_enabled() -> bool {
    feature_flag_enabled("CHAT_EMAIL_DELIVERY_ENABLED")
}

fn whatsapp_delivery_enabled() -> bool {
    feature_flag_enabled("CHAT_WHATSAPP_DELIVERY_ENABLED")
}

/// Admin email override para canary/staging.
fn alert_email_override() -> Option<String> {
    std::env::var("CHAT_ALERT_EMAIL_OVERRIDE")
        .ok()
        .filter(|s| !s.is_empty())
}

/// `SITE_URL` para construir enlaces al panel.
fn site_url() -> String {
    std::env::var("SITE_URL").unwrap_or_else(|_| "https://nakomi.studio".to_string())
}

/// Guarda un mensaje y crea todas las alertas en una sola transacción.
/// Retorna el mensaje persistido.
///
/// Solo genera alertas para mensajes de cliente/visitor (`sender_type` == "client").
/// Los mensajes de IA, admin, employee se persisten sin outbox.
///
/// Después del commit, el caller debe hacer broadcast WS y `notification_hub`.
/* [01AA-4-f3s] Contexto de alerta derivado de la sesión (puro): preview,
 * etiqueta del remitente y URL del panel. Extraído de send_message_with_alerts. */
fn alert_context(
    session: &ChatSession,
    session_id: Uuid,
    content: &str,
) -> (String, String, String) {
    let preview: String = content.chars().take(80).collect();
    let sender_label = session
        .visitor_name
        .as_deref()
        .unwrap_or("Visitante")
        .to_string();
    let base = site_url();
    let panel_url = if let Some(order_id) = session.order_id {
        format!("{base}/panel?order={order_id}")
    } else {
        format!("{base}/panel?seccion=mensajes&chat={session_id}")
    };
    (preview, sender_label, panel_url)
}

/* [01AA-4-f3s] Outbox email (paso 3): override de staging > primer email admin.
 * Extraído de send_message_with_alerts. */
async fn enqueue_email_outbox(
    tx: &mut sqlx::PgConnection,
    msg: &ChatMessage,
    session_id: Uuid,
    sender_label: &str,
    preview: &str,
    panel_url: &str,
    admin_emails: &[String],
) -> Result<(), AppError> {
    /* Resolver email del admin: override de staging > primer email admin de BD */
    let email_recipient = alert_email_override().or_else(|| admin_emails.first().cloned());

    if let Some(to_email) = email_recipient {
        let idempotency_key = format!("chat:{}:email:{}", msg.id, to_email);
        let payload = serde_json::to_value(AlertPayload {
            message_id: msg.id,
            session_id,
            sender_label: sender_label.to_string(),
            preview: preview.to_string(),
            panel_url: panel_url.to_string(),
            occurred_at: msg.created_at,
        })
        .map_err(|error| AppError::Internal(format!("Error serializando alerta email: {error}")))?;

        ChatAlertRepository::insert_tx(
            tx,
            &idempotency_key,
            AlertEventType::ClientMessage.as_str(),
            "email",
            &to_email,
            Some("chat_message"),
            Some(msg.id),
            &payload,
        )
        .await?;
    }
    Ok(())
}

/* [01AA-4-f3s] Outbox WhatsApp (paso 4). Extraído de send_message_with_alerts. */
async fn enqueue_whatsapp_outbox(
    tx: &mut sqlx::PgConnection,
    msg: &ChatMessage,
    session_id: Uuid,
    sender_label: &str,
    preview: &str,
    panel_url: &str,
) -> Result<(), AppError> {
    let idempotency_key = format!("chat:{}:whatsapp:admin", msg.id);
    let payload = serde_json::to_value(AlertPayload {
        message_id: msg.id,
        session_id,
        sender_label: sender_label.to_string(),
        preview: preview.to_string(),
        panel_url: panel_url.to_string(),
        occurred_at: msg.created_at,
    })
    .map_err(|error| AppError::Internal(format!("Error serializando alerta WhatsApp: {error}")))?;

    ChatAlertRepository::insert_tx(
        tx,
        &idempotency_key,
        AlertEventType::ClientMessage.as_str(),
        "whatsapp",
        "admin",
        Some("chat_message"),
        Some(msg.id),
        &payload,
    )
    .await?;
    Ok(())
}

pub async fn send_message_with_alerts(
    pool: &PgPool,
    session_id: Uuid,
    sender_type: &str,
    sender_id: Option<&str>,
    content: &str,
) -> Result<ChatMessage, AppError> {
    /* Mensajes que no son de cliente: persistir sin alertas */
    if sender_type != "client" || !alerts_enabled() {
        return ChatRepository::save_message(pool, session_id, sender_type, sender_id, content)
            .await
            .map_err(|e| AppError::Internal(format!("Error guardando mensaje: {e}")));
    }

    /* Pre-fetch datos necesarios fuera de la transacción (son estables) */
    let admin_ids = UserRepository::admin_ids(pool).await?;
    if admin_ids.is_empty() {
        return ChatRepository::save_message(pool, session_id, sender_type, sender_id, content)
            .await
            .map_err(|e| AppError::Internal(format!("Error guardando mensaje: {e}")));
    }

    /* Pre-fetch admin emails para la outbox (evita query dentro de TX) */
    let admin_emails = UserRepository::admin_emails(pool).await?;

    let session = ChatRepository::find_session_by_id(pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Sesión no encontrada".into()))?;

    /* ===== Transacción: mensaje + notificaciones + outbox ===== */
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("Error iniciando transacción de alertas: {e}")))?;

    /* 1. Persistir mensaje ([279A-4] macro compile-time verificado en nakomi_dev) */
    let msg = sqlx::query_as!(
        crate::models::ChatMessage,
        r#"INSERT INTO chat_messages (session_id, sender_type, sender_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING id, session_id, sender_type, sender_id, content, created_at,
                  message_type, metadata, sequence_num"#,
        session_id,
        sender_type,
        sender_id,
        content
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Error guardando mensaje: {e}")))?;

    let (preview, sender_label, panel_url) = alert_context(&session, session_id, content);

    /* 2. Notificaciones in-app para cada admin (deduplicadas por constraint) */
    for &admin_id in &admin_ids {
        let notif = CreateNotification {
            user_id: admin_id,
            notification_type: NOTIF_NEW_MESSAGE.to_string(),
            title: format!("💬 {sender_label}: {preview}"),
            body: Some(preview.clone()),
            link: Some(panel_url.clone()),
            reference_type: Some("chat_session".to_string()),
            reference_id: Some(session_id),
        };
        NotificationRepository::create_tx(&mut tx, &notif).await?;
    }

    /* 3. Outbox email */
    if email_delivery_enabled() {
        enqueue_email_outbox(
            &mut tx,
            &msg,
            session_id,
            &sender_label,
            &preview,
            &panel_url,
            &admin_emails,
        )
        .await?;
    }

    /* 4. Outbox WhatsApp */
    if whatsapp_delivery_enabled() && std::env::var("GLORY_ALERT_GATEWAY_URL").is_ok() {
        enqueue_whatsapp_outbox(
            &mut tx,
            &msg,
            session_id,
            &sender_label,
            &preview,
            &panel_url,
        )
        .await?;
    }

    /* 5. Commit atómico: se aplica completo o se revierte */
    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("Error en commit de alertas: {e}")))?;

    tracing::info!(
        %session_id,
        message_id = %msg.id,
        admins = admin_ids.len(),
        "Mensaje de cliente persistido con alertas"
    );

    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::enabled_flag_value;

    #[test]
    fn feature_flags_only_accept_explicit_true_values() {
        for enabled in ["true", "TRUE", " 1 "] {
            assert!(
                enabled_flag_value(enabled),
                "{enabled} debe activar el flag"
            );
        }
        for disabled in ["", "false", "0", "yes", "enabled", "tru"] {
            assert!(
                !enabled_flag_value(disabled),
                "{disabled} no debe activar el flag"
            );
        }
    }
}
