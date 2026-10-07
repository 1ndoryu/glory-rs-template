/* [01AA-4-f3q] Memoria de visitante T-3 (extraido de ai_tools_misc.rs).
 * capture_email + save_client_info con escrituras independientes de nombre
 * y email ([267A-2]: guardar el nombre nunca vacía un email ya capturado). */

/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: query runtime intencional.
 * `update_session_visitor_name` usa `sqlx::query` sin macro para no depender de
 * caché offline en una actualización puntual de sesión (259A-4e). */

use serde_json::{json, Value};
use sqlx::PgPool;

use crate::repositories::ChatRepository;

use crate::services::ai_tools::{tool_status, ToolExecResult};

/* [124A-CHAT2] Helper: actualiza visitor_name en chat_sessions para que el panel
 * muestre el nombre real del visitante capturado por la IA.
 * Usa sqlx::query (sin macro) para evitar requerir caché offline. */
async fn update_session_visitor_name(
    pool: &PgPool,
    session_id: uuid::Uuid,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_sessions SET visitor_name = $2, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

/* [237A-10] Validación de email: formato básico RFC 5322 simplificado.
 * No es exhaustivo (DNS/MX check queda fuera), pero filtra la mayoría de
 * entradas inválidas que la IA pueda aceptar por error. */
fn is_valid_email(email: &str) -> bool {
    let trimmed = email.trim().to_lowercase();
    if trimmed.len() < 5 || trimmed.len() > 254 {
        return false;
    }
    /* Debe tener exactamente un @ con texto antes y después */
    let parts: Vec<&str> = trimmed.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    /* Dominio debe tener al menos un punto y no empezar/terminar con punto */
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    /* Local no puede empezar/terminar con punto ni tener dos puntos seguidos */
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    true
}

/* [T-3] capture_email: guarda email del visitante en visitor_profiles.
 * También actualiza display_name si lo proporcionó.
 * [124A-CHAT2] Actualiza visitor_name en chat_sessions para que el panel muestre el nombre real. */
pub(crate) async fn exec_capture_email(
    pool: &PgPool,
    visitor_id: Option<&str>,
    session_id: uuid::Uuid,
    args: &Value,
) -> ToolExecResult {
    let Some(vid) = visitor_id else {
        return tool_status("error", "visitor_id no disponible");
    };

    let raw_email = args["email"].as_str().unwrap_or("");
    if raw_email.is_empty() {
        return tool_status("error", "Email no proporcionado");
    }

    /* [237A-10] Validación real de formato email */
    if !is_valid_email(raw_email) {
        return tool_status(
            "error",
            "El email proporcionado no tiene un formato válido.",
        );
    }

    let email_normalized = raw_email.trim().to_lowercase();
    let display_name = args["display_name"].as_str();

    /* Si la IA nos da el nombre junto con el email, actualizar visitor_name en la sesión. */
    if let Some(name) = display_name {
        if let Err(error) = update_session_visitor_name(pool, session_id, name).await {
            tracing::error!(%session_id, "Error actualizando nombre de sesión: {error}");
            return tool_status("error", "Error guardando el nombre del visitante");
        }
    }

    match ChatRepository::capture_visitor_email(pool, vid, &email_normalized, display_name).await {
        Ok(profile) => {
            /* [267A-2] No registrar PII: basta identificar al visitante y el resultado. */
            tracing::info!(visitor_id = %vid, "Email de visitante capturado");
            ToolExecResult {
                tool_result_json: json!({
                    "status": "ok",
                    "email": profile.email,
                    "display_name": profile.display_name,
                    "message": "Email guardado correctamente."
                })
                .to_string(),
                rich_message: None,
            }
        }
        Err(e) => {
            tracing::error!("Error guardando email visitor {vid}: {e}");
            tool_status("error", "Error guardando email")
        }
    }
}

/* [T-3] save_client_info: guarda preferencias/datos del cliente en visitor_profiles.
 * Hace merge con preferencias existentes (JSON ||).
 * [124A-CHAT2] Si se incluye 'name', actualiza visitor_name en chat_sessions para el panel. */
pub(crate) async fn exec_save_client_info(
    pool: &PgPool,
    visitor_id: Option<&str>,
    session_id: uuid::Uuid,
    args: &Value,
) -> ToolExecResult {
    let Some(vid) = visitor_id else {
        return tool_status("error", "visitor_id no disponible");
    };

    /* [267A-2] Nombre y email tienen escrituras independientes: guardar el nombre
     * nunca debe convertir un email previamente capturado en cadena vacía. */
    if let Some(name) = args["name"].as_str() {
        if !name.trim().is_empty() {
            let normalized_name = name.trim();
            if let Err(error) =
                ChatRepository::update_visitor_display_name(pool, vid, normalized_name).await
            {
                tracing::error!(visitor_id = %vid, "Error guardando nombre del visitante: {error}");
                return tool_status("error", "Error guardando el nombre del visitante");
            }
            if let Err(error) = update_session_visitor_name(pool, session_id, normalized_name).await
            {
                tracing::error!(%session_id, "Error actualizando nombre de sesión: {error}");
                return tool_status("error", "Error guardando el nombre en la conversación");
            }
        }
    }

    /* Construir objeto de preferencias solo con campos que la IA proporcionó */
    let mut prefs = serde_json::Map::new();
    if let Some(v) = args.get("industry") {
        prefs.insert("industry".to_string(), v.clone());
    }
    if let Some(v) = args.get("budget_range") {
        prefs.insert("budget_range".to_string(), v.clone());
    }
    if let Some(v) = args.get("interests") {
        prefs.insert("interests".to_string(), v.clone());
    }
    if let Some(v) = args.get("project_description") {
        prefs.insert("project_description".to_string(), v.clone());
    }
    if let Some(v) = args.get("notes") {
        prefs.insert("notes".to_string(), v.clone());
    }

    if prefs.is_empty() && args["name"].as_str().is_none() {
        return ToolExecResult {
            tool_result_json: json!({"status": "ok", "message": "Sin datos nuevos"}).to_string(),
            rich_message: None,
        };
    }

    if prefs.is_empty() {
        return ToolExecResult {
            tool_result_json: json!({"status": "ok", "message": "Nombre guardado."}).to_string(),
            rich_message: None,
        };
    }

    let prefs_value = Value::Object(prefs);
    match ChatRepository::update_visitor_preferences(pool, vid, &prefs_value).await {
        Ok(()) => {
            tracing::info!("Preferencias actualizadas para visitor {vid}");
            ToolExecResult {
                tool_result_json: json!({
                    "status": "ok",
                    "saved_fields": prefs_value,
                    "message": "Información del cliente guardada."
                })
                .to_string(),
                rich_message: None,
            }
        }
        Err(e) => {
            tracing::error!("Error guardando preferencias visitor {vid}: {e}");
            tool_status("error", "Error guardando información")
        }
    }
}
