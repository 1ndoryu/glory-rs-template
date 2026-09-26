/* [044A-38 Fase 9] NotificationHub: gestión de conexiones WS de notificaciones por usuario.
 * Patrón similar a ChatHub pero indexado por user_id en vez de session_id.
 * Cada usuario autenticado puede conectarse vía WS y recibir notificaciones push.
 * El hub persiste en BD y broadcastea en tiempo real a todos los tabs del usuario.
 * [096A-13] mpsc::unbounded_channel reemplaza broadcast::channel (mismo motivo que chat.rs). */

use std::sync::Arc;

use dashmap::DashMap;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{CreateNotification, Notification, WsNotification};
use crate::repositories::NotificationRepository;

pub type UserNotifSender = mpsc::UnboundedSender<WsNotification>;

/// Hub central de notificaciones: canales mpsc indexados por `user_id`.
/// [096A-13] Cada suscriptor tiene su propio canal mpsc individual.
#[derive(Clone)]
pub struct NotificationHub {
    pool: PgPool,
    /* user_id → Vec de senders mpsc (uno por tab conectado) */
    channels: Arc<DashMap<Uuid, Vec<UserNotifSender>>>,
}

impl NotificationHub {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            channels: Arc::new(DashMap::new()),
        }
    }

    /// Suscribirse al canal de un usuario (para recibir notificaciones en WS).
    /// [096A-13] Crea un canal mpsc individual por suscriptor.
    #[must_use]
    pub fn subscribe(&self, user_id: Uuid) -> mpsc::UnboundedReceiver<WsNotification> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.channels.entry(user_id).or_default().push(tx);
        rx
    }

    /// Crea una notificación en BD y la emite por WS al usuario si está conectado.
    /// [096A-13] `broadcast_to_user()` itera Vec de senders con `send()` lock-free.
    pub async fn notify(&self, params: CreateNotification) -> Result<Notification, AppError> {
        let user_id = params.user_id;

        /* Persistir en BD */
        let notification = NotificationRepository::create(&self.pool, &params).await?;

        /* Broadcast al usuario si tiene canal abierto */
        let ws_msg: WsNotification = notification.clone().into();
        self.broadcast_to_user(user_id, &ws_msg);

        /* También enviar contador actualizado */
        self.send_unread_count(user_id).await;

        Ok(notification)
    }

    /// Envía el conteo actual de no leídas por WS.
    /// [096A-13] El await de DB está fuera de cualquier lock de `DashMap` o `Mutex`.
    pub async fn send_unread_count(&self, user_id: Uuid) {
        if let Ok(count) = NotificationRepository::count_unread(&self.pool, user_id).await {
            self.broadcast_to_user(user_id, &WsNotification::UnreadCount { count });
        }
    }

    /// [096A-13] Itera el Vec de senders del usuario y hace retain de los vivos.
    fn broadcast_to_user(&self, user_id: Uuid, msg: &WsNotification) {
        if let Some(mut senders) = self.channels.get_mut(&user_id) {
            senders.retain(|tx| tx.send(msg.clone()).is_ok());
        }
    }

    /// Emite notificación a múltiples usuarios (ej: admins).
    /// Recibe un `CreateNotification` base y lo clona para cada usuario.
    pub async fn notify_many(
        &self,
        user_ids: &[Uuid],
        base: &CreateNotification,
    ) -> Result<Vec<Notification>, AppError> {
        let mut results = Vec::with_capacity(user_ids.len());

        for &uid in user_ids {
            let params = CreateNotification {
                user_id: uid,
                notification_type: base.notification_type.clone(),
                title: base.title.clone(),
                body: base.body.clone(),
                link: base.link.clone(),
                reference_type: base.reference_type.clone(),
                reference_id: base.reference_id,
            };
            match self.notify(params).await {
                Ok(n) => results.push(n),
                Err(e) => {
                    tracing::warn!("Error notificando a {uid}: {e}");
                }
            }
        }

        Ok(results)
    }

    /// Limpia canales de usuarios sin senders activos (housekeeping).
    /// [096A-13] Remove entradas cuyo Vec está vacío (todos los senders droppeados).
    pub fn cleanup_empty_channels(&self) {
        self.channels
            .retain(|_user_id, senders| !senders.is_empty());
    }
}
