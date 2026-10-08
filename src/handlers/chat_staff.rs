use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;
use glory_agent::errors::AgentError;

use super::chat_staff_config::{guardar_config, leer_config}; // [08AA-6] dominio config

/* [169A-4] Atención humana del chat (panel admin). Todo requiere JWT
 * (`AuthUser`); vive en el router `AppState` porque el extractor pide ese
 * estado, y emite por `AppState.hub` (mismo `ChatHub` que el visitante).
 * Enviar un mensaje como staff implica tomar el hilo: apaga la IA y marca
 * el ciclo `escalated` (como la pestaña Mensajes de Nakomi). */

pub(super) fn fail(e: &AgentError) -> AppError {
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
        /* [279A-2 F5] Consola dueña: clientes, envío manual, uso y auditoría.
         * Todo JSON para operar por terminal/HTTP (ver plan §8). */
        .route("/agent/clientes", get(listar_clientes).post(crear_cliente))
        .route("/agent/clientes/:id", patch(actualizar_cliente))
        .route("/agent/clientes/:id/sesiones", get(sesiones_de_cliente))
        .route("/agent/enviar", post(enviar_manual))
        .route("/agent/uso", get(uso_mensajes))
        .route("/agent/auditoria", get(auditoria))
        /* [289A-1] Vinculación desde la consola: proxy de estado+QR del
         * gateway (el navegador nunca habla con el gateway directo). */
        .route("/agent/whatsapp/sesiones", get(sesiones_whatsapp))
        .route("/agent/whatsapp/sesiones/:canal/qr", get(qr_whatsapp))
        /* [011A-2] Sombra F5-Paso1: huella solo-lectura tras GLORY_SHADOW=1. */
        .merge(super::sombra::sombra_routes())
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
    /* [289A-2] Teléfono del cliente (`canal_sesiones`; el panel muestra el
     * número en la conversación). */
    telefono: Option<String>,
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
         cs.telefono AS telefono, \
         m.body AS last_body, m.sender AS last_sender, m.created_at AS last_at, \
         (SELECT COUNT(*) FROM agent_outbox o WHERE o.status = 'pending' \
          AND o.kind = 'whatsapp' AND o.payload->>'session_id' = s.id::TEXT) AS alertas, \
          s.updated_at \
          FROM agent_sessions s \
          LEFT JOIN atencion_sesiones a ON a.session_id = s.id \
          LEFT JOIN canal_sesiones cs ON cs.session_id = s.id \
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

#[derive(Debug, Deserialize)]
struct HistorialQuery {
    limit: Option<i64>,
    before_seq: Option<i64>,
}

/// Hilo para el panel (paginado por cursor).
/// [309A-3] `before_seq` trae mensajes anteriores a esa secuencia (el panel
/// los antepone al hacer scroll arriba); `limit` acota la página (el panel
/// pide `PAGINA+1` y si llegan todas hay más). Sin cursor trae lo último.
/// Orden ASC de lectura; `list_messages` del núcleo no acepta cursor, así
/// que la consulta es propia (columnas en el orden de `ChatMessage`).
async fn historial(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<HistorialQuery>,
) -> Result<Json<Vec<glory_agent::models::ChatMessage>>, AppError> {
    let limit = q.limit.unwrap_or(100).clamp(1, 200);
    let mut msgs: Vec<glory_agent::models::ChatMessage> = sqlx::query_as(
        "SELECT id, session_id, sender, body, sequence_num, input_tokens, output_tokens, created_at \
         FROM agent_messages WHERE session_id = $1 \
         AND ($2::BIGINT IS NULL OR sequence_num < $2) \
         ORDER BY sequence_num DESC LIMIT $3",
    )
    .bind(id)
    .bind(q.before_seq)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::from)?;
    msgs.reverse();
    Ok(Json(msgs))
}

#[derive(Debug, Deserialize)]
struct RespuestaStaff {
    body: String,
}

