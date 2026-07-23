/* [P-1 Chatbot v2] Endpoints REST de mensajes: obtener y enviar mensajes.
 * Separado de rest.rs para mantener el límite de líneas por archivo.
 * send_message contiene lógica de rate limiting, restricciones de orden,
 * notificaciones, IA automática, escalación y email. */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ChatMessage, ChatMessageResponse, ChatSession, CreateNotification, SendMessageRequest,
    UserRole, NOTIF_NEW_MESSAGE,
};
use crate::repositories::OrderRepository;
use crate::services::{AiChatService, AiResponse};
use crate::AppState;

use super::{enrich_messages, MessagesQuery};

/// Obtener mensajes de una sesión
#[utoipa::path(
    get,
    path = "/api/chat/sessions/{session_id}/messages",
    params(
        ("session_id" = Uuid, Path, description = "ID de la sesión"),
        ("limit" = Option<i64>, Query, description = "Límite de mensajes"),
        ("offset" = Option<i64>, Query, description = "Offset para paginación"),
    ),
    responses(
        (status = 200, description = "Mensajes", body = Vec<ChatMessage>),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "chat"
)]
pub async fn get_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<Uuid>,
    Query(params): Query<MessagesQuery>,
) -> Result<Json<Vec<ChatMessageResponse>>, AppError> {
    authorized_session(&state, &auth, session_id).await?;
    /* [237A-5] Límites negativos no cruzan el boundary y el repositorio devuelve
     * siempre la ventana más reciente en orden cronológico. */
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);
    let messages =
        crate::repositories::ChatRepository::list_messages(&state.pool, session_id, limit, offset)
            .await?;

    /* [064A-70] Enriquecer mensajes con avatar + nombre del sender */
    let enriched = enrich_messages(&state.pool, messages).await;

    Ok(Json(enriched))
}

fn sender_type_for(role: crate::models::UserRole) -> &'static str {
    match role {
        crate::models::UserRole::Admin => "admin",
        crate::models::UserRole::Employee => "employee",
        crate::models::UserRole::Client => "client",
    }
}

/* [237A-5] La persistencia amplía el historial visible, por lo que cada lectura
 * y escritura vuelve a verificar participantes en el boundary. Admin supervisa;
 * cliente y empleado solo acceden a conversaciones propias/asignadas. */
