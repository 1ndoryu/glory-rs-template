/* [P-1 Chatbot v2] Módulo de handlers de chat: WebSocket + REST.
 * Refactorizado de un solo archivo (~670 líneas) a módulo por dominio.
 * rest.rs subdividido en rest_messages, rest_notes, rest_upload.
 * Interfaz pública: ws_routes() + rest_routes() sin cambios. */

mod file_ai;
mod rest;
pub(crate) mod rest_messages;
pub(crate) mod rest_notes;
pub(crate) mod rest_upload;
mod ws_staff;
mod ws_visitor;
mod ws_visitor_helpers;

pub use rest::*;

use crate::AppState;
use axum::Router;

/* ws_routes combina visitor + staff WebSocket routes (montadas en root) */
pub fn ws_routes() -> Router<AppState> {
    ws_visitor::ws_routes().merge(ws_staff::ws_staff_routes())
}

use serde::Deserialize;
use uuid::Uuid;

use crate::models::{ChatMessage, ChatMessageResponse};
use crate::repositories::UserRepository;

/* ============================================================
TIPOS COMPARTIDOS ENTRE SUBMÓDULOS
============================================================ */

#[derive(Deserialize)]
pub struct VisitorWsParams {
    pub visitor_id: String,
    pub visitor_name: Option<String>,
    /* [T-9] Token JWT opcional para clientes autenticados */
    pub token: Option<String>,
    /* [084A-28] Contexto de origen: "hosting:{uuid}", "service:{slug}", etc. */
    pub context: Option<String>,
}

#[derive(Deserialize)]
pub struct StaffWsParams {
    pub token: String,
}

#[derive(Deserialize)]
pub struct MessagesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/* [064A-70] Enriquecer mensajes con avatar y display_name del sender.
 * Hace un solo query batch por los sender_ids que son UUIDs válidos. */
async fn enrich_messages(
    pool: &sqlx::PgPool,
    messages: Vec<ChatMessage>,
) -> Vec<ChatMessageResponse> {
    use std::collections::HashMap;

    let user_ids: Vec<Uuid> = messages
        .iter()
        .filter(|m| matches!(m.sender_type.as_str(), "employee" | "admin" | "client"))
        .filter_map(|m| m.sender_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()))
        .collect::<std::collections::HashSet<Uuid>>()
        .into_iter()
        .collect();

    let mut avatar_map: HashMap<Uuid, (Option<String>, Option<String>)> = HashMap::new();
    if !user_ids.is_empty() {
        if let Ok(rows) = UserRepository::info_by_ids(pool, &user_ids).await {
            for (id, avatar_url, display_name) in rows {
                avatar_map.insert(id, (avatar_url, display_name));
            }
        }
    }

    messages
        .into_iter()
        .map(|m| {
            let (avatar, name) = m
                .sender_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .and_then(|uid| avatar_map.get(&uid))
                .cloned()
                .unwrap_or_default();
            ChatMessageResponse {
                id: m.id,
                session_id: m.session_id,
                sender_type: m.sender_type,
                sender_id: m.sender_id,
                content: m.content,
                created_at: m.created_at,
                sender_avatar_url: avatar,
                sender_display_name: name,
                message_type: m.message_type,
                metadata: m.metadata,
                sequence_num: m.sequence_num,
            }
        })
        .collect()
}
