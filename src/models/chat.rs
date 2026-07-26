/* [044A-38 Fase 5] Modelos de chat: sesiones, mensajes y tipos WebSocket.
 * chat_sessions vinculadas opcionalmente a orders (order_id).
 * sender_type: client|ai|ai_intermediary|employee|admin (roles marketplace). */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/* ============================================================
MODELOS DE BD
============================================================ */

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct ChatSession {
    pub id: Uuid,
    pub visitor_id: Option<String>,
    pub visitor_name: Option<String>,
    pub user_id: Option<Uuid>,
    pub order_id: Option<Uuid>,
    pub status: String,
    pub assigned_staff_id: Option<Uuid>,
    pub ai_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /* [064A-72] Metadata del visitante capturada en la conexión WS.
     * default: los queries que no seleccionan estas columnas las reciben como None. */
    #[sqlx(default)]
    pub visitor_ip: Option<String>,
    #[sqlx(default)]
    pub visitor_user_agent: Option<String>,
    /* [104A-39] Cuándo el staff vio los mensajes por última vez. Default para
     * queries que no seleccionan esta columna (sesiones visitante, etc.). */
    #[sqlx(default)]
    pub last_viewed_at: Option<DateTime<Utc>>,
    /* [104A-40] Cuándo se conectó el visitante por última vez via WS.
     * Actualizado en cada conexión WS del visitor. Default para queries legacy. */
    #[sqlx(default)]
    pub visitor_last_connected_at: Option<DateTime<Utc>>,
    /* [124A-PAIS] País del visitante: CF-IPCountry header (si hay Cloudflare) o
     * lookup a ipapi.co al crear sesión. Default para queries legacy. */
    #[sqlx(default)]
    pub visitor_country: Option<String>,
    /* [124A-ESC] true cuando la IA detectó que se necesita intervención humana.
     * Se persiste en BD para que el panel muestre el indicador al recargar. */
    #[sqlx(default)]
    pub is_escalated: bool,
    /* [237A-9] Modo de IA: automatic|human_priority|manual_pause.
     * automatic: IA responde siempre. human_priority: IA como fallback 10min.
     * manual_pause: IA desactivada completamente.
     * Default "automatic" vía BD; queries legacy que no seleccionan esta
     * columna reciben "" (tratado como "automatic" en código). */
    #[sqlx(default)]
    pub ai_mode: String,
    /* [257A-9] Versión durable para invalidar respuestas IA que ya estaban
     * generándose cuando un humano intervino o cambió el modo. */
    #[sqlx(default)]
    pub ai_generation_epoch: i64,
}

/* [P-2] Perfil de visitante — memoria persistente entre sesiones.
 * Vinculado por visitor_id (localStorage), opcionalmente a user_id (si se registra). */
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct VisitorProfile {
    pub id: Uuid,
    pub visitor_id: String,
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
    pub display_name: Option<String>,
    pub context_summary: Option<String>,
    pub preferences: Option<serde_json::Value>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub total_sessions: i32,
    pub ip_addresses: Vec<String>,
    pub device_fingerprints: Vec<String>,
}

/* [P-2] Adjunto de mensaje de chat (imágenes, archivos, audio).
 * ai_description: generado por Vision (imágenes), Whisper (audio) o extracción (PDF). */
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct ChatAttachment {
    pub id: Uuid,
    pub message_id: Uuid,
    pub file_name: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub ai_description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct ChatMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub sender_type: String,
    pub sender_id: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    /* [P-2] Mensajes ricos: tipo + metadatos estructurados.
     * message_type: text|image|file|audio|invoice|service_card|order_card|action
     * metadata: JSON con datos específicos según message_type. */
    #[sqlx(default)]
    pub message_type: Option<String>,
    #[sqlx(default)]
    pub metadata: Option<serde_json::Value>,
    /* [237A-8] Secuencia monotónica por sesión para detección de gaps y dedupe.
     * Incrementada atómicamente en save_message/save_rich_message vía CTE. */
    #[sqlx(default)]
    pub sequence_num: Option<i64>,
}

/* [064A-70] Respuesta enriquecida con datos del sender (avatar + nombre) */
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatMessageResponse {
    pub id: Uuid,
    pub session_id: Uuid,
    pub sender_type: String,
    pub sender_id: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub sender_avatar_url: Option<String>,
    pub sender_display_name: Option<String>,
    /* [P-2] Campos de mensajes ricos */
    pub message_type: Option<String>,
    pub metadata: Option<serde_json::Value>,
    /* [237A-8] Secuencia monotónica por sesión */
    pub sequence_num: Option<i64>,
}