pub(super) async fn authorized_session(
    state: &AppState,
    auth: &AuthUser,
    session_id: Uuid,
) -> Result<ChatSession, AppError> {
    let session = crate::repositories::ChatRepository::find_session_by_id(&state.pool, session_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Conversación no encontrada".into()))?;

    if auth.effective_role == UserRole::Admin {
        return Ok(session);
    }

    let authorized = if let Some(order_id) = session.order_id {
        OrderRepository::get_order_participants(&state.pool, order_id)
            .await?
            .is_some_and(|(client_id, assigned_staff_id)| {
                client_id == auth.user_id || assigned_staff_id == Some(auth.user_id)
            })
    } else {
        session.user_id == Some(auth.user_id) || session.assigned_staff_id == Some(auth.user_id)
    };

    if !authorized {
        return Err(AppError::Forbidden(
            "No tienes acceso a esta conversación.".to_string(),
        ));
    }
    Ok(session)
}

/* [174A-2][237A-5] Política de escritura en chat de orden. */
async fn enforce_order_chat_sender_permission(
    state: &AppState,
    session: &ChatSession,
    auth: &AuthUser,
) -> Result<(), AppError> {
    if let Some(order_id) = session.order_id {
        let participants = OrderRepository::get_order_participants(&state.pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden de la conversación no encontrada".into()))?;
        let is_admin = auth.effective_role == UserRole::Admin;
        let is_client = auth.user_id == participants.0;
        let is_assigned = participants.1 == Some(auth.user_id);
        if !is_admin && !is_client && !is_assigned {
            return Err(AppError::Forbidden(
                "Solo el cliente, el empleado asignado o un administrador pueden enviar mensajes."
                    .to_string(),
            ));
        }

        if is_assigned {
            OrderRepository::toggle_ai_intermediary(&state.pool, order_id, false).await?;
        }
    }
    Ok(())
}

/* [174A-2] Mantiene la notificación al interlocutor fuera del flujo principal del envío.
 * [20CA-10] Además notifica a todos los admins cuando un visitante/cliente envía mensaje. */
async fn notify_chat_recipient(
    state: &AppState,
    session_id: Uuid,
    sender_type: &str,
    content: &str,
) {
    if let Ok(Some(session)) =
        crate::repositories::ChatRepository::find_session_by_id(&state.pool, session_id).await
    {
        let recipient_id = if sender_type == "client" {
            session.assigned_staff_id
        } else if let Some(order_id) = session.order_id {
            OrderRepository::client_id_by_id(&state.pool, order_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        if let Some(recipient_id) = recipient_id {
            let preview: String = content.chars().take(80).collect();
            let panel_link = session.order_id.map_or_else(
                || format!("/panel?seccion=mensajes&chat={session_id}"),
                |order_id| format!("/panel?order={order_id}"),
            );
            let _ = state
                .notification_hub
                .notify(CreateNotification {
                    user_id: recipient_id,
                    notification_type: NOTIF_NEW_MESSAGE.to_string(),
                    title: "Nuevo mensaje en el chat".to_string(),
                    body: Some(preview),
                    link: Some(panel_link),
                    reference_type: Some("chat_session".to_string()),
                    reference_id: Some(session_id),
                })
                .await;
        }

        /* [20CA-10] Si el remitente es visitante/cliente, notificar también a todos los admins
         * para que ningún mensaje quede desatendido. Excluir al staff asignado (ya notificado arriba). */
        if sender_type == "visitor" || sender_type == "user" {
            if let Ok(admin_ids) = crate::repositories::UserRepository::admin_ids(&state.pool).await
            {
                let admins_excluding_staff: Vec<Uuid> = admin_ids
                    .into_iter()
                    .filter(|id| Some(*id) != session.assigned_staff_id)
                    .collect();
                if !admins_excluding_staff.is_empty() {
                    let preview: String = content.chars().take(80).collect();
                    let panel_link = session.order_id.map_or_else(
                        || format!("/panel?seccion=mensajes&chat={session_id}"),
                        |order_id| format!("/panel?order={order_id}"),
                    );
                    let base = CreateNotification {
                        user_id: Uuid::nil(),
                        notification_type: NOTIF_NEW_MESSAGE.to_string(),
                        title: "Nuevo mensaje de cliente en chat".to_string(),
                        body: Some(preview),
                        link: Some(panel_link),
                        reference_type: Some("chat_session".to_string()),
                        reference_id: Some(session_id),
                    };
                    let _ = state
                        .notification_hub
                        .notify_many(&admins_excluding_staff, &base)
                        .await;
                }
            }
        }
    }
}

/// Enviar mensaje REST (alternativa a WebSocket)
#[utoipa::path(
    post,
    path = "/api/chat/sessions/{session_id}/messages",
    params(("session_id" = Uuid, Path, description = "ID de la sesión")),
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Mensaje enviado", body = ChatMessage),
        (status = 401, description = "No autorizado"),
    ),
    security(("bearer_auth" = [])),
    tag = "chat"
)]
/* [035A-21] Handler orquestador legacy: valida rate-limit, permisos, persistencia y broadcast.
 * Se permite sobrepasar el umbral mientras el flujo REST de chat siga consolidado en un solo endpoint. */
#[allow(clippy::too_many_lines)]
pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<ChatMessage>), AppError> {
    let session = authorized_session(&state, &auth, session_id).await?;
    if session.status == "closed" {
        return Err(AppError::Conflict(
            "La conversación está archivada y es de solo lectura.".to_string(),
        ));
    }

    /* [104A-36] Rate limiting para REST — reutiliza ChatTimingService con user_id como key.
     * Solo aplica a clientes; staff/admin no tienen límite. */
    if auth.effective_role == crate::models::UserRole::Client {
        let (result, _msg) = state.chat_timing.check_rate(&auth.user_id.to_string());
        if matches!(result, crate::services::RateCheckResult::Muted) {
            return Err(AppError::TooManyRequests(
                "Has enviado demasiados mensajes. Espera un momento.".to_string(),
            ));
        }
    }

    let sender_type = sender_type_for(auth.effective_role);

    /* [154A-15e] Restricción de chat de orden: solo 2 personas (cliente + empleado asignado).
     * Admin solo puede enviar si es el empleado asignado. Si no es sesión de orden, sin restricción. */
    enforce_order_chat_sender_permission(&state, &session, &auth).await?;

    let sender_id = auth.user_id.to_string();

    let msg = state
        .chat_hub
        .send_message(session_id, sender_type, Some(&sender_id), &req.content)
        .await?;

    /* [104A-38] Notificar al otro participante del chat (solo sesiones de orden).
     * Cliente envía → notificar staff asignado. Staff/admin envía → notificar cliente. */
    notify_chat_recipient(&state, session_id, sender_type, &req.content).await;

    /* Si IA habilitada y sender es client, generar respuesta */
    if sender_type == "client" {
        if let Ok(Some(s)) =
            crate::repositories::ChatRepository::find_session_by_id(&state.pool, session_id).await
        {
            /* [T-10] IA intermediaria: si sesión de orden con intermediary habilitado */
            if let Some(order_id) = s.order_id {
                spawn_intermediary_if_enabled(
                    &state,
                    session_id,
                    order_id,
                    auth.user_id,
                    &req.content,
                );
            } else if s.ai_enabled {
                handle_ai_and_escalation(&state, session_id, &s, &auth, &req.content).await;
            }
        }
    }

    Ok((StatusCode::CREATED, Json(msg)))
}

