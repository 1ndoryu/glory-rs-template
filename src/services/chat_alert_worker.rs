/* [237A-7d] Worker de alertas de chat: procesa la outbox periódicamente.
 * Reclama filas pending con FOR UPDATE SKIP LOCKED, envía email (SMTP)
 * o WhatsApp (gateway firmado), y actualiza el estado.
 *
 * Ejecuta como tokio::spawn desde main.rs, igual que otros background tasks.
 * Tick cada 5 segundos. Se detiene cuando el proceso termina. */

use std::time::Duration;

use sqlx::PgPool;

use crate::models::ChatAlertOutbox;
use crate::repositories::{ChatAlertRepository, EmailLogRepository};

const TICK_INTERVAL: Duration = Duration::from_secs(5);
const CLAIM_LIMIT: i64 = 20;
const SMTP_TIMEOUT: Duration = Duration::from_secs(30);
const GATEWAY_TIMEOUT: Duration = Duration::from_secs(10);

/// Punto de entrada del worker. Se spawnea desde main.rs.
pub async fn run_chat_alert_worker(
    pool: PgPool,
    email_config: Option<crate::services::EmailConfig>,
    http_client: reqwest::Client,
) {
    tracing::info!("[chat-alert-worker] Iniciando worker de alertas de chat");

    loop {
        /* Recuperar processing abandonados antes de reclamar nuevos */
        recover_stale_processing(&pool).await;

        /* Reclamar un lote */
        let batch = match ChatAlertRepository::claim(&pool, CLAIM_LIMIT).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("[chat-alert-worker] Error reclamando outbox: {e}");
                tokio::time::sleep(TICK_INTERVAL).await;
                continue;
            }
        };

        if batch.is_empty() {
            tokio::time::sleep(TICK_INTERVAL).await;
            continue;
        }

        let batch_len = batch.len();
        tracing::debug!("[chat-alert-worker] Procesando lote de {batch_len} alertas");

        for entry in &batch {
            process_entry(&pool, entry, &email_config, &http_client).await;
        }

        /* Emitir resumen del lote */
        log_batch_summary(&pool).await;

        tokio::time::sleep(TICK_INTERVAL).await;
    }
}

/// Recupera filas 'processing' abandonadas (>5 min) a 'pending'.
async fn recover_stale_processing(pool: &PgPool) {
    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::seconds(300);
    let result = sqlx::query(
        "UPDATE chat_alert_outbox SET status = 'pending', locked_at = NULL, updated_at = $1
         WHERE status = 'processing' AND locked_at < $2",
    )
    .bind(now)
    .bind(cutoff)
    .execute(pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::info!(
                "[chat-alert-worker] Recuperadas {} filas processing abandonadas",
                r.rows_affected()
            );
        }
        Err(e) => {
            tracing::error!("[chat-alert-worker] Error recuperando stale: {e}");
        }
        _ => {}
    }
}

/// Procesa una entrada individual de la outbox.
async fn process_entry(
    pool: &PgPool,
    entry: &ChatAlertOutbox,
    email_config: &Option<crate::services::EmailConfig>,
    http_client: &reqwest::Client,
) {
    match entry.channel.as_str() {
        "email" => process_email(pool, entry, email_config).await,
        "whatsapp" => process_whatsapp(pool, entry, http_client).await,
        other => {
            let error = format!("Canal desconocido: {other}");
            let _ = ChatAlertRepository::mark_dead(pool, entry.id, &error).await;
        }
    }
}