/* ============================================================
REQUESTS / RESPONSES
============================================================ */

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChatSessionRequest {
    pub visitor_id: Option<String>,
    pub visitor_name: Option<String>,
    pub order_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatSessionResponse {
    pub id: Uuid,
    pub order_id: Option<Uuid>,
    /// [064A-31] Número de orden legible (si la sesión está vinculada a una orden)
    pub order_number: Option<i32>,
    pub status: String,
    pub ai_enabled: bool,
    pub assigned_staff_id: Option<Uuid>,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /* [064A-72] Info del visitante para panel lateral */
    pub visitor_name: Option<String>,
    pub visitor_ip: Option<String>,
    pub visitor_user_agent: Option<String>,
    /* [124A-PAIS] País del visitante */
    pub visitor_country: Option<String>,
    /* [104A-39] Cuándo se vio por última vez esta sesión (para badge unread) */
    pub last_viewed_at: Option<DateTime<Utc>>,
    /* [104A-40] Cuándo se conectó el visitante por última vez via WS */
    pub visitor_last_connected_at: Option<DateTime<Utc>>,
    /* [124A-ESC] true cuando la IA detectó que se necesita intervención humana */
    pub is_escalated: bool,
    /* [154A-14] Nombres y avatares de los participantes — enriquecido desde orders+users */
    pub client_name: Option<String>,
    pub client_avatar_url: Option<String>,
    pub employee_name: Option<String>,
    pub employee_avatar_url: Option<String>,
}

/* [064A-72] Modelo de notas de sesión de chat */
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct ChatSessionNote {
    pub id: Uuid,
    pub session_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSessionNoteRequest {
    pub content: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateVisitorNameRequest {
    pub name: String,
}

/* ============================================================
MENSAJES WEBSOCKET (protocolo JSON)
============================================================ */

/// Mensaje entrante del cliente WebSocket
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    #[serde(rename = "message")]
    Message { content: String },
    #[serde(rename = "typing")]
    /* [104A-40] session_id opcional: staff lo envía para indicar en qué sesión escribe.
     * Visitor no lo necesita (siempre en su propia sesión). */
    Typing {
        content: String,
        session_id: Option<Uuid>,
    },
    #[serde(rename = "join")]
    Join { session_id: Uuid },
    #[serde(rename = "close")]
    Close,
    #[serde(rename = "toggle_ai")]
    ToggleAi { session_id: Uuid, enabled: bool },
    /* [T-2] Acción desde botones de mensajes ricos (service_card, invoice, etc.) */
    #[serde(rename = "action")]
    Action {
        action_type: String,
        payload: serde_json::Value,
    },
}

/// Mensaje saliente del servidor WebSocket
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    #[serde(rename = "message")]
    Message {
        id: Uuid,
        session_id: Uuid,
        sender: String,
        sender_id: Option<String>,
        content: String,
        created_at: DateTime<Utc>,
        /* [P-2] Campos opcionales para mensajes ricos */
        #[serde(skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
        /* [237A-8] Secuencia monotónica + tipo de entrega */
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_num: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delivery: Option<String>,
    },
    #[serde(rename = "typing")]
    Typing {
        session_id: Uuid,
        sender: String,
        content: String,
    },
    #[serde(rename = "status")]
    Status { session_id: Uuid, value: String },
    #[serde(rename = "session_new")]
    SessionNew { session: ChatSession },
    #[serde(rename = "session_closed")]
    SessionClosed { session_id: Uuid },
    #[serde(rename = "error")]
    Error { message: String },
    /* [084A-40] Comando /reset: el backend ordena al cliente limpiar estado local */
    #[serde(rename = "reset")]
    Reset,
    /* [104A-40] Estado de conexión del visitante: online/offline.
     * Enviado al canal de staff cuando el visitor conecta o desconecta su WS.
     * Sirve como señal de presencia y confirmación de lectura (si está online, vio los mensajes). */
    #[serde(rename = "visitor_status")]
    VisitorStatus {
        session_id: Uuid,
        online: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        last_connected_at: Option<DateTime<Utc>>,
    },
}

/* [237A-8] Helper para construir WsServerMessage::Message desde ChatMessage.
 * Evita repetir todos los campos en cada sitio de construcción.
 * delivery: "live" para mensajes nuevos, "history" para replay. */
impl WsServerMessage {
    pub fn from_chat_message(msg: &ChatMessage, delivery: &str) -> Self {
        Self::Message {
            id: msg.id,
            session_id: msg.session_id,
            sender: msg.sender_type.clone(),
            sender_id: msg.sender_id.clone(),
            content: msg.content.clone(),
            created_at: msg.created_at,
            message_type: msg.message_type.clone(),
            metadata: msg.metadata.clone(),
            sequence_num: msg.sequence_num,
            delivery: Some(delivery.to_string()),
        }
    }
}