/// Responder como humano: persiste (sender `staff`), emite por el WS del
/// visitante y TOMA el hilo (`ai_enabled=false` + ciclo `escalated`).
/// [289A-2] Si la sesión tiene hilo `WhatsApp`, el mensaje también se encola
/// al outbox `whatsapp` (`motivo: manual`): antes solo quedaba en el panel
/// y al cliente de `WhatsApp` no le llegaba nada.
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
    /* Secuencia con `insert_message_seq` (reseed + retry 23505): el contador
     * en memoria del hub vuelve a 1 en cada reinicio y colisiona. */
    let msg = glory_agent::persistence::insert_message_seq(
        &state.pool,
        &state.hub,
        id,
        "staff",
        &body,
        None,
        None,
    )
    .await
    .map_err(|e| fail(&e))?;
    let seq = msg.sequence_num;
    let _ = state
        .hub
        .broadcast(id, &glory_agent::models::WsServerMessage::live(msg));
    if let Some((canal, telefono)) =
        crate::repositories::ClienteRepository::hilo_whatsapp(&state.pool, id)
            .await
            .map_err(AppError::from)?
    {
        let destino = telefono.as_deref().map(str::trim).filter(|v| !v.is_empty());
        if let Some(destino) = destino {
            glory_agent::persistence::enqueue_outbox(
                &state.pool,
                "whatsapp",
                serde_json::json!({
                    "session_id": id,
                    "destino": destino,
                    "texto": body,
                    "via": canal,
                    "motivo": "manual",
                }),
            )
            .await
            .map_err(|e| fail(&e))?;
        }
    }
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
    /* [289A-2] Igual que Responder: `insert_message_seq`, nunca el contador
     * en memoria del hub (colisiona tras reinicio). */
    let msg = glory_agent::persistence::insert_message_seq(
        &state.pool,
        &state.hub,
        id,
        "staff",
        &nota,
        None,
        None,
    )
    .await
    .map_err(|e| fail(&e))?;
    let seq = msg.sequence_num;
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

/* [08AA-6] La config editable vive en `chat_staff_config` (split god-object). */

/* [279A-2 F5] Consola dueña (backend operable por HTTP; el panel web existe
 * para bandeja/hilo/responder y estos endpoints lo extienden sin N+1).
 * Ejemplos (con JWT `$T`):
 * `curl -H "Authorization: Bearer $T" /api/admin/agent/clientes?query=0412`
 * `curl -X POST -H "Authorization: Bearer $T" /api/admin/agent/enviar
 *   -d '{"telefono":"04120825234","texto":"Hola, soy MN"}'` */

