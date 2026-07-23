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
    AlertEventType, AlertPayload, ChatMessage, CreateNotification, NOTIF_NEW_MESSAGE,
};
use crate::repositories::{ChatAlertRepository, ChatRepository, NotificationRepository, UserRepository};

/* Feature flags: si están desactivados, no se crean outbox entries. */
fn alerts_enabled() -> bool {
    std::env::var("CHAT_ALERT_CAPTURE_ENABLED")
        .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
        .unwrap_or(true)
}

fn email_delivery_enabled() -> bool {
    std::env::var("CHAT_EMAIL_DELIVERY_ENABLED")
        .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
        .unwrap_or(true)
}

fn whatsapp_delivery_enabled() -> bool {
    std::env::var("CHAT_WHATSAPP_DELIVERY_ENABLED")
        .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
        .unwrap_or(true)
}

/// Admin email override para canary/staging.
fn alert_email_override() -> Option<String> {
    std::env::var("CHAT_ALERT_EMAIL_OVERRIDE").ok().filter(|s| !s.is_empty())
}

/// SITE_URL para construir enlaces al panel.
fn site_url() -> String {
    std::env::var("SITE_URL").unwrap_or_else(|_| "https://nakomi.studio".to_string())
}

/// Guarda un mensaje y crea todas las alertas en una sola transacción.
/// Retorna el mensaje persistido.
///
/// Solo genera alertas para mensajes de cliente/visitor (sender_type == "client").
/// Los mensajes de IA, admin, employee se persisten sin outbox.
///
/// Después del commit, el caller debe hacer broadcast WS y notification_hub.
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
    let admin_ids = UserRepository::admin_ids(pool).await.unwrap_or_default();
    if admin_ids.is_empty() {
        return ChatRepository::save_message(pool, session_id, sender_type, sender_id, content)
            .await
            .map_err(|e| AppError::Internal(format!("Error guardando mensaje: {e}")));
    }

    /* Pre-fetch admin emails para la outbox (evita query dentro de TX) */
    let admin_emails = UserRepository::admin_emails(pool).await.unwrap_or_default();

    let session = ChatRepository::find_session_by_id(pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Sesión no encontrada".into()))?;

    /* ===== Transacción: mensaje + notificaciones + outbox ===== */
    let mut tx = pool.begin().await.map_err(|e| {
        AppError::Internal(format!("Error iniciando transacción de alertas: {e}"))
    })?;

    /* 1. Persistir mensaje (runtime query: no depende de BD local para compilar) */
    let msg = sqlx::query_as::<_, crate::models::ChatMessage>(
        "INSERT INTO chat_messages (session_id, sender_type, sender_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING id, session_id, sender_type, sender_id, content, created_at,
                  message_type, metadata",
    )
    .bind(session_id)
    .bind(sender_type)
    .bind(sender_id)
    .bind(content)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Error guardando mensaje: {e}")))?;

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
        let _ = NotificationRepository::create_tx(&mut *tx, &notif).await;
    }

    /* 3. Outbox email */
    if email_delivery_enabled() {
        /* Resolver email del admin: override de staging > primer email admin de BD */
        let email_recipient = alert_email_override()
            .or_else(|| admin_emails.first().cloned());

        if let Some(to_email) = email_recipient {
            let idempotency_key = format!("chat:{}:email:{}", msg.id, to_email);
            let payload = serde_json::to_value(AlertPayload {
                message_id: msg.id,
                session_id,
                sender_label: sender_label.clone(),
                preview: preview.clone(),
                panel_url: panel_url.clone(),
                occurred_at: msg.created_at,
            })
            .unwrap_or_default();

            let _ = ChatAlertRepository::insert_tx(
                &mut *tx,
                &idempotency_key,
                AlertEventType::ClientMessage.as_str(),
                "email",
                &to_email,
                Some("chat_message"),
                Some(msg.id),
                &payload,
            )
            .await;
        }
    }

    /* 4. Outbox WhatsApp */
    if whatsapp_delivery_enabled() && std::env::var("GLORY_ALERT_GATEWAY_URL").is_ok() {
        let idempotency_key = format!("chat:{}:whatsapp:admin", msg.id);
        let payload = serde_json::to_value(AlertPayload {
            message_id: msg.id,
            session_id,
            sender_label: sender_label.clone(),
            preview: preview.clone(),
            panel_url: panel_url.clone(),
            occurred_at: msg.created_at,
        })
        .unwrap_or_default();

        let _ = ChatAlertRepository::insert_tx(
            &mut *tx,
            &idempotency_key,
            AlertEventType::ClientMessage.as_str(),
            "whatsapp",
            "admin",
            Some("chat_message"),
            Some(msg.id),
            &payload,
        )
        .await;
    }

    /* 5. Commit: todo o nada */
    tx.commit().await.map_err(|e| {
        AppError::Internal(format!("Error en commit de alertas: {e}"))
    })?;

    tracing::info!(
        %session_id,
        message_id = %msg.id,
        admins = admin_ids.len(),
        "Mensaje de cliente persistido con alertas"
    );

    Ok(msg)
}
