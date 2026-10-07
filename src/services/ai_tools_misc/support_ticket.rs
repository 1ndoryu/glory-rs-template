/* [01AA-4-f3q] Tickets de soporte desde el chatbot (extraido de ai_tools_misc.rs).
 * [T-9] Se almacena como nota en visitor_profile.context_summary para que el
 * equipo lo vea. En el futuro se puede crear una tabla dedicada de tickets. */

use serde_json::{json, Value};
use sqlx::PgPool;

use crate::repositories::ChatRepository;

use crate::services::ai_tools::{
    require_auth, tool_status, RichMessage, ToolAuthContext, ToolExecResult,
};

pub(crate) async fn exec_create_support_ticket(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
    visitor_id: Option<&str>,
    args: &Value,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    let category = args["category"].as_str().unwrap_or("general");
    let description = args["description"].as_str().unwrap_or("");
    let priority = args["priority"].as_str().unwrap_or("medium");

    if description.is_empty() {
        return tool_status("error", "Se necesita una descripción del problema");
    }

    let ticket_owner = visitor_id.unwrap_or("anónimo");

    /* Se almacena como nota en visitor_profile.context_summary para que el equipo lo vea.
     * En el futuro se puede crear una tabla dedicada de tickets. */
    if let Some(vid) = visitor_id {
        let ticket_json = json!({
            "type": "support_ticket",
            "category": category,
            "priority": priority,
            "description": description,
            "user_id": auth.user_id,
            "effective_role": auth.effective_role.to_string(),
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        let _ = ChatRepository::update_visitor_preferences(pool, vid, &ticket_json).await;
    }

    tracing::info!(
        user_id = %auth.user_id,
        visitor_id = ticket_owner,
        category,
        priority,
        "Ticket de soporte creado desde chatbot"
    );

    ToolExecResult {
        tool_result_json: json!({
            "status": "ok",
            "message": "Ticket de soporte creado exitosamente. El equipo lo revisará pronto.",
            "category": category,
            "priority": priority,
            "user_id": auth.user_id,
        })
        .to_string(),
        rich_message: Some(RichMessage {
            content: format!("📋 Ticket de soporte creado — {category} ({priority})"),
            message_type: "support_ticket".to_string(),
            metadata: json!({
                "category": category,
                "priority": priority,
                "description": description,
            }),
        }),
    }
}
