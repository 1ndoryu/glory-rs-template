/* [P-1 Chatbot v2] WS handler del visitante (anónimo o cliente).
 * Endpoint: /ws/chat/visitor?visitor_id=X&visitor_name=Y
 * Captura IP y User-Agent, reutiliza sesión activa, reenvía historial.
 * [T-1] Usa ChatTimingService para rate limiting y timing inteligente. */

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;

use crate::models::WsServerMessage;
use crate::repositories::{ChatRepository, UserRepository};
use crate::services::TimingEvent;
use crate::AppState;
use uuid::Uuid;

use super::ws_visitor_helpers::{process_visitor_messages, send_history};
use super::VisitorWsParams;

/* ============================================================
WEBSOCKET: VISITOR (anónimo o cliente autenticado)
============================================================ */

async fn ws_visitor(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Query(mut params): Query<VisitorWsParams>,
) -> impl IntoResponse {
    /* [237A-5] visitor_id es una identidad persistente, no texto libre.
     * Rechazar basura legacy impide que "undefined"/"null" mezclen historiales. */
    let Ok(visitor_uuid) = Uuid::parse_str(params.visitor_id.trim()) else {
        tracing::warn!(visitor_id = %params.visitor_id, "WS visitor rechazado: visitor_id inválido");
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "visitor_id debe ser un UUID válido",
        )
            .into_response();
    };
    params.visitor_id = visitor_uuid.to_string();

    /* [064A-72] [074A-41] Capturar IP (proxy headers → fallback a socket) y User-Agent */
    let (visitor_ip, visitor_ua, visitor_country) = extract_visitor_context(&headers, addr);
    let visitor_country_clone = visitor_country.clone();
    let visitor_ip_clone = visitor_ip.clone();
    let http_client_clone = state.http_client.clone();

    ws.on_upgrade(move |socket| async move {
        /* Geo-lookup asíncrono solo si CF no dio el país */
        let country = if visitor_country_clone.is_some() {
            visitor_country_clone
        } else {
            lookup_country_from_ip(&http_client_clone, visitor_ip_clone.as_deref()).await
        };
        handle_visitor_ws(socket, state, params, visitor_ip, visitor_ua, country).await;
    })
    .into_response()
}

/* [124A-PAIS] Geo-lookup por IP via ipapi.co (fallback cuando no hay CF-IPCountry).
 * Timeout 2s para no bloquear. Ignora IPs locales/privadas. */
async fn lookup_country_from_ip(client: &reqwest::Client, ip: Option<&str>) -> Option<String> {
    let ip = ip?;
    /* Saltar IPs privadas/loopback */
    if ip == "127.0.0.1"
        || ip.starts_with("192.168.")
        || ip.starts_with("10.")
        || ip.starts_with("172.16.")
        || ip == "::1"
    {
        return None;
    }
    let url = format!("https://ipapi.co/{ip}/country_name/");
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        client
            .get(&url)
            .header("User-Agent", "nakomi-studio/1.0")
            .send(),
    )
    .await
    {
        Ok(Ok(resp)) if resp.status().is_success() => resp
            .text()
            .await
            .ok()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty() && !t.to_lowercase().contains("error")),
        _ => None,
    }
}

fn try_extract_auth_context(
    token: Option<&str>,
    jwt_secret: &str,
) -> Option<crate::services::ai_tools::ToolAuthContext> {
    token.and_then(|t| {
        crate::services::AuthService::verify_token(t, jwt_secret)
            .ok()
            .map(|claims| {
                crate::services::ai_tools::ToolAuthContext::new(
                    claims.sub,
                    claims.role,
                    claims.effective_role,
                    claims.impersonator,
                )
            })
    })
}

fn extract_visitor_context(
    headers: &axum::http::HeaderMap,
    addr: SocketAddr,
) -> (Option<String>, Option<String>, Option<String>) {
    let visitor_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        })
        .or_else(|| Some(addr.ip().to_string()));
    let visitor_ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let visitor_country = headers
        .get("cf-ipcountry")
        .and_then(|v| v.to_str().ok())
        .filter(|&c| c != "XX" && c != "T1")
        .map(String::from);

    (visitor_ip, visitor_ua, visitor_country)
}

fn spawn_visitor_send_task(
    mut sender: futures::stream::SplitSink<WebSocket, Message>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<WsServerMessage>,
) -> tokio::task::JoinHandle<()> {
    /* [096A-13] mpsc::UnboundedReceiver: recv() retorna Some(msg) o None (canal cerrado).
     * No existe Lagged como en broadcast — el canal es FIFO ilimitado.
     * [096A-11] Timeout de 5s en cada write WebSocket. Si el cliente está muerto
     * (conexión TCP sin FIN/RST), el write se bloquea indefinidamente.
     * Con timeout, la tarea aborta y libera el receiver. */
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    sender.send(Message::Text(json)),
                )
                .await
                {
                    Ok(Ok(())) => { /* enviado OK */ }
                    Ok(Err(_)) => {
                        tracing::debug!("Visitor WS write error, cerrando send task");
                        break;
                    }
                    Err(_) => {
                        tracing::warn!("Visitor WS write timeout (5s), abortando send task");
                        break;
                    }
                }
            }
        }
        /* Canal cerrado: cleanup normal, sin Lagged */
    })
}

