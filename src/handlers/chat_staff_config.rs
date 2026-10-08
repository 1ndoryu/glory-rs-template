/* [08AA-6] Config editable del panel (allowlist + validación pura + endpoints
 * leer/guardar). Salió de `chat_staff.rs` (god-object >800): este dominio
 * solo depende de `fail` y del estado compartido, sin rutas propias
 * (`staff_routes` los referencia vía `super`).
 * Gotcha: `telefono_valido` admite vacío (el panel borra con "") y es
 * DISTINTO del de `chat_tools` (allí exige ≥6 dígitos): no unificar. */

use axum::extract::State;
use axum::Json;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;

use super::chat_staff::fail;

/// Claves editables desde el panel (allowlist: nada fuera de aquí).
/* [07AA-1 F4] Todo-controlable del plan 03AA-4: a las 7 de F1-F3 se suman
 * `ventana_retraso_min` (triage), `whatsapp_autorizados` (política,
 * fail-closed a `Publico`), `ia_tope_tokens_dia` (tope_uso), `corte_whatsapp`
 * (outbox_idempotency, base de F5) y el tono del turno (`whatsapp_acuse /
 * fallback / aviso_asesor_texto`, que `turno::leer_tono` lee con fallback a
 * las constantes). `GLORY_ALERT_GATEWAY_URL` queda env a propósito (URL de
 * infra, no editable por UI); `prompt_extra` sigue guardada sin lector. */
pub(super) const CLAVES_CONFIG: &[&str] = &[
    "prompt_extra",
    "contacto_telefono",
    "whatsapp_admin",
    "wa_numero_a",
    "wa_numero_b",
    "ai_enabled_global",
    "tools_deshabilitadas",
    "ventana_retraso_min",
    "whatsapp_autorizados",
    "ia_tope_tokens_dia",
    "corte_whatsapp",
    "whatsapp_acuse_texto",
    "whatsapp_fallback_texto",
    "whatsapp_aviso_asesor_texto",
];

/// ¿Teléfono con la forma que acepta el panel? Dígitos/`+`/espacios (≤24).
fn telefono_valido(valor: &str) -> bool {
    let v = valor.trim();
    v.is_empty() || (v.len() <= 24 && v.chars().all(|c| c.is_ascii_digit() || "+ ".contains(c)))
}

/// ¿CSV de autorizados aceptable? Vacío (nadie = todo `Publico`) o dígitos,
/// `+`, espacios, comas y guiones (≤2000); lo raro lo filtra
/// `leer_autorizados` igual que al remitente.
fn autorizados_validos(valor: &str) -> bool {
    let v = valor.trim();
    v.is_empty()
        || (v.len() <= 2000
            && v.chars()
                .all(|c| c.is_ascii_digit() || "+-, ".contains(c) || c == '-'))
}

/// ¿Ventana de retraso aceptable? Vacío (vuelve al default 10) o 1..=1440.
fn ventana_valida(valor: &str) -> bool {
    let v = valor.trim();
    v.is_empty() || v.parse::<i64>().is_ok_and(|n| (1..=1440).contains(&n))
}

/// ¿Tope diario aceptable? Vacío (default 2M) o 1..=100M.
fn tope_valido(valor: &str) -> bool {
    let v = valor.trim();
    v.is_empty()
        || v.parse::<i64>()
            .is_ok_and(|n| (1..=100_000_000).contains(&n))
}

/// ¿Texto de tono aceptable? Vacío (vuelve a la constante) o ≤500 chars.
fn tono_valido(valor: &str) -> bool {
    valor.trim().chars().count() <= 500
}

