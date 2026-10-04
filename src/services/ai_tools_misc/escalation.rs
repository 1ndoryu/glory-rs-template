/* [01AA-4-f3q] Escalación a humano con CTA de WhatsApp (extraido de ai_tools_misc.rs).
 * [237A-7g] Marca escalación + genera contact_cta; la URL sale de
 * PUBLIC_SUPPORT_WHATSAPP (número público del soporte). */

use serde_json::{json, Value};

use super::ai_tools::{tool_status, RichMessage, ToolExecResult};

pub(crate) fn exec_request_human(args: &Value) -> ToolExecResult {
    let reason = args["reason"].as_str().unwrap_or("Sin motivo especificado");

    /* Construir CTA de WhatsApp si el número público está configurado */
    let support_whatsapp = std::env::var("PUBLIC_SUPPORT_WHATSAPP")
        .ok()
        .filter(|s| !s.is_empty());

    let rich_message = support_whatsapp.and_then(|raw_number| {
        /* Normalizar a dígitos para wa.me — requiere al menos 7 dígitos (número real) */
        let digits: String = raw_number.chars().filter(char::is_ascii_digit).collect();
        if digits.len() < 7 {
            tracing::warn!("PUBLIC_SUPPORT_WHATSAPP no tiene dígitos suficientes: {raw_number}");
            return None;
        }
        let prefill = "Hola, quiero conversar más a fondo sobre mi proyecto con Nakomi Studio.";
        let href = format!(
            "https://wa.me/{digits}?text={}",
            urlencoding::encode(prefill)
        );

        Some(RichMessage {
            content: "Este caso necesita atención personal.".to_string(),
            message_type: "contact_cta".to_string(),
            metadata: json!({
                "label": "Escribir por WhatsApp",
                "href": href,
                "fallback": "El equipo fue notificado y responderá por este chat.",
                "reason": reason,
            }),
        })
    });

    ToolExecResult {
        tool_result_json: json!({
            "status": "escalated",
            "reason": reason,
            "message": "Se ha notificado al equipo. Un especialista se conectará pronto."
        })
        .to_string(),
        rich_message,
    }
}
