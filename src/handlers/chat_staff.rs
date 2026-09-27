use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;
use glory_agent::errors::AgentError;

/* [169A-4] Atención humana del chat (panel admin). Todo requiere JWT
 * (`AuthUser`); vive en el router `AppState` porque el extractor pide ese
 * estado, y emite por `AppState.hub` (mismo `ChatHub` que el visitante).
 * Enviar un mensaje como staff implica tomar el hilo: apaga la IA y marca
 * el ciclo `escalated` (como la pestaña Mensajes de Nakomi). */

fn fail(e: &AgentError) -> AppError {
    AppError::Internal(e.to_string())
}

pub fn staff_routes() -> Router<AppState> {
    Router::new()
        .route("/agent/sesiones", get(listar_sesiones))
        .route("/agent/sesiones/:id/historial", get(historial))
        .route("/agent/sesiones/:id/mensajes", post(responder))
        .route("/agent/sesiones/:id/devolver", post(devolver_a_ia))
        .route("/agent/sesiones/:id", patch(actualizar_sesion))
        .route("/agent/config", get(leer_config).put(guardar_config))
}

#[derive(Debug, Deserialize)]
struct FiltroSesiones {
    estado: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct SesionResumen {
    id: Uuid,
    visitor_name: Option<String>,
    contact: Option<String>,
    status: String,
    ai_enabled: bool,
    /* [279A-2 F3] Estado propio de delegación (puede faltar en sesiones
     * viejas: LEFT JOIN + default en el panel). */
    estado_atencion: Option<String>,
    modo_atencion: Option<String>,
    last_body: Option<String>,
    last_sender: Option<String>,
    #[sqlx(rename = "last_at")]
    last_at: Option<chrono::DateTime<chrono::Utc>>,
    alertas: Option<i64>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Bandeja staff: sesiones con último mensaje y avisos pendientes, en UNA
/// consulta (LATERAL + subselect; prohibido N+1 por regla 7).
async fn listar_sesiones(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(f): Query<FiltroSesiones>,
) -> Result<Json<Vec<SesionResumen>>, AppError> {
    if let Some(e) = &f.estado {
        if !glory_agent::persistence::valid_session_status(e) {
            return Err(AppError::BadRequest(
                "estado debe ser open|escalated|closed".to_string(),
            ));
        }
    }
    let limit = f.limit.unwrap_or(50).clamp(1, 200);
    let filas: Vec<SesionResumen> = sqlx::query_as(
        "SELECT s.id, s.visitor_name, s.contact, s.status, s.ai_enabled, \
         a.estado AS estado_atencion, a.modo AS modo_atencion, \
         m.body AS last_body, m.sender AS last_sender, m.created_at AS last_at, \
         (SELECT COUNT(*) FROM agent_outbox o WHERE o.status = 'pending' \
          AND o.kind = 'whatsapp' AND o.payload->>'session_id' = s.id::TEXT) AS alertas, \
         s.updated_at \
         FROM agent_sessions s \
         LEFT JOIN atencion_sesiones a ON a.session_id = s.id \
         LEFT JOIN LATERAL (SELECT body, sender, created_at FROM agent_messages \
           WHERE session_id = s.id ORDER BY sequence_num DESC LIMIT 1) m ON true \
         WHERE ($1::TEXT IS NULL OR s.status = $1) \
         ORDER BY s.updated_at DESC LIMIT $2",
    )
    .bind(f.estado.clone())
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(filas))
}

#[derive(Debug, Deserialize)]
struct Limite {
    limit: Option<i64>,
}

/// Hilo completo para el panel (reutiliza el repo del núcleo).
async fn historial(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<Limite>,
) -> Result<Json<Vec<glory_agent::models::ChatMessage>>, AppError> {
    let msgs = glory_agent::persistence::list_messages(&state.pool, id, q.limit.unwrap_or(100))
        .await
        .map_err(|e| fail(&e))?;
    Ok(Json(msgs))
}

#[derive(Debug, Deserialize)]
struct RespuestaStaff {
    body: String,
}

/// Responder como humano: persiste (sender `staff`), emite por el WS del
/// visitante y TOMA el hilo (`ai_enabled=false` + ciclo `escalated`).
async fn responder(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<RespuestaStaff>,
) -> Result<Json<serde_json::Value>, AppError> {
    let body = input.body.trim().to_string();
    if body.is_empty() || body.len() > 8000 {
        return Err(AppError::BadRequest(
            "mensaje vacio o >8000 chars".to_string(),
        ));
    }
    glory_agent::persistence::ensure_session(&state.pool, id)
        .await
        .map_err(|e| fail(&e))?;
    let seq = state.hub.next_sequence(id);
    let msg = glory_agent::persistence::insert_message(&state.pool, id, "staff", &body, seq)
        .await
        .map_err(|e| fail(&e))?;
    let _ = state
        .hub
        .broadcast(id, &glory_agent::models::WsServerMessage::live(msg));
    glory_agent::persistence::set_session_ai(&state.pool, id, false)
        .await
        .map_err(|e| fail(&e))?;
    glory_agent::persistence::upsert_response_cycle(&state.pool, id, "escalated")
        .await
        .map_err(|e| fail(&e))?;
    /* [279A-2 F3] La toma humana también mueve la máquina propia a
     * `delegada` (un solo estado de verdad, sin doble fuente). */
    crate::repositories::ClienteRepository::marcar_atencion(&state.pool, id, "delegada")
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"ok": true, "sequence_num": seq})))
}