/* [174A-2] Extraído de send_message: genera respuesta IA, envía rich messages y escala si necesario. */
async fn handle_ai_and_escalation(
    state: &AppState,
    session_id: Uuid,
    session: &crate::models::ChatSession,
    auth: &AuthUser,
    content: &str,
) {
    if content.len() > crate::services::ChatTimingService::max_ai_combined_chars() {
        let _ = state
            .chat_hub
            .send_message(
                session_id,
                "ai",
                Some("ai"),
                "Recibí demasiado texto de golpe. Envíame un resumen más corto o espera a una persona del equipo.",
            )
            .await;
        return;
    }

    let budget_key = session
        .visitor_id
        .clone()
        .unwrap_or_else(|| session_id.to_string());
    let (budget_result, budget_msg) = state
        .chat_timing
        .check_visitor_ai_budget(&budget_key, content);
    if !matches!(budget_result, crate::services::RateCheckResult::Ok) {
        let _ = state
            .chat_hub
            .send_message(
                session_id,
                "ai",
                Some("ai"),
                budget_msg
                    .as_deref()
                    .unwrap_or("Límite temporal del asistente alcanzado."),
            )
            .await;
        return;
    }

    let ai_resp = AiChatService::generate_response(
        &state.pool,
        &state.ai_config,
        &state.http_client,
        state.stripe_secret_key.as_deref(),
        crate::services::AiSessionContext {
            session_id,
            visitor_id: session.visitor_id.as_deref(),
            auth: Some(crate::services::ai_tools::ToolAuthContext::new(
                auth.user_id,
                auth.role,
                auth.effective_role,
                auth.impersonator,
            )),
            user_id: Some(auth.user_id),
            context: None,
        },
        content,
    )
    .await
    .unwrap_or_else(|e| crate::services::AiResponse {
        text: format!("Error IA: {e}"),
        needs_escalation: true,
        rich_messages: Vec::new(),
    });

    /* [T-2] Enviar rich messages antes del texto */
    for rm in &ai_resp.rich_messages {
        let _ = state
            .chat_hub
            .send_rich_message(
                session_id,
                "ai",
                Some("ai"),
                &rm.content,
                &rm.message_type,
                &rm.metadata,
            )
            .await;
    }

    if let Err(err) = state
        .chat_hub
        .send_message(session_id, "ai", Some("ai"), &ai_resp.text)
        .await
    {
        tracing::error!("No se pudo guardar la respuesta AI en sesión {session_id}: {err}");
    }

    /* [T-6] [114A-8] Escalación: notificar admins + enviar email */
    if ai_resp.needs_escalation {
        let visitor = session.visitor_name.as_deref().unwrap_or("Visitante");
        notify_escalation(state, session_id, visitor).await;
    }
}

