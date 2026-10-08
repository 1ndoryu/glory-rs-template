//! [08AA-8] Envío manual + uso + auditoría + gateway `WhatsApp` del staff.
//!
//! Extraído de `chat_staff.rs` (god-object + límite 500): la consola de la
//! dueña (envío `wa_a`/`wa_b`, uso por día, auditoría de tomas humanas y
//! proxy de estado/QR del gateway Baileys). `chat_staff.rs` conserva bandeja
//! de sesiones, clientes y `staff_routes`, que referencia estos handlers
//! como `chat_staff_envio::*`. Sin cambios de comportamiento; el `join!` de
//! la rama `session_id` se documenta abajo.

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;

use super::chat_staff::fail;

#[derive(Debug, Deserialize)]
pub(super) struct EnvioManual {
    session_id: Option<Uuid>,
    cliente_id: Option<Uuid>,
    telefono: Option<String>,
    nombre: Option<String>,
    canal: Option<String>,
    texto: String,
    media_url: Option<String>,
}

/// Texto+media validados del envío manual (frontera: nada sin sanitizar).
fn validar_texto_media(input: &EnvioManual) -> Result<(String, Option<String>), AppError> {
    let texto = input.texto.trim();
    if texto.is_empty() || texto.len() > 4000 {
        return Err(AppError::BadRequest(
            "texto requerido (1..4000)".to_string(),
        ));
    }
    let media = input
        .media_url
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty());
    if let Some(u) = media {
        if u.len() > 2048 || !(u.starts_with("http://") || u.starts_with("https://")) {
            return Err(AppError::BadRequest(
                "media_url debe ser http(s) (<=2048)".to_string(),
            ));
        }
    }
    Ok((texto.to_string(), media.map(str::to_string)))
}

/// Resuelve a qué hilo y destino va el envío: hilo existente, cliente (abre
/// hilo `wa_a`/`completo` salvo `wa_b`) o teléfono nuevo (registra + abre).
async fn resolver_destino_envio(
    state: &AppState,
    input: &EnvioManual,
) -> Result<(Uuid, String), AppError> {
    if let Some(sid) = input.session_id {
        /* [08AA-8] Las 2 lecturas solo dependen de `sid`: van en `join!` en
         * vez de secuenciales (las demás ramas son cadenas dependientes en
         * ramas exclusivas y no se pueden unir; ver prevencion sqlite). */
        let (existe, tel) = tokio::join!(
            glory_agent::persistence::get_session(&state.pool, sid),
            sqlx::query_scalar("SELECT telefono FROM canal_sesiones WHERE session_id = $1")
                .bind(sid)
                .fetch_optional(&state.pool)
        );
        let existe = existe.map_err(|e| fail(&e))?;
        if existe.is_none() {
            return Err(AppError::NotFound("sesion no existe".to_string()));
        }
        let tel: Option<String> = tel?;
        let dest = tel
            .or(input.telefono.clone())
            .ok_or_else(|| AppError::BadRequest("sin telefono destino".to_string()))?;
        Ok((
            sid,
            crate::repositories::ClienteRepository::normalizar_telefono(&dest),
        ))
    } else if let Some(cid) = input.cliente_id {
        let cliente: Option<crate::models::ClienteRow> = sqlx::query_as(
            "SELECT id, nombre, telefono, origen, interes, presupuesto, zona, \
              notas, created_at, updated_at FROM clientes WHERE id = $1",
        )
        .bind(cid)
        .fetch_optional(&state.pool)
        .await?;
        let Some(c) = cliente else {
            return Err(AppError::NotFound("cliente no existe".to_string()));
        };
        let canal = if c.origen == "wa_b" { "wa_b" } else { "wa_a" };
        let modo = if canal == "wa_b" {
            "inicial"
        } else {
            "completo"
        };
        let sid = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&state.pool, sid)
            .await
            .map_err(|e| fail(&e))?;
        crate::repositories::ClienteRepository::vincular_canal(
            &state.pool,
            sid,
            c.id,
            &c.telefono,
            canal,
            modo,
        )
        .await?;
        Ok((sid, c.telefono))
    } else if let Some(tel) = input.telefono.clone() {
        if !super::chat_tools::telefono_valido(&tel) {
            return Err(AppError::BadRequest("telefono invalido".to_string()));
        }
        let canal = input.canal.as_deref().map_or("wa_a", str::trim);
        if !matches!(canal, "wa_a" | "wa_b") {
            return Err(AppError::BadRequest("canal debe ser wa_a|wa_b".to_string()));
        }
        let modo = if canal == "wa_b" {
            "inicial"
        } else {
            "completo"
        };
        let norm = crate::repositories::ClienteRepository::normalizar_telefono(&tel);
        let nombre = input
            .nombre
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        let cliente = crate::repositories::ClienteRepository::registrar_con_origen(
            &state.pool,
            nombre,
            &norm,
            canal,
        )
        .await?;
        let sid = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&state.pool, sid)
            .await
            .map_err(|e| fail(&e))?;
        crate::repositories::ClienteRepository::vincular_canal(
            &state.pool,
            sid,
            cliente.id,
            &norm,
            canal,
            modo,
        )
        .await?;
        Ok((sid, norm))
    } else {
        Err(AppError::BadRequest(
            "session_id|cliente_id|telefono requerido".to_string(),
        ))
    }
}