fn register_visitor_timing_session(
    state: &AppState,
    session: &crate::models::ChatSession,
    session_id: Uuid,
    params: &VisitorWsParams,
    auth: Option<crate::services::ai_tools::ToolAuthContext>,
    visitor_ip: Option<String>,
) -> tokio::sync::mpsc::Sender<TimingEvent> {
    let user_id = auth.map(|auth| auth.user_id);
    state.chat_timing.register_session(
        session_id,
        session.visitor_name.clone(),
        crate::services::TimingSessionDeps {
            pool: state.pool.clone(),
            ai_config: state.ai_config.clone(),
            chat_timing: state.chat_timing.clone(),
            hub: state.chat_hub.clone(),
            notification_hub: state.notification_hub.clone(),
            http_client: state.http_client.clone(),
            stripe_key: state.stripe_secret_key.clone(),
            visitor_id: params.visitor_id.clone(),
            client_ip: visitor_ip,
            user_id,
            auth,
            context: params.context.clone(),
            email_config: state.email_config.clone(),
        },
    )
}

/* [095A-16] Persistir identidad autenticada fuera del prompt inmediato.
 * El JWT ya llega por WebSocket; este enlace deja la sesión y el visitor_profile
 * preparados para historial, panel y futuras tools con permisos reales. */
async fn link_authenticated_visitor_context(
    state: &AppState,
    session_id: Uuid,
    visitor_id: &str,
    user_id: Uuid,
) {
    if let Err(e) = ChatRepository::link_session_to_user(&state.pool, session_id, user_id).await {
        tracing::warn!("Error vinculando chat_session {session_id} a user {user_id}: {e}");
    }
    if let Err(e) = ChatRepository::link_visitor_to_user(&state.pool, visitor_id, user_id).await {
        tracing::warn!("Error vinculando visitor_profile a user {user_id}: {e}");
    }
    if let Ok(Some(user)) = UserRepository::find_by_id(&state.pool, user_id).await {
        let display_name = user.display_name.as_deref().unwrap_or(&user.username);
        if let Err(e) = ChatRepository::update_visitor_email(
            &state.pool,
            visitor_id,
            &user.email,
            Some(display_name),
        )
        .await
        {
            tracing::warn!("Error enriqueciendo visitor_profile autenticado {user_id}: {e}");
        }
    }
}

/* [035A-21] Orquestador WS legacy: handshake, anti-bot, spawn de tareas y cleanup final.
 * Se documenta la excepción hasta dividir el flujo por fases sin cambiar comportamiento. */
