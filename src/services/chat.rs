/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: chat service usa runtime
 * queries para operaciones de broadcast y persistencia con tipos dinámicos. */
/* [044A-38 Fase 5] ChatHub: gestión de conexiones WebSocket y routing de mensajes.
 * DashMap para sesiones activas en memoria. Cada sesión tiene múltiples suscriptores
 * (visitante/cliente + staff). Los mensajes se persisten en BD y se broadcastean.
 * [096A-13] mpsc::unbounded_channel reemplaza broadcast::channel.
 *   - broadcast::Sender::send() usa std::sync::Mutex + Notify internamente,
 *     lo que bloquea workers tokio en futex_wait bajo contención (8 workers → freeze total).
 *   - mpsc::UnboundedSender::send() es lock-free: nunca bloquea el OS thread.
 *   - Cada suscriptor tiene su propio canal mpsc individual (no compartido).
 *   - broadcast() itera el Vec de senders y hace retain() limpiando caídos.
 * [096A-14] staff_senders migrado a tokio::sync::Mutex: std::sync::Mutex bloqueaba
 *   el worker tokio completo (OS thread) durante retain()+send(). Con tokio::sync::Mutex,
 *   la task se suspende sin ocupar el worker. broadcast_to_staff() ahora es async. */

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use dashmap::{mapref::entry::Entry, DashMap};
use sqlx::PgPool;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{ChatMessage, ChatSession, ChatSessionResponse, WsServerMessage};
use crate::repositories::ChatRepository;

pub type SessionSender = mpsc::UnboundedSender<WsServerMessage>;

/// Hub central de chat: estado en memoria de sesiones activas + broadcasters.
/// [096A-13] Cada suscriptor tiene su propio `mpsc::unbounded_channel`.
/// En vez de un `broadcast::Sender` compartido, mantenemos un Vec de senders por sesión.
/// [T-4] `connection_counts`: refcount por sesión para multi-conexión (tabs/dispositivos).
#[derive(Clone)]
pub struct ChatHub {
    pool: PgPool,
    /* session_id → Vec de senders mpsc (uno por suscriptor conectado) */
    channels: Arc<DashMap<Uuid, Vec<SessionSender>>>,
    /* Canal global: staff conectados reciben session_new, visitor_status, etc.
     * [096A-14] tokio::sync::Mutex: no bloquea el worker tokio bajo contención. */
    staff_senders: Arc<TokioMutex<Vec<SessionSender>>>,
    /* [T-4] Contador de conexiones WS activas por sesión (tabs/dispositivos) */
    connection_counts: Arc<DashMap<Uuid, AtomicUsize>>,
    /* [267A-3] Presencia exclusiva de visitantes. Staff también se suscribe a
     * canales de sesión y no debe impedir ni provocar el seguimiento por email. */
    visitor_connection_counts: Arc<DashMap<Uuid, AtomicUsize>>,
}