#[derive(Debug, Deserialize)]
struct DevolucionIA {
    nota: String,
}

/// Devolver a la IA con nota (cierre de `consultar_agente`): la nota queda
/// como mensaje `staff`, el ciclo pasa a `answered`, la IA se reactiva y la
/// máquina propia vuelve a `activa`. Solo humanos (JWT).
async fn devolver_a_ia(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<DevolucionIA>,
) -> Result<Json<serde_json::Value>, AppError> {
    let nota = input.nota.trim().to_string();
    if nota.is_empty() || nota.len() > 2000 {
        return Err(AppError::BadRequest("nota requerida (1..2000)".to_string()));
    }
    glory_agent::persistence::ensure_session(&state.pool, id)
        .await
        .map_err(|e| fail(&e))?;
    let seq = state.hub.next_sequence(id);
    let msg = glory_agent::persistence::insert_message(&state.pool, id, "staff", &nota, seq)
        .await
        .map_err(|e| fail(&e))?;
    let _ = state
        .hub
        .broadcast(id, &glory_agent::models::WsServerMessage::live(msg));
    glory_agent::persistence::set_session_ai(&state.pool, id, true)
        .await
        .map_err(|e| fail(&e))?;
    glory_agent::persistence::upsert_response_cycle(&state.pool, id, "answered")
        .await
        .map_err(|e| fail(&e))?;
    crate::repositories::ClienteRepository::marcar_atencion(&state.pool, id, "activa")
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"ok": true, "sequence_num": seq})))
}

#[derive(Debug, Deserialize)]
struct CambioSesion {
    #[serde(rename = "aiEnabled")]
    ai_enabled: Option<bool>,
    status: Option<String>,
}

/// Soltar la IA (`aiEnabled: true`), tomarla (`false`) o cerrar/archivar
/// (`status`). Validado en boundary.
async fn actualizar_sesion(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<CambioSesion>,
) -> Result<Json<glory_agent::models::ChatSession>, AppError> {
    if input.ai_enabled.is_none() && input.status.is_none() {
        return Err(AppError::BadRequest("nada que cambiar".to_string()));
    }
    if let Some(status) = &input.status {
        if !glory_agent::persistence::valid_session_status(status) {
            return Err(AppError::BadRequest(
                "status debe ser open|escalated|closed".to_string(),
            ));
        }
        glory_agent::persistence::set_session_status(&state.pool, id, status)
            .await
            .map_err(|e| fail(&e))?;
        if status == "escalated" {
            glory_agent::persistence::upsert_response_cycle(&state.pool, id, "escalated")
                .await
                .map_err(|e| fail(&e))?;
        }
    }
    if let Some(enabled) = input.ai_enabled {
        glory_agent::persistence::set_session_ai(&state.pool, id, enabled)
            .await
            .map_err(|e| fail(&e))?;
        /* [279A-2 F3] El toggle manual también mueve la máquina propia
         * (una sola verdad): soltar → `activa`, tomar → `delegada`. */
        let estado = if enabled { "activa" } else { "delegada" };
        crate::repositories::ClienteRepository::marcar_atencion(&state.pool, id, estado)
            .await
            .map_err(AppError::from)?;
    }
    let sesion = glory_agent::persistence::get_session(&state.pool, id)
        .await
        .map_err(|e| fail(&e))?
        .ok_or_else(|| AppError::NotFound("sesion no existe".to_string()))?;
    Ok(Json(sesion))
}

/// Claves editables desde el panel (allowlist: nada fuera de aquí).
const CLAVES_CONFIG: &[&str] = &[
    "prompt_extra",
    "contacto_telefono",
    "whatsapp_admin",
    "ai_enabled_global",
    "tools_deshabilitadas",
];

async fn leer_config(
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

/// Guarda config. `ai_enabled_global` solo `on|off`; `whatsapp_admin` solo
/// dígitos/`+`/espacios (≤24); el resto respeta el tope del núcleo (8000).
async fn guardar_config(
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
        match clave.as_str() {
            "ai_enabled_global" if valor != "on" && valor != "off" => {
                return Err(AppError::BadRequest(
                    "ai_enabled_global debe ser on|off".to_string(),
                ));
            }
            "whatsapp_admin"
                if !valor.trim().is_empty()
                    && (!valor
                        .chars()
                        .all(|c| c.is_ascii_digit() || "+ ".contains(c))
                        || valor.len() > 24) =>
            {
                return Err(AppError::BadRequest("whatsapp_admin invalido".to_string()));
            }
            _ => {}
        }
        glory_agent::persistence::set_config(&state.pool, clave, valor.trim())
            .await
            .map_err(|e| fail(&e))?;
    }
    Ok(Json(serde_json::json!({"ok": true})))
}