#[derive(Debug, Deserialize)]
struct FiltroClientes {
    query: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct ClienteResumen {
    id: Uuid,
    nombre: Option<String>,
    telefono: String,
    origen: String,
    interes: Option<String>,
    presupuesto: Option<String>,
    zona: Option<String>,
    notas: Option<String>,
    sesiones: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Lista de clientes con nº de sesiones (UNA consulta, sin N+1).
async fn listar_clientes(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(f): Query<FiltroClientes>,
) -> Result<Json<Vec<ClienteResumen>>, AppError> {
    let limit = f.limit.unwrap_or(50).clamp(1, 200);
    let q = f.query.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let filas: Vec<ClienteResumen> = sqlx::query_as(
        "SELECT c.id, c.nombre, c.telefono, c.origen, c.interes, c.presupuesto, c.zona, \
          c.notas, (SELECT COUNT(*) FROM canal_sesiones cs WHERE cs.cliente_id = c.id) AS sesiones, \
          c.created_at, c.updated_at \
         FROM clientes c \
         WHERE ($1::TEXT IS NULL OR c.nombre ILIKE '%' || $1 || '%' OR c.telefono ILIKE '%' || $1 || '%') \
         ORDER BY c.updated_at DESC LIMIT $2",
    )
    .bind(q)
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(filas))
}

#[derive(Debug, Deserialize)]
struct NuevoCliente {
    nombre: Option<String>,
    telefono: String,
    origen: Option<String>,
}

/// Alta manual de cliente (la dueña lo crea y luego le envía por `/enviar`).
async fn crear_cliente(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<NuevoCliente>,
) -> Result<Json<crate::models::Cliente>, AppError> {
    let telefono = input.telefono.trim();
    if !super::chat_tools::telefono_valido(telefono) {
        return Err(AppError::BadRequest("telefono invalido".to_string()));
    }
    let origen = input.origen.as_deref().map_or("web", str::trim);
    if !matches!(origen, "web" | "wa_a" | "wa_b") {
        return Err(AppError::BadRequest(
            "origen debe ser web|wa_a|wa_b".to_string(),
        ));
    }
    let nombre = input
        .nombre
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if let Some(n) = nombre {
        if n.len() > 80 {
            return Err(AppError::BadRequest("nombre <=80".to_string()));
        }
    }
    let normalizado = crate::repositories::ClienteRepository::normalizar_telefono(telefono);
    let fila = crate::repositories::ClienteRepository::registrar_con_origen(
        &state.pool,
        nombre,
        &normalizado,
        origen,
    )
    .await?;
    Ok(Json(crate::models::Cliente::from_row(fila)))
}

#[derive(Debug, Deserialize)]
struct CambioCliente {
    nombre: Option<String>,
    interes: Option<String>,
    presupuesto: Option<String>,
    zona: Option<String>,
    notas: Option<String>,
}

/// Ficha comercial editable (interés/presupuesto/zona/notas los completa F4
/// con memoria; hoy los edita la dueña a mano).
async fn actualizar_cliente(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<CambioCliente>,
) -> Result<Json<crate::models::Cliente>, AppError> {
    for (campo, v) in [
        ("nombre", &input.nombre),
        ("interes", &input.interes),
        ("presupuesto", &input.presupuesto),
        ("zona", &input.zona),
        ("notas", &input.notas),
    ] {
        if let Some(t) = v {
            let tope = if campo == "notas" { 2000 } else { 200 };
            if t.trim().is_empty() || t.len() > tope {
                return Err(AppError::BadRequest(format!(
                    "{campo} requerido (1..{tope})"
                )));
            }
        }
    }
    if input.nombre.is_none()
        && input.interes.is_none()
        && input.presupuesto.is_none()
        && input.zona.is_none()
        && input.notas.is_none()
    {
        return Err(AppError::BadRequest("nada que cambiar".to_string()));
    }
    let fila: Option<crate::models::ClienteRow> = sqlx::query_as(
        "UPDATE clientes SET \
          nombre = COALESCE($2, nombre), interes = COALESCE($3, interes), \
          presupuesto = COALESCE($4, presupuesto), zona = COALESCE($5, zona), \
          notas = COALESCE($6, notas), updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, nombre, telefono, origen, interes, presupuesto, zona, \
           notas, created_at, updated_at",
    )
    .bind(id)
    .bind(input.nombre.as_deref().map(str::trim))
    .bind(input.interes.as_deref().map(str::trim))
    .bind(input.presupuesto.as_deref().map(str::trim))
    .bind(input.zona.as_deref().map(str::trim))
    .bind(input.notas.as_deref().map(str::trim))
    .fetch_optional(&state.pool)
    .await?;
    let Some(row) = fila else {
        return Err(AppError::NotFound("cliente no existe".to_string()));
    };
    Ok(Json(crate::models::Cliente::from_row(row)))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct SesionDeCliente {
    session_id: Uuid,
    canal: Option<String>,
    telefono: Option<String>,
    modo: Option<String>,
    estado_atencion: Option<String>,
    status: Option<String>,
    ai_enabled: Option<bool>,
    last_body: Option<String>,
    last_sender: Option<String>,
    #[sqlx(rename = "last_at")]
    last_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Sesiones de un cliente (sus hilos web/WhatsApp enlazados por `clientes`).
async fn sesiones_de_cliente(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SesionDeCliente>>, AppError> {
    let filas: Vec<SesionDeCliente> = sqlx::query_as(
        "SELECT cs.session_id, cs.canal, cs.telefono, cs.modo, a.estado AS estado_atencion, \
          s.status, s.ai_enabled, \
          m.body AS last_body, m.sender AS last_sender, m.created_at AS last_at \
         FROM canal_sesiones cs \
         JOIN agent_sessions s ON s.id = cs.session_id \
         LEFT JOIN atencion_sesiones a ON a.session_id = cs.session_id \
         LEFT JOIN LATERAL (SELECT body, sender, created_at FROM agent_messages \
           WHERE session_id = cs.session_id ORDER BY sequence_num DESC LIMIT 1) m ON true \
         WHERE cs.cliente_id = $1 ORDER BY s.updated_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(filas))
}

#[derive(Debug, Deserialize)]
struct EnvioManual {
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
        let existe = glory_agent::persistence::get_session(&state.pool, sid)
            .await
            .map_err(|e| fail(&e))?;
        if existe.is_none() {
            return Err(AppError::NotFound("sesion no existe".to_string()));
        }
        let tel: Option<String> =
            sqlx::query_scalar("SELECT telefono FROM canal_sesiones WHERE session_id = $1")
                .bind(sid)
                .fetch_optional(&state.pool)
                .await?;
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
async fn enviar_manual(
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
struct FiltroUso {
    dias: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct UsoDia {
    dia: Option<chrono::NaiveDate>,
    remitente: Option<String>,
    mensajes: Option<i64>,
    tokens_est: Option<i64>,
    tokens_in: Option<i64>,
    tokens_out: Option<i64>,
}

/// Uso (mensajes y tokens por día×remitente; `tokens_in/out` exactos del
/// núcleo F0 en mensajes `ai`, estima `len/4` del trigger en el resto).
async fn uso_mensajes(
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

#[derive(Debug, Serialize, sqlx::FromRow)]
struct AuditoriaFila {
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
async fn auditoria(
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
async fn sesiones_whatsapp(_auth: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
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
async fn qr_whatsapp(
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

/* [08AA-6] El test de la allowlist vive con su dominio en
 * `chat_staff_config::pruebas`. */