impl ChatHub {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            channels: Arc::new(DashMap::new()),
            staff_senders: Arc::new(TokioMutex::new(Vec::new())),
            connection_counts: Arc::new(DashMap::new()),
            visitor_connection_counts: Arc::new(DashMap::new()),
        }
    }

    /// Suscribirse al canal de una sesión (para recibir mensajes).
    /// [T-4] Incrementa contador de conexiones activas para refcount multi-tab.
    /// [096A-13] Crea un canal mpsc individual por suscriptor.
    #[must_use]
    pub fn subscribe(&self, session_id: Uuid) -> mpsc::UnboundedReceiver<WsServerMessage> {
        self.connection_counts
            .entry(session_id)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        self.channels.entry(session_id).or_default().push(tx);
        rx
    }

    #[must_use]
    pub fn subscribe_visitor(&self, session_id: Uuid) -> mpsc::UnboundedReceiver<WsServerMessage> {
        self.visitor_connection_counts
            .entry(session_id)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);
        self.subscribe(session_id)
    }

    /// [T-4] Decrementar refcount de conexiones WS. Retorna cuántas quedan.
    /// Solo cerrar sesión si retorna 0 (última conexión).
    #[must_use]
    pub fn unsubscribe(&self, session_id: Uuid) -> usize {
        /* [257A-4] La eliminación debe hacerse mediante el OccupiedEntry que ya
         * posee el lock del shard. Antes se llamaba remove() mientras seguía vivo
         * un Ref de get(); DashMap intentaba adquirir dos veces el mismo shard y
         * bloqueaba permanentemente un worker Tokio por cada desconexión WS. Tras
         * varias reconexiones, todos los workers quedaban bloqueados y el watchdog
         * reiniciaba el sitio con una ventana de Bad Gateway. */
        match self.connection_counts.entry(session_id) {
            Entry::Occupied(entry) => {
                let current = entry.get().load(Ordering::Relaxed);
                if current <= 1 {
                    entry.remove();
                    0
                } else {
                    entry.get().fetch_sub(1, Ordering::Relaxed) - 1
                }
            }
            Entry::Vacant(_) => 0,
        }
    }

    #[must_use]
    pub fn unsubscribe_visitor(&self, session_id: Uuid) -> usize {
        let visitor_remaining = match self.visitor_connection_counts.entry(session_id) {
            Entry::Occupied(entry) => {
                let current = entry.get().load(Ordering::Relaxed);
                if current <= 1 {
                    entry.remove();
                    0
                } else {
                    entry.get().fetch_sub(1, Ordering::Relaxed) - 1
                }
            }
            Entry::Vacant(_) => 0,
        };
        let _ = self.unsubscribe(session_id);
        visitor_remaining
    }

    /// Eliminar canales cuando la sesión se cierra.
    /// [096A-13] Elimina el Vec de senders; los Drop implícitos cierran los canales.
    pub fn remove_channel(&self, session_id: Uuid) {
        self.channels.remove(&session_id);
    }

    /// [064A-68] Suscribirse al canal global de staff (nuevas sesiones, `visitor_status`, etc.)
    /// [096A-13] Crea un canal mpsc individual por staff conectado.
    /// [096A-14] `tokio::sync::Mutex`: lock().await no bloquea el worker.
    pub async fn subscribe_staff(&self) -> mpsc::UnboundedReceiver<WsServerMessage> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.staff_senders.lock().await.push(tx);
        rx
    }

    /// Broadcast de un mensaje a todos los suscriptores de una sesión.
    /// [096A-13] Itera el Vec de senders: `send()` lock-free, `retain()` limpia caídos.
    /// La lock del shard `DashMap` se mantiene solo durante la iteración (microsegundos).
    pub fn broadcast(&self, session_id: Uuid, msg: &WsServerMessage) {
        if let Some(mut senders) = self.channels.get_mut(&session_id) {
            senders.retain(|tx| tx.send(msg.clone()).is_ok());
        }
    }

    /// [096A-13] Broadcast a todos los staff conectados.
    /// [096A-14] `tokio::sync::Mutex`: lock().await suspende la task sin bloquear el worker.
    async fn broadcast_to_staff(&self, msg: &WsServerMessage) {
        let mut senders = self.staff_senders.lock().await;
        senders.retain(|tx| tx.send(msg.clone()).is_ok());
    }

    /* ============================================================
    OPERACIONES DE CHAT
    ============================================================ */

    /// Crear o recuperar sesión para un visitante anónimo
    #[allow(clippy::similar_names)] /* visitor_ip vs visitor_id — nombres semánticos claros */
    pub async fn get_or_create_visitor_session(
        &self,
        visitor_id: &str,
        requested_session_id: Option<Uuid>,
        visitor_name: Option<&str>,
        visitor_ip: Option<&str>,
        visitor_user_agent: Option<&str>,
        visitor_country: Option<&str>,
    ) -> Result<ChatSession, AppError> {
        if let Some(session_id) = requested_session_id {
            if let Some(existing) =
                ChatRepository::find_session_by_id_and_visitor(&self.pool, session_id, visitor_id)
                    .await?
            {
                return Ok(existing);
            }
            return Err(AppError::NotFound(
                "La conversación recuperada no pertenece al visitante".into(),
            ));
        }
        if let Some(existing) =
            ChatRepository::find_session_by_visitor(&self.pool, visitor_id).await?
        {
            /* [064A-72] Actualizar IP/UA si cambió (reconexiones) */
            /* [124A-PAIS] También actualizar country si se obtuvo ahora */
            if visitor_ip.is_some() || visitor_user_agent.is_some() || visitor_country.is_some() {
                let _ = sqlx::query(
                    "UPDATE chat_sessions SET visitor_ip = COALESCE($2, visitor_ip), \
                     visitor_user_agent = COALESCE($3, visitor_user_agent), \
                     visitor_country = COALESCE($4, visitor_country) WHERE id = $1",
                )
                .bind(existing.id)
                .bind(visitor_ip)
                .bind(visitor_user_agent)
                .bind(visitor_country)
                .execute(&self.pool)
                .await;
            }
            return Ok(existing);
        }
        let session =
            ChatRepository::create_session(&self.pool, Some(visitor_id), visitor_name, None, None)
                .await?;

        /* [064A-72] Guardar IP y user-agent en la nueva sesión */
        /* [124A-PAIS] También guardar country */
        if visitor_ip.is_some() || visitor_user_agent.is_some() || visitor_country.is_some() {
            let _ = sqlx::query(
                "UPDATE chat_sessions SET visitor_ip = $2, visitor_user_agent = $3, \
                 visitor_country = $4 WHERE id = $1",
            )
            .bind(session.id)
            .bind(visitor_ip)
            .bind(visitor_user_agent)
            .bind(visitor_country)
            .execute(&self.pool)
            .await;
        }

        /* [064A-68] Notificar a todos los staff conectados sobre la nueva sesión */
        self.broadcast_to_staff(&WsServerMessage::SessionNew {
            session: session.clone(),
        })
        .await;

        Ok(session)
    }

    /// Crear o recuperar sesión vinculada a una orden.
    /// [064A-31] Auto-asigna al empleado de la orden como staff y desactiva IA.
    pub async fn get_or_create_order_session(
        &self,
        order_id: Uuid,
        user_id: Uuid,
    ) -> Result<ChatSession, AppError> {
        /* [237A-5] El repositorio resuelve creación/reapertura de forma atómica.
         * Esto conserva el historial y evita dos sesiones si llegan requests simultáneos. */
        let session =
            ChatRepository::get_or_reopen_order_session(&self.pool, order_id, user_id).await?;

        /* Auto-asignar empleado de la orden como staff del chat */
        let employee_id: Option<Uuid> =
            sqlx::query_scalar("SELECT assigned_employee_id FROM orders WHERE id = $1")
                .bind(order_id)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);

        if let Some(eid) = employee_id {
            let _ = ChatRepository::assign_staff(&self.pool, session.id, eid).await;
            /* Recargar sesión con staff asignado y ai_enabled=false */
            if let Ok(Some(updated)) =
                ChatRepository::find_session_by_id(&self.pool, session.id).await
            {
                /* [064A-68] Notificar a staff conectados sobre nueva sesión de orden */
                self.broadcast_to_staff(&WsServerMessage::SessionNew {
                    session: updated.clone(),
                })
                .await;
                return Ok(updated);
            }
        }

        self.broadcast_to_staff(&WsServerMessage::SessionNew {
            session: session.clone(),
        })
        .await;

        Ok(session)
    }

    /// Enviar mensaje y persistir + broadcast
    pub async fn send_message(
        &self,
        session_id: Uuid,
        sender_type: &str,
        sender_id: Option<&str>,
        content: &str,
    ) -> Result<ChatMessage, AppError> {
        tracing::debug!(%session_id, sender = sender_type, len = content.len(), "send_message: persistiendo en BD");
        let msg =
            ChatRepository::save_message(&self.pool, session_id, sender_type, sender_id, content)
                .await?;

        let ws_msg = WsServerMessage::from_chat_message(&msg, "live");
        self.broadcast(session_id, &ws_msg);
        tracing::debug!(%session_id, sender = sender_type, "send_message: broadcast completado");

        Ok(msg)
    }

    /* [T-2] Enviar mensaje rico (service_card, invoice, etc.) con tipo y metadata.
     * Persiste en BD con message_type + metadata y hace broadcast al WS. */
    pub async fn send_rich_message(
        &self,
        session_id: Uuid,
        sender_type: &str,
        sender_id: Option<&str>,
        content: &str,
        message_type: &str,
        metadata: &serde_json::Value,
    ) -> Result<ChatMessage, AppError> {
        let msg = ChatRepository::save_rich_message(
            &self.pool,
            session_id,
            sender_type,
            sender_id,
            content,
            message_type,
            metadata,
        )
        .await?;

        let ws_msg = WsServerMessage::from_chat_message(&msg, "live");
        self.broadcast(session_id, &ws_msg);

        Ok(msg)
    }

    /// Broadcast de typing sin persistir (alta frecuencia)
    pub fn send_typing(&self, session_id: Uuid, sender: &str, content: &str) {
        self.broadcast(
            session_id,
            &WsServerMessage::Typing {
                session_id,
                sender: sender.to_string(),
                content: content.to_string(),
            },
        );
    }

    /// Staff toma sesión
    pub async fn staff_join_session(
        &self,
        session_id: Uuid,
        staff_id: Uuid,
    ) -> Result<ChatSession, AppError> {
        let session = ChatRepository::assign_staff(&self.pool, session_id, staff_id).await?;
        self.broadcast(
            session_id,
            &WsServerMessage::Status {
                session_id,
                value: "staff_handling".to_string(),
            },
        );
        Ok(session)
    }

    /// Toggle AI en sesión
    pub async fn toggle_ai(
        &self,
        session_id: Uuid,
        enabled: bool,
    ) -> Result<ChatSession, AppError> {
        let session = ChatRepository::toggle_ai(&self.pool, session_id, enabled).await?;
        let status = if enabled {
            "ai_handling"
        } else {
            "staff_handling"
        };
        self.broadcast(
            session_id,
            &WsServerMessage::Status {
                session_id,
                value: status.to_string(),
            },
        );
        Ok(session)
    }

    /// Cerrar sesión. [T-4] También limpia el refcount de conexiones.
    pub async fn close_session(&self, session_id: Uuid) -> Result<(), AppError> {
        ChatRepository::close_session(&self.pool, session_id).await?;
        self.broadcast(session_id, &WsServerMessage::SessionClosed { session_id });
        self.remove_channel(session_id);
        self.connection_counts.remove(&session_id);
        self.visitor_connection_counts.remove(&session_id);
        Ok(())
    }

    /// Listar sesiones e historial como responses con `last_message` preview.
    pub async fn list_sessions_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ChatSessionResponse>, AppError> {
        let sessions = ChatRepository::list_sessions_for_user(&self.pool, user_id).await?;
        self.enrich_sessions(sessions).await
    }

    /// Listar todas las sesiones e historial (staff/admin).
    pub async fn list_all_sessions(&self) -> Result<Vec<ChatSessionResponse>, AppError> {
        let sessions = ChatRepository::list_sessions(&self.pool).await?;
        self.enrich_sessions(sessions).await
    }

    /// Enriquecer sesiones con `last_message` preview, `order_number` y nombres de participantes
    async fn enrich_sessions(
        &self,
        sessions: Vec<ChatSession>,
    ) -> Result<Vec<ChatSessionResponse>, AppError> {
        /* [154A-14] Struct local para traer nombres/avatares de participantes de orden */
        #[derive(sqlx::FromRow)]
        struct OrderParticipants {
            order_id: Uuid,
            client_name: Option<String>,
            client_avatar: Option<String>,
            employee_name: Option<String>,
            employee_avatar: Option<String>,
        }

        let ids: Vec<Uuid> = sessions.iter().map(|s| s.id).collect();
        let last_msgs = ChatRepository::last_messages_for_sessions(&self.pool, &ids).await?;

        /* [064A-31] Obtener order_number para sesiones vinculadas a órdenes */
        let order_ids: Vec<Uuid> = sessions.iter().filter_map(|s| s.order_id).collect();
        let order_numbers: Vec<(Uuid, i32)> = if order_ids.is_empty() {
            vec![]
        } else {
            sqlx::query_as::<_, (Uuid, i32)>(
                "SELECT id, order_number FROM orders WHERE id = ANY($1)",
            )
            .bind(&order_ids)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        };

        /* [154A-14] Obtener nombres y avatares de cliente/empleado para sesiones de orden.
         * Un solo JOIN trae ambos participantes de cada orden vinculada. */
        let participants: Vec<OrderParticipants> = if order_ids.is_empty() {
            vec![]
        } else {
            sqlx::query_as::<_, OrderParticipants>(
                "SELECT o.id AS order_id, \
                   c.display_name AS client_name, c.avatar_url AS client_avatar, \
                   e.display_name AS employee_name, e.avatar_url AS employee_avatar \
                 FROM orders o \
                 JOIN users c ON c.id = o.client_id \
                 LEFT JOIN users e ON e.id = o.assigned_employee_id \
                 WHERE o.id = ANY($1)",
            )
            .bind(&order_ids)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        };

        Ok(sessions
            .into_iter()
            .map(|s| {
                let last = last_msgs.iter().find(|m| m.session_id == s.id);
                let order_num = s.order_id.and_then(|oid| {
                    order_numbers
                        .iter()
                        .find(|(id, _)| *id == oid)
                        .map(|(_, n)| *n)
                });
                let parts = s
                    .order_id
                    .and_then(|oid| participants.iter().find(|p| p.order_id == oid));
                ChatSessionResponse {
                    id: s.id,
                    order_id: s.order_id,
                    order_number: order_num,
                    status: s.status,
                    ai_enabled: s.ai_enabled,
                    assigned_staff_id: s.assigned_staff_id,
                    last_message: last.map(|m| m.content.clone()),
                    last_message_at: last.map(|m| m.created_at),
                    created_at: s.created_at,
                    visitor_name: s.visitor_name,
                    visitor_ip: s.visitor_ip,
                    visitor_user_agent: s.visitor_user_agent,
                    visitor_country: s.visitor_country,
                    last_viewed_at: s.last_viewed_at,
                    visitor_last_connected_at: s.visitor_last_connected_at,
                    is_escalated: s.is_escalated,
                    client_name: parts.and_then(|p| p.client_name.clone()),
                    client_avatar_url: parts.and_then(|p| p.client_avatar.clone()),
                    employee_name: parts.and_then(|p| p.employee_name.clone()),
                    employee_avatar_url: parts.and_then(|p| p.employee_avatar.clone()),
                }
            })
            .collect())
    }

    /// Acceso al pool (para AI service u otros)
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /* [104A-40] Broadcast de estado del visitante al canal de sesión + canal global de staff.
     * online=true cuando el visitor abre la conexión WS; online=false al desconectar.
     * Staff usa esto para mostrar online/offline y confirmar que el visitante lee los mensajes.
     * [096A-13] broadcast() + broadcast_to_staff() usan mpsc lock-free. */
    /* [096A-14] async: broadcast_to_staff usa tokio::sync::Mutex */
    pub async fn notify_visitor_online(
        &self,
        session_id: Uuid,
        last_connected_at: chrono::DateTime<chrono::Utc>,
    ) {
        let msg = WsServerMessage::VisitorStatus {
            session_id,
            online: true,
            last_connected_at: Some(last_connected_at),
        };
        self.broadcast(session_id, &msg);
        self.broadcast_to_staff(&msg).await;
    }

    /* [096A-14] async: broadcast_to_staff usa tokio::sync::Mutex */
    pub async fn notify_visitor_offline(
        &self,
        session_id: Uuid,
        last_connected_at: Option<chrono::DateTime<chrono::Utc>>,
    ) {
        let msg = WsServerMessage::VisitorStatus {
            session_id,
            online: false,
            last_connected_at,
        };
        self.broadcast(session_id, &msg);
        self.broadcast_to_staff(&msg).await;
    }
}