/// Enviar manual («dime y lo envío»): valida, resuelve hilo+destino y encola
/// outbox `whatsapp` con `destino`+`media_url`; el worker lo manda.
pub(super) async fn enviar_manual(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<EnvioManual>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (texto, media) = validar_texto_media(&input)?;
    let (sesion, destino) = resolver_destino_envio(&state, &input).await?;
    let mut payload = serde_json::json!({
        "session_id": sesion.to_string(),
        "destino": destino,
        "texto": texto,
        "motivo": "manual",
    });
    /* [289A-1] `via` = canal del hilo resuelto (única fuente de verdad,
     * cubre las 3 ramas de resolución). El gateway envía por esa sesión. */
    if let Ok(canal) = crate::repositories::ClienteRepository::canal_de(&state.pool, sesion).await {
        payload["via"] = serde_json::Value::String(canal.unwrap_or_else(|| "wa_a".to_string()));
    }
    if let Some(u) = media {
        payload["media_url"] = serde_json::Value::String(u);
    }
    let outbox = glory_agent::persistence::enqueue_outbox(&state.pool, "whatsapp", payload)
        .await
        .map_err(|e| fail(&e))?;
    Ok(Json(serde_json::json!({
        "ok": true, "session_id": sesion, "destino": destino, "outbox_id": outbox.id,
    })))
}

#[derive(Debug, Deserialize)]
pub(super) struct FiltroUso {
    dias: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(super) struct UsoDia {
    dia: Option<chrono::NaiveDate>,
    remitente: Option<String>,
    mensajes: Option<i64>,
    tokens_est: Option<i64>,
    tokens_in: Option<i64>,
    tokens_out: Option<i64>,
}

/// Uso (mensajes y tokens por día×remitente; `tokens_in/out` exactos del
/// núcleo F0 en mensajes `ai`, estima `len/4` del trigger en el resto).
pub(super) async fn uso_mensajes(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(f): Query<FiltroUso>,
) -> Result<Json<Vec<UsoDia>>, AppError> {
    let dias = f.dias.unwrap_or(7).clamp(1, 90);
    let filas: Vec<UsoDia> = sqlx::query_as(
        "SELECT date_trunc('day', created_at)::DATE AS dia, sender AS remitente, \
          COUNT(*) AS mensajes, SUM(tokens_est) AS tokens_est, \
          SUM(COALESCE(tokens_in, 0)) AS tokens_in, SUM(COALESCE(tokens_out, 0)) AS tokens_out \
         FROM uso_mensajes \
         WHERE created_at >= NOW() - ($1 || ' days')::INTERVAL \
         GROUP BY 1, 2 ORDER BY 1 DESC, 2",
    )
    .bind(dias.to_string())
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(filas))
}