/// Envía email vía SMTP y marca el resultado.
async fn process_email(
    pool: &PgPool,
    entry: &ChatAlertOutbox,
    email_config: &Option<crate::services::EmailConfig>,
) {
    if entry.event_type == "chat.continuation" {
        process_continuation_email(pool, entry, email_config).await;
        return;
    }
    let Some(config) = email_config else {
        let _ = ChatAlertRepository::mark_dead(pool, entry.id, "SMTP no configurado").await;
        return;
    };

    /* Extraer datos del payload */
    let payload: crate::models::AlertPayload = match serde_json::from_value(entry.payload.clone()) {
        Ok(p) => p,
        Err(e) => {
            let _ =
                ChatAlertRepository::mark_dead(pool, entry.id, &format!("Payload inválido: {e}"))
                    .await;
            return;
        }
    };

    let subject = format!(
        "💬 {} necesita atención — Nakomi Studio",
        payload.sender_label
    );
    let html = crate::services::email_templates::render_chat_client_message_admin(
        &payload.sender_label,
        &payload.preview,
        &payload.panel_url,
    );

    let result = tokio::time::timeout(
        SMTP_TIMEOUT,
        crate::services::EmailService::send(config, &entry.recipient, &subject, &html),
    )
    .await;

    match result {
        Ok(Ok(())) => {
            let _ = ChatAlertRepository::mark_sent(pool, entry.id).await;
            let _ = EmailLogRepository::insert(
                pool,
                &entry.recipient,
                &subject,
                "chat_client_message",
                Some("chat_message"),
                entry.reference_id,
                "sent",
                None,
            )
            .await;
            tracing::debug!(
                entry_id = %entry.id,
                recipient = %entry.recipient,
                "Email de alerta enviado"
            );
        }
        Ok(Err(e)) => {
            let _ = ChatAlertRepository::mark_retry(pool, entry.id, &e, entry.attempts).await;
            let _ = EmailLogRepository::insert(
                pool,
                &entry.recipient,
                &subject,
                "chat_client_message",
                Some("chat_message"),
                entry.reference_id,
                "failed",
                Some(&e),
            )
            .await;
            tracing::warn!(
                entry_id = %entry.id,
                attempt = entry.attempts,
                "Email de alerta falló: {e}"
            );
        }
        Err(_) => {
            let error = "Timeout SMTP (30s)";
            let _ = ChatAlertRepository::mark_retry(pool, entry.id, error, entry.attempts).await;
            tracing::warn!(
                entry_id = %entry.id,
                attempt = entry.attempts,
                "Email de alerta timeout"
            );
        }
    }
}

/* [267A-3] El token en claro nace dentro del worker y nunca se persiste ni se
 * registra. La presencia/época se revalida justo antes del SMTP. */
async fn process_continuation_email(
    pool: &PgPool,
    entry: &ChatAlertOutbox,
    email_config: &Option<crate::services::EmailConfig>,
) {
    let Some(config) = email_config else {
        let _ = ChatAlertRepository::mark_dead(pool, entry.id, "SMTP no configurado").await;
        return;
    };
    let payload: crate::models::ContinuationAlertPayload =
        match serde_json::from_value(entry.payload.clone()) {
            Ok(payload) => payload,
            Err(error) => {
                let _ = ChatAlertRepository::mark_dead(
                    pool,
                    entry.id,
                    &format!("Payload de continuación inválido: {error}"),
                )
                .await;
                return;
            }
        };
    match crate::repositories::continuation_token::is_disconnect_cycle_current(
        pool,
        payload.session_id,
        payload.disconnect_epoch,
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => {
            let _ = ChatAlertRepository::mark_cancelled(
                pool,
                entry.id,
                "visitor_reconnected_or_consent_revoked",
            )
            .await;
            return;
        }
        Err(error) => {
            let _ = ChatAlertRepository::mark_retry(
                pool,
                entry.id,
                &format!("Error revalidando continuación: {error}"),
                entry.attempts,
            )
            .await;
            return;
        }
    }

    let token = match crate::repositories::continuation_token::generate_token(
        pool,
        payload.session_id,
        &payload.visitor_id,
        &entry.recipient,
        payload.disconnect_epoch,
    )
    .await
    {
        Ok(token) => token,
        Err(error) => {
            let _ = ChatAlertRepository::mark_retry(
                pool,
                entry.id,
                &format!("Error generando token: {error}"),
                entry.attempts,
            )
            .await;
            return;
        }
    };
    let site_url = std::env::var("SITE_URL")
        .unwrap_or_else(|_| "https://nakomi.studio".to_string())
        .trim_end_matches('/')
        .to_string();
    let continuation_url = format!(
        "{site_url}/continuar-chat#token={}",
        urlencoding::encode(&token)
    );
    let result = tokio::time::timeout(
        SMTP_TIMEOUT,
        crate::services::EmailService::send_chat_continuation(
            config,
            pool,
            &entry.recipient,
            &payload.visitor_name,
            &continuation_url,
            payload.session_id,
        ),
    )
    .await;
    match result {
        Ok(Ok(())) => {
            let _ = ChatAlertRepository::mark_sent(pool, entry.id).await;
        }
        Ok(Err(error)) => {
            let _ = crate::repositories::continuation_token::revoke_for_session(
                pool,
                payload.session_id,
            )
            .await;
            let _ = ChatAlertRepository::mark_retry(pool, entry.id, &error, entry.attempts).await;
        }
        Err(_) => {
            let _ = crate::repositories::continuation_token::revoke_for_session(
                pool,
                payload.session_id,
            )
            .await;
            let _ = ChatAlertRepository::mark_retry(
                pool,
                entry.id,
                "Timeout SMTP de continuación",
                entry.attempts,
            )
            .await;
        }
    }
}