/// Valida un par clave/valor de config (pura para testear). `false` = 400
/// con el motivo; el llamador nunca persiste lo inválido.
fn config_valida(clave: &str, valor: &str) -> bool {
    match clave {
        "ai_enabled_global" => valor == "on" || valor == "off",
        "whatsapp_admin" | "wa_numero_a" | "wa_numero_b" => telefono_valido(valor),
        "ventana_retraso_min" => ventana_valida(valor),
        "whatsapp_autorizados" => autorizados_validos(valor),
        "ia_tope_tokens_dia" => tope_valido(valor),
        "corte_whatsapp" => matches!(valor.trim(), "" | "total" | "wa_b" | "apagado"),
        "whatsapp_acuse_texto" | "whatsapp_fallback_texto" | "whatsapp_aviso_asesor_texto" => {
            tono_valido(valor)
        }
        _ => valor.trim().chars().count() <= 8000,
    }
}

pub(super) async fn leer_config(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<std::collections::HashMap<String, Option<String>>>, AppError> {
    let mut mapa = std::collections::HashMap::new();
    for clave in CLAVES_CONFIG {
        let valor = glory_agent::persistence::get_config(&state.pool, clave)
            .await
            .map_err(|e| fail(&e))?;
        mapa.insert((*clave).to_string(), valor);
    }
    Ok(Json(mapa))
}

/// Guarda config. Cada clave valida con `config_valida` (pura, testeada);
/// lo inválido es 400 y nunca se persiste.
pub(super) async fn guardar_config(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    if input.is_empty() || input.len() > CLAVES_CONFIG.len() {
        return Err(AppError::BadRequest("payload vacio o excesivo".to_string()));
    }
    for (clave, valor) in &input {
        if !CLAVES_CONFIG.contains(&clave.as_str()) {
            return Err(AppError::BadRequest(format!("clave no editable: {clave}")));
        }
        if !config_valida(clave, valor) {
            return Err(AppError::BadRequest(format!("{clave} invalido")));
        }
        glory_agent::persistence::set_config(&state.pool, clave, valor.trim())
            .await
            .map_err(|e| fail(&e))?;
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

#[cfg(test)]
mod pruebas {
    use super::config_valida;

    /* [07AA-1 F4] La allowlist se valida pura: lo inválido es 400 y nunca
     * se persiste. */
    #[test]
    fn config_valida_cubre_todo_controlable() {
        assert!(config_valida("ai_enabled_global", "on"));
        assert!(config_valida("ai_enabled_global", "off"));
        assert!(!config_valida("ai_enabled_global", "ON"));
        assert!(config_valida("whatsapp_admin", "+34600111222"));
        assert!(config_valida("wa_numero_a", ""));
        assert!(!config_valida("wa_numero_a", "abc"));
        assert!(!config_valida("wa_numero_a", &"1".repeat(25)));
        assert!(config_valida("ventana_retraso_min", ""));
        assert!(config_valida("ventana_retraso_min", "10"));
        assert!(config_valida("ventana_retraso_min", "1440"));
        assert!(!config_valida("ventana_retraso_min", "0"));
        assert!(!config_valida("ventana_retraso_min", "1441"));
        assert!(config_valida("whatsapp_autorizados", ""));
        assert!(config_valida(
            "whatsapp_autorizados",
            "584120825234, +34600111222"
        ));
        assert!(!config_valida("whatsapp_autorizados", "llama al 0412"));
        assert!(config_valida("ia_tope_tokens_dia", ""));
        assert!(config_valida("ia_tope_tokens_dia", "2000000"));
        assert!(!config_valida("ia_tope_tokens_dia", "0"));
        assert!(!config_valida("ia_tope_tokens_dia", "-5"));
        assert!(config_valida("corte_whatsapp", ""));
        assert!(config_valida("corte_whatsapp", "total"));
        assert!(config_valida("corte_whatsapp", "wa_b"));
        assert!(config_valida("corte_whatsapp", "apagado"));
        assert!(!config_valida("corte_whatsapp", "mitad"));
        assert!(config_valida("whatsapp_acuse_texto", ""));
        assert!(config_valida("whatsapp_acuse_texto", "Ya voy 👀"));
        assert!(!config_valida("whatsapp_acuse_texto", &"x".repeat(501)));
        assert!(config_valida("prompt_extra", "tono cercano"));
        assert!(!config_valida("prompt_extra", &"x".repeat(8001)));
    }
}