#[cfg(test)]
mod tests {
    use super::ChatHub;
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    fn test_hub() -> ChatHub {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .expect("test database URL must parse");
        ChatHub::new(pool)
    }

    #[tokio::test]
    async fn unsubscribe_removes_last_connection_without_relocking_dashmap() {
        let hub = test_hub();
        let session_id = Uuid::new_v4();

        let first_receiver = hub.subscribe(session_id);
        assert_eq!(hub.unsubscribe(session_id), 0);
        drop(first_receiver);

        let second_receiver = hub.subscribe(session_id);
        let third_receiver = hub.subscribe(session_id);
        assert_eq!(hub.unsubscribe(session_id), 1);
        assert_eq!(hub.unsubscribe(session_id), 0);
        assert_eq!(hub.unsubscribe(session_id), 0);
        drop((second_receiver, third_receiver));
    }

    #[tokio::test]
    async fn visitor_presence_ignores_non_visitor_session_subscribers() {
        let hub = test_hub();
        let session_id = Uuid::new_v4();
        let staff_session_receiver = hub.subscribe(session_id);
        let visitor_one = hub.subscribe_visitor(session_id);
        let visitor_two = hub.subscribe_visitor(session_id);

        assert_eq!(hub.unsubscribe_visitor(session_id), 1);
        assert_eq!(hub.unsubscribe_visitor(session_id), 0);

        drop((staff_session_receiver, visitor_one, visitor_two));
    }
}