/* [174A-2] Notifica a admins de escalación via notificación in-app + email */
async fn notify_escalation(state: &AppState, session_id: Uuid, visitor: &str) {
    if let Ok(admin_ids) = crate::repositories::UserRepository::admin_ids(&state.pool).await {
        if !admin_ids.is_empty() {
            let base = crate::models::CreateNotification {
                user_id: uuid::Uuid::nil(),
                notification_type: crate::models::NOTIF_ESCALATION_NEEDED.to_string(),
                title: format!("Escalación: {visitor} necesita ayuda"),
                body: Some(
                    "La IA detectó que se requiere intervención humana en la sesión de chat."
                        .to_string(),
                ),
                link: Some(format!("/admin/chat?session={session_id}")),
                reference_type: Some("chat_session".to_string()),
                reference_id: Some(session_id),
            };
            let _ = state.notification_hub.notify_many(&admin_ids, &base).await;
        }
    }

    if let Some(ref email_cfg) = state.email_config {
        if let Ok(emails) = crate::repositories::UserRepository::admin_emails(&state.pool).await {
            if !emails.is_empty() {
                let site_url = std::env::var("SITE_URL")
                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                crate::services::EmailService::send_escalation_emails(
                    email_cfg,
                    &state.pool,
                    &emails,
                    visitor,
                    session_id,
                    &site_url,
                )
                .await;
            }
        }
    }
}

/* [T-10] Lanza respuesta IA intermediaria en background si la orden lo tiene habilitado */
fn spawn_intermediary_if_enabled(
    state: &AppState,
    session_id: Uuid,
    order_id: Uuid,
    user_id: Uuid,
    content: &str,
) {
    let pool = state.pool.clone();
    let ai_config = state.ai_config.clone();
    let http_client = state.http_client.clone();
    let hub = state.chat_hub.clone();
    let content = content.to_string();
    tokio::spawn(async move {
        /* Verificar si la orden tiene intermediary habilitado */
        let order =
            match crate::repositories::OrderRepository::find_order_by_id(&pool, order_id).await {
                Ok(Some(o)) if o.ai_intermediary_enabled.unwrap_or(false) => o,
                _ => return,
            };

        /* Generar respuesta con prompt de intermediario */
        let ai_resp = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            AiChatService::generate_intermediary_response(
                &pool,
                &ai_config,
                &http_client,
                session_id,
                &order,
                user_id,
                &content,
            ),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                /* [035A-12] El intermediario no puede fallar en silencio.
                 * Si el proveedor IA cae, dejar acuse visible al cliente. */
                tracing::error!("Error IA intermediaria orden #{}: {e}", order.order_number);
                AiResponse {
                    text: "He recibido tu mensaje y lo compartiré con el equipo. Te responderemos pronto.".to_string(),
                    needs_escalation: true,
                    rich_messages: Vec::new(),
                }
            }
            Err(_) => {
                tracing::error!(
                    "Timeout IA intermediaria orden #{} tras 10s; usando acuse de respaldo",
                    order.order_number,
                );
                AiResponse {
                    text: "He recibido tu mensaje y lo compartiré con el equipo. Te responderemos pronto.".to_string(),
                    needs_escalation: true,
                    rich_messages: Vec::new(),
                }
            }
        };

        /* Enviar respuesta como ai_intermediary */
        if let Err(err) = hub
            .send_message(session_id, "ai_intermediary", Some("ai"), &ai_resp.text)
            .await
        {
            tracing::error!(
                "No se pudo guardar la respuesta IA intermediaria en sesión {session_id}: {err}",
            );
            return;
        }

        /* [T-10c] Auto-resumen si >5 mensajes en la sesión */
        if let Ok(msgs) =
            crate::repositories::ChatRepository::list_messages(&pool, session_id, 100, 0).await
        {
            if msgs.len() > 5 {
                AiChatService::maybe_update_order_summary(
                    &pool,
                    &ai_config,
                    &http_client,
                    order_id,
                    &msgs,
                )
                .await;
            }
        }
    });
}
