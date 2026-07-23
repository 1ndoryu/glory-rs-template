/* [237A-7d] Modelos del sistema de alertas de chat.
 * chat_alert_outbox: cola durable para email y WhatsApp.
 * Cada mensaje de cliente genera entradas idempotentes por canal.
 * El worker procesa las entradas y las marca como sent/dead. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/* Canal de entrega de la alerta.
 * 'in_app' no usa outbox: se inserta directo en notifications dentro de la TX. */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertChannel {
    Email,
    WhatsApp,
}

impl AlertChannel {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::WhatsApp => "whatsapp",
        }
    }
}

impl std::fmt::Display for AlertChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/* Estado de una entrada en la outbox. */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    Pending,
    Processing,
    AcceptedByGateway,
    Sent,
    Failed,
    Dead,
}

impl OutboxStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::AcceptedByGateway => "accepted_by_gateway",
            Self::Sent => "sent",
            Self::Failed => "failed",
            Self::Dead => "dead",
        }
    }
}

/* Tipo de evento que generó la alerta. */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertEventType {
    #[serde(rename = "chat.client_message")]
    ClientMessage,
}

impl AlertEventType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClientMessage => "chat.client_message",
        }
    }
}

/* Fila de chat_alert_outbox. */
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatAlertOutbox {
    pub id: Uuid,
    pub idempotency_key: String,
    pub event_type: String,
    pub channel: String,
    pub recipient: String,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub available_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

/* Payload versionado para la outbox. */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub message_id: Uuid,
    pub session_id: Uuid,
    pub sender_label: String,
    pub preview: String,
    pub panel_url: String,
    pub occurred_at: DateTime<Utc>,
}

/* Ciclo de escalamiento de chat. */
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatEscalation {
    pub id: Uuid,
    pub session_id: Uuid,
    pub opened_by_message_id: Option<Uuid>,
    pub status: String,
    pub reason: Option<String>,
    pub cta_message_id: Option<Uuid>,
    pub opened_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
}

/* Ciclo de respuesta (10 min human takeover / AI fallback). */
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatResponseCycle {
    pub id: Uuid,
    pub session_id: Uuid,
    pub opened_by_message_id: Uuid,
    pub first_client_message_at: DateTime<Utc>,
    pub deadline_at: DateTime<Utc>,
    pub status: String,
    pub claimed_at: Option<DateTime<Utc>>,
    pub answered_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/* Estado del modo IA de una sesión. */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiMode {
    Automatic,
    HumanPriority,
    ManualPause,
}

impl AiMode {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::HumanPriority => "human_priority",
            Self::ManualPause => "manual_pause",
        }
    }
}

impl std::fmt::Display for AiMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