/// Envía WhatsApp vía gateway firmado y marca el resultado.
async fn process_whatsapp(pool: &PgPool, entry: &ChatAlertOutbox, http_client: &reqwest::Client) {
    let gateway_url = match std::env::var("GLORY_ALERT_GATEWAY_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            let _ = ChatAlertRepository::mark_dead(
                pool,
                entry.id,
                "GLORY_ALERT_GATEWAY_URL no configurado",
            )
            .await;
            return;
        }
    };

    let shared_secret = match std::env::var("GLORY_INTERNAL_ALERT_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            let _ = ChatAlertRepository::mark_dead(
                pool,
                entry.id,
                "GLORY_INTERNAL_ALERT_SECRET no configurado",
            )
            .await;
            return;
        }
    };

    let payload: crate::models::AlertPayload = match serde_json::from_value(entry.payload.clone()) {
        Ok(p) => p,
        Err(e) => {
            let _ =
                ChatAlertRepository::mark_dead(pool, entry.id, &format!("Payload inválido: {e}"))
                    .await;
            return;
        }
    };

    let gw_payload = crate::services::whatsapp_gateway::GatewayPayload {
        event: entry.event_type.clone(),
        message_id: payload.message_id.to_string(),
        session_id: payload.session_id.to_string(),
        visitor_label: payload.sender_label.clone(),
        preview: payload.preview.clone(),
        panel_url: payload.panel_url.clone(),
        occurred_at: payload.occurred_at.to_rfc3339(),
        idempotency_key: entry.idempotency_key.clone(),
    };

    let result = tokio::time::timeout(
        GATEWAY_TIMEOUT,
        crate::services::whatsapp_gateway::send_alert(
            http_client,
            &gateway_url,
            &shared_secret,
            &gw_payload,
        ),
    )
    .await;

    let gw_result = match result {
        Ok(r) => r,
        Err(_) => {
            let _ = ChatAlertRepository::mark_retry(
                pool,
                entry.id,
                "Timeout gateway (10s)",
                entry.attempts,
            )
            .await;
            return;
        }
    };

    match gw_result {
        crate::services::whatsapp_gateway::GatewayResult::Accepted => {
            let _ = ChatAlertRepository::mark_accepted_by_gateway(pool, entry.id).await;
            tracing::debug!(
                entry_id = %entry.id,
                "WhatsApp alerta aceptada por gateway"
            );
        }
        crate::services::whatsapp_gateway::GatewayResult::Retryable(error) => {
            let _ = ChatAlertRepository::mark_retry(pool, entry.id, &error, entry.attempts).await;
            tracing::warn!(
                entry_id = %entry.id,
                attempt = entry.attempts,
                "WhatsApp alerta reintentable: {error}"
            );
        }
        crate::services::whatsapp_gateway::GatewayResult::Fatal(error) => {
            let _ = ChatAlertRepository::mark_dead(pool, entry.id, &error).await;
            tracing::error!(
                entry_id = %entry.id,
                "WhatsApp alerta fatal: {error}"
            );
        }
    }
}

/// Log resumen del estado de la outbox.
async fn log_batch_summary(pool: &PgPool) {
    if let Ok(counts) = ChatAlertRepository::counts_by_status(pool).await {
        let total: i64 = counts.iter().map(|(_, c)| c).sum();
        if total > 0 {
            let parts: Vec<String> = counts.iter().map(|(s, c)| format!("{s}={c}")).collect();
            tracing::info!(
                "[chat-alert-worker] Outbox: {} ({})",
                total,
                parts.join(", ")
            );
        }
    }
    if let Ok(Some(age)) = ChatAlertRepository::oldest_pending_age_secs(pool).await {
        if age > 60 {
            tracing::warn!("[chat-alert-worker] Alerta más antigua pendiente: {age}s");
        }
    }
}