#[derive(Debug, Deserialize)]
pub(super) struct Limite {
    limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(super) struct AuditoriaFila {
    session_id: Uuid,
    extracto: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    status: Option<String>,
    ai_enabled: Option<bool>,
    estado_atencion: Option<String>,
    modo_atencion: Option<String>,
    nombre: Option<String>,
    telefono: Option<String>,
    ia: Option<i64>,
    humano: Option<i64>,
    visitante: Option<i64>,
}

/// Auditoría: últimas tomas humanas con contexto + conteo IA/humano/visitante
/// por hilo (UNA consulta, sin N+1).
pub(super) async fn auditoria(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(q): Query<Limite>,
) -> Result<Json<Vec<AuditoriaFila>>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let filas: Vec<AuditoriaFila> = sqlx::query_as(
        "SELECT m.session_id, LEFT(m.body, 200) AS extracto, m.created_at, \
          s.status, s.ai_enabled, a.estado AS estado_atencion, a.modo AS modo_atencion, \
          c.nombre, c.telefono, \
          (SELECT COUNT(*) FROM agent_messages WHERE session_id = m.session_id AND sender = 'ai') AS ia, \
          (SELECT COUNT(*) FROM agent_messages WHERE session_id = m.session_id AND sender = 'staff') AS humano, \
          (SELECT COUNT(*) FROM agent_messages WHERE session_id = m.session_id AND sender = 'client') AS visitante \
         FROM agent_messages m \
         JOIN agent_sessions s ON s.id = m.session_id \
         LEFT JOIN atencion_sesiones a ON a.session_id = m.session_id \
         LEFT JOIN canal_sesiones cs ON cs.session_id = m.session_id \
         LEFT JOIN clientes c ON c.id = cs.cliente_id \
         WHERE m.sender = 'staff' ORDER BY m.created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(filas))
}

/// Base del gateway Baileys: `GATEWAY_BASE_URL` explícita, o derivada de
/// `GLORY_ALERT_GATEWAY_URL` (`.../send` → base), o default local.
fn gateway_base() -> String {
    if let Ok(b) = std::env::var("GATEWAY_BASE_URL") {
        let b = b.trim().trim_end_matches('/').to_string();
        if !b.is_empty() {
            return b;
        }
    }
    let envio = std::env::var("GLORY_ALERT_GATEWAY_URL").unwrap_or_default();
    let base = envio
        .trim()
        .strip_suffix("/send")
        .unwrap_or(envio.trim())
        .trim_end_matches('/');
    if base.is_empty() {
        "http://127.0.0.1:3102".to_string()
    } else {
        base.to_string()
    }
}

fn cliente_gateway() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

/// Estado de las sesiones Baileys (`via|nombre|numero|estado`) para la
/// consola. El gateway caído es error explícito (nunca lista vacía
/// silenciosa que parezca "sin sesiones").
pub(super) async fn sesiones_whatsapp(
    _auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut peticion = cliente_gateway().get(format!("{}/sesiones", gateway_base()));
    if let Ok(s) = std::env::var("GATEWAY_SEND_SECRET") {
        if !s.trim().is_empty() {
            peticion = peticion.header("X-Gateway-Secret", s);
        }
    }
    let resp = peticion
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("gateway WhatsApp no responde: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "gateway WhatsApp devolvió {}",
            resp.status()
        )));
    }
    let cuerpo: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("gateway WhatsApp ilegible: {e}")))?;
    Ok(Json(cuerpo))
}

/// QR pendiente de una sesión para vincular desde la consola. `canal`
/// validado en el boundary; sin QR pendiente (ya vinculada o gateway sin
/// esa sesión) devuelve 404 explícito, no imagen vacía.
pub(super) async fn qr_whatsapp(
    _auth: AuthUser,
    Path(canal): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if !matches!(canal.as_str(), "wa_a" | "wa_b") {
        return Err(AppError::BadRequest("canal debe ser wa_a|wa_b".to_string()));
    }
    let mut peticion = cliente_gateway().get(format!("{}/sesiones/{canal}/qr", gateway_base()));
    if let Ok(s) = std::env::var("GATEWAY_SEND_SECRET") {
        if !s.trim().is_empty() {
            peticion = peticion.header("X-Gateway-Secret", s);
        }
    }
    let resp = peticion
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("gateway WhatsApp no responde: {e}")))?;
    if resp.status() == axum::http::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound(
            "sin QR pendiente (sesión ya vinculada o gateway sin esa sesión)".to_string(),
        ));
    }
    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "gateway WhatsApp devolvió {}",
            resp.status()
        )));
    }
    let png = resp
        .bytes()
        .await
        .map_err(|e| AppError::Internal(format!("QR ilegible: {e}")))?;
    axum::response::Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .body(axum::body::Body::from(png))
        .map_err(|e| AppError::Internal(format!("no se pudo armar el QR: {e}")))
}
