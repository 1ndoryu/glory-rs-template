/* [044A-38 Fase 9] Handlers de notificaciones.
 * REST: listar, marcar leídas, marcar todas leídas, conteo no leídas.
 * WS: /ws/notifications?token=JWT — push en tiempo real al usuario. */

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use futures::StreamExt;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{MarkReadBody, NotificationResponse, UnreadCountResponse};
use crate::repositories::NotificationRepository;
use crate::services::AuthService;
use crate::AppState;

/* ========== Parámetros de query ========== */

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: String,
}

/* ========== REST Handlers ========== */

/// Lista notificaciones del usuario autenticado (paginadas)
#[utoipa::path(
    get,
    path = "/api/notifications",
    params(
        ("limit" = Option<i64>, Query, description = "Límite de resultados (default 20)"),
        ("offset" = Option<i64>, Query, description = "Offset para paginación")
    ),
    responses(
        (status = 200, description = "Lista de notificaciones", body = Vec<NotificationResponse>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_notifications(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<NotificationResponse>>, AppError> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0).max(0);

    let notifications =
        NotificationRepository::list_for_user(&state.pool, auth.user_id, limit, offset).await?;

    let response: Vec<NotificationResponse> = notifications
        .into_iter()
        .map(NotificationResponse::from)
        .collect();

    Ok(Json(response))
}

/// Obtiene el conteo de notificaciones no leídas
#[utoipa::path(
    get,
    path = "/api/notifications/unread-count",
    responses(
        (status = 200, description = "Conteo de no leídas", body = UnreadCountResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_unread_count(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UnreadCountResponse>, AppError> {
    let count = NotificationRepository::count_unread(&state.pool, auth.user_id).await?;
    Ok(Json(UnreadCountResponse { count }))
}

/// Marca notificaciones específicas como leídas
#[utoipa::path(
    patch,
    path = "/api/notifications/read",
    request_body = MarkReadBody,
    responses(
        (status = 200, description = "Notificaciones marcadas como leídas")
    ),
    security(("bearer_auth" = []))
)]
pub async fn mark_read(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<MarkReadBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    /* Parsear UUIDs del body */
    let ids: Vec<Uuid> = body
        .ids
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    if ids.is_empty() {
        return Err(AppError::BadRequest(
            "No se proporcionaron IDs válidos".into(),
        ));
    }

    let affected = NotificationRepository::mark_read(&state.pool, auth.user_id, &ids).await?;

    /* Enviar conteo actualizado por WS */
    state.notification_hub.send_unread_count(auth.user_id).await;

    Ok(Json(serde_json::json!({ "marked": affected })))
}

/// Marca todas las notificaciones como leídas
#[utoipa::path(
    patch,
    path = "/api/notifications/read-all",
    responses(
        (status = 200, description = "Todas marcadas como leídas")
    ),
    security(("bearer_auth" = []))
)]
pub async fn mark_all_read(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let affected = NotificationRepository::mark_all_read(&state.pool, auth.user_id).await?;

    /* Enviar conteo actualizado (0) por WS */
    state.notification_hub.send_unread_count(auth.user_id).await;

    Ok(Json(serde_json::json!({ "marked": affected })))
}

/* ========== WebSocket Handler ========== */

/// Endpoint WS para notificaciones en tiempo real.
/// El cliente conecta con `?token=JWT` y recibe push de notificaciones + conteo.
pub async fn ws_notifications(
    State(state): State<AppState>,
    Query(params): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    /* Verificar JWT del query param */
    let claims = AuthService::verify_token(&params.token, &state.jwt_secret)
        .map_err(|_| AppError::Unauthorized)?;

    let user_id = claims.sub;

    Ok(ws.on_upgrade(move |socket| handle_notification_ws(socket, state, user_id)))
}

/// Manejo de la conexión WS de notificaciones
async fn handle_notification_ws(socket: WebSocket, state: AppState, user_id: Uuid) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = state.notification_hub.subscribe(user_id);

    /* Enviar conteo inicial de no leídas */
    if let Ok(count) = NotificationRepository::count_unread(&state.pool, user_id).await {
        let init_msg = crate::models::WsNotification::UnreadCount { count };
        if let Ok(json) = serde_json::to_string(&init_msg) {
            let _ = futures::SinkExt::send(&mut ws_sender, Message::Text(json)).await;
        }
    }

    /* Spawn tarea que reenvía del mpsc channel al WS.
     * [096A-13] subscribe() devuelve mpsc::UnboundedReceiver (lock-free recv). */
    let send_task = tokio::spawn(async move {
        while let Some(notif) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&notif) {
                if futures::SinkExt::send(&mut ws_sender, Message::Text(json))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    /* Mantener la conexión abierta: consumir pings/pongs y detectar cierre.
     * [096A-1] Timeout de inactividad: 5 minutos sin mensajes → cerrar. */
    loop {
        /* [259A-1] let-else + patrones anidados (clippy unnested_or_patterns). */
        let Ok(Some(Ok(msg))) =
            tokio::time::timeout(std::time::Duration::from_secs(300), ws_receiver.next()).await
        else {
            break;
        };
        if matches!(msg, Message::Close(_)) {
            break;
        }
        /* Responder a pings (axum lo hace automáticamente, pero por si acaso) */
    }

    send_task.abort();
}

/* ========== Routes ========== */

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/unread-count", get(get_unread_count))
        .route("/notifications/read", patch(mark_read))
        .route("/notifications/read-all", patch(mark_all_read))
}

pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws/notifications", get(ws_notifications))
}