#[allow(clippy::too_many_lines)]
async fn handle_visitor_ws(
    socket: WebSocket,
    state: AppState,
    params: VisitorWsParams,
    visitor_ip: Option<String>,
    visitor_ua: Option<String>,
    visitor_country: Option<String>,
) {
    /* [084A-42] Anti-bot: verificar límite de conexiones WS por IP */
    let ip_for_tracking = visitor_ip.clone().unwrap_or_default();
    if !ip_for_tracking.is_empty() && !state.chat_timing.track_ip_connect(&ip_for_tracking) {
        tracing::warn!("Anti-bot: rechazada conexión WS de IP {ip_for_tracking}");
        return;
    }

    /* [T-9] Detectar usuario autenticado si envió token JWT */
    let auth = try_extract_auth_context(params.token.as_deref(), &state.jwt_secret);
    if let Some(auth) = auth {
        tracing::info!(
            user_id = %auth.user_id,
            role = %auth.role,
            effective_role = %auth.effective_role,
            impersonator = ?auth.impersonator,
            "Chat visitor autenticado"
        );
    }

    let session = match state
        .chat_hub
        .get_or_create_visitor_session(
            &params.visitor_id,
            params.visitor_name.as_deref(),
            visitor_ip.as_deref(),
            visitor_ua.as_deref(),
            visitor_country.as_deref(),
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Error creando sesión visitor: {e}");
            return;
        }
    };

    let session_id = session.id;
    tracing::info!(%session_id, visitor_id = %params.visitor_id, "WS session obtenida/creada");
    let rx = state.chat_hub.subscribe(session_id);
    let (mut sender, mut receiver) = socket.split();

    /* [T-3] Upsert visitor_profile: crea o actualiza perfil persistente.
     * Trackea IPs, fingerprints, sesiones. Usado para contexto IA. */
    if let Err(e) = ChatRepository::upsert_visitor_profile(
        &state.pool,
        &params.visitor_id,
        visitor_ip.as_deref(),
        visitor_ua.as_deref(),
    )
    .await
    {
        tracing::warn!("Error upserting visitor_profile: {e}");
    }

    /* [095A-16] Identidad persistente: si el widget trae JWT, vincular la sesión
     * y el perfil del visitor al usuario real. Así la IA recibe contexto registrado
     * y el panel puede distinguir cliente/admin aunque el chat haya nacido como visitor. */
    if let Some(auth) = auth {
        link_authenticated_visitor_context(&state, session_id, &params.visitor_id, auth.user_id)
            .await;
    }

    /* [064A-29] Enviar historial de mensajes previos al reconectar */
    let Ok(had_messages) = send_history(&state, session_id, &mut sender).await else {
        return;
    };

    /* [124A-VISIT1] Auto-greeting eliminado: creaba una sesión visible en el panel
     * por cada visitante que abre el chat, incluso sin escribir. La IA responde
     * en cuanto el visitante envía su primer mensaje. */
    let _ = had_messages; /* variable usada para suprimir unused warning */

    /* [104A-40] Registrar timestamp de conexión y notificar al staff que el visitante está online.
     * Sirve como confirmación de lectura: si el visitante está online, vio los mensajes. */
    let visitor_online_at: chrono::DateTime<chrono::Utc> =
        crate::repositories::ChatRepository::update_visitor_last_connected(&state.pool, session_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Error actualizando visitor_last_connected_at: {e}");
                chrono::Utc::now()
            });
    state
        .chat_hub
        .notify_visitor_online(session_id, visitor_online_at)
        .await;

    /* Notificar a staff de nueva sesión */
    state.chat_hub.broadcast(
        session_id,
        &WsServerMessage::SessionNew {
            session: session.clone(),
        },
    );

    /* Task: enviar mensajes del broadcast al WS del visitante */
    let send_task = spawn_visitor_send_task(sender, rx);

    /* [T-1] Registrar sesión en timing service */
    let timing_tx = register_visitor_timing_session(
        &state,
        &session,
        session_id,
        &params,
        auth,
        visitor_ip.clone(),
    );

    /* Procesar mensajes del visitante. Retorna true si el cierre fue explícito
     * (usuario cerró chat o rate limit forzó cierre). */
    tracing::info!(%session_id, "Esperando mensajes del visitante...");
    let explicit_close = process_visitor_messages(
        &mut receiver,
        &state,
        session_id,
        &params.visitor_id,
        visitor_ip.as_deref(),
        &timing_tx,
    )
    .await;

    cleanup_visitor_session(
        &state,
        session_id,
        &ip_for_tracking,
        explicit_close,
        visitor_online_at,
        &timing_tx,
    )
    .await;
    send_task.abort();
}

/* [174A-2] Cleanup multi-conexión: unsubscribe, offline, disconnect, IP tracking */
async fn cleanup_visitor_session(
    state: &AppState,
    session_id: Uuid,
    ip_for_tracking: &str,
    explicit_close: bool,
    visitor_online_at: chrono::DateTime<chrono::Utc>,
    timing_tx: &tokio::sync::mpsc::Sender<TimingEvent>,
) {
    let remaining = state.chat_hub.unsubscribe(session_id);
    if explicit_close {
        state
            .chat_hub
            .notify_visitor_offline(session_id, Some(visitor_online_at))
            .await;
        /* [096A-8] try_send: si el timing loop está ocupado con IA, no bloquear cleanup.
         * unregister_session() eliminará el sender del DashMap, lo que cerrará el canal
         * y el timing loop terminará por sí solo al hacer recv(). */
        let _ = timing_tx.try_send(TimingEvent::Disconnect);
        state.chat_timing.unregister_session(session_id);
        let _ = state.chat_hub.close_session(session_id).await;
    } else if remaining == 0 {
        /* [257A-5] Recargar, perder red o reconectar no cancela el timing loop.
         * El mensaje ya fue persistido y puede estar esperando el debounce de
         * escritura; enviar Disconnect aquí vaciaba el buffer antes de que la IA
         * respondiera. El loop queda vivo para que la nueva conexión reutilice su
         * sender y se autolimpia al cerrar de verdad o al timeout global. */
        state
            .chat_hub
            .notify_visitor_offline(session_id, Some(visitor_online_at))
            .await;
    }
    if !ip_for_tracking.is_empty() {
        state.chat_timing.track_ip_disconnect(ip_for_tracking);
    }
}

/* ============================================================
ROUTES (WebSocket — montadas en root, no bajo /api)
============================================================ */

pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws/chat/visitor", get(ws_visitor))
}
