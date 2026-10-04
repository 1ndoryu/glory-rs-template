/* [01AA-4-F3r] Extraído de `main.rs`: loops de fondo de chat (cleanup de
 * sesiones y detección de mensajes sin responder). Sin cambio de comportamiento. */

use sqlx::PgPool;

/* [114A-13][237A-5] Background loop: archiva solo sesiones anónimas inactivas.
 * Los chats de pedido o usuario autenticado se conservan activos indefinidamente;
 * las anónimas archivadas siguen disponibles en el historial del panel. */
pub(crate) async fn session_cleanup_loop(pool: PgPool) {
    use crate::repositories::ChatRepository;

    const INACTIVITY_HOURS: i32 = 24;
    const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_hours(1);

    loop {
        tokio::time::sleep(CHECK_INTERVAL).await;
        match ChatRepository::close_inactive_sessions(&pool, INACTIVITY_HOURS).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(
                "[chat-cleanup] {n} sesiones anónimas inactivas archivadas (>{INACTIVITY_HOURS}h)"
            ),
            Err(e) => tracing::error!("[chat-cleanup] Error cerrando sesiones inactivas: {e}"),
        }
    }
}

/* [20CA-8][277A-3] Background loop: detecta mensajes sin responder >20min y
 * envía notificación in-app a admins. Ejecuta cada 5 minutos.
 * Un mensaje "sin responder" es el último de la sesión y fue enviado por
 * el visitante/cliente (no staff/system), y nadie lo ha visto.
 * [277A-3] Deduplicación por timestamp: en vez de HashSet limpiado cada tick,
 * se guarda el instante de la última notificación por sesión. Solo se vuelve
 * a notificar si pasaron al menos 30 minutos desde la última notificación
 * para esa misma sesión. Esto evita el bug anterior donde notified.clear()
 * cada 5 min causaba notificaciones repetidas. */
pub(crate) async fn unanswered_messages_loop(pool: PgPool) {
    use crate::models::CreateNotification;
    use crate::repositories::{ChatRepository, NotificationRepository, UserRepository};
    use std::collections::HashMap;

    const THRESHOLD_MINUTES: i64 = 20;
    const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_mins(5);
    /* Mínimo 30 min entre notificaciones para la misma sesión */
    const RE_NOTIFY_AFTER: std::time::Duration = std::time::Duration::from_mins(30);
    let mut last_notified: HashMap<uuid::Uuid, std::time::Instant> = HashMap::new();

    loop {
        tokio::time::sleep(CHECK_INTERVAL).await;

        /* Limpiar entradas antiguas (>2h) para no crecer indefinidamente */
        last_notified.retain(|_, t| t.elapsed() < std::time::Duration::from_hours(2));

        match ChatRepository::find_unanswered_sessions(&pool, THRESHOLD_MINUTES).await {
            Ok(results) => {
                let now = std::time::Instant::now();
                let new_ids: Vec<_> = results
                    .into_iter()
                    .filter(|sid| {
                        last_notified
                            .get(sid)
                            .is_none_or(|t| now.duration_since(*t) >= RE_NOTIFY_AFTER)
                    })
                    .collect();

                if new_ids.is_empty() {
                    continue;
                }

                tracing::info!(
                    "[unanswered-chat] {} sesiones con mensajes sin responder >{THRESHOLD_MINUTES}min",
                    new_ids.len()
                );

                let admin_ids = match UserRepository::admin_ids(&pool).await {
                    Ok(ids) => ids,
                    Err(e) => {
                        tracing::error!("[unanswered-chat] Error obteniendo admins: {e}");
                        continue;
                    }
                };
                if admin_ids.is_empty() {
                    continue;
                }

                for admin_id in &admin_ids {
                    let notif = CreateNotification {
                        user_id: *admin_id,
                        notification_type: "unanswered_chat".to_string(),
                        title: format!(
                            "{} mensaje(s) sin responder",
                            new_ids.len()
                        ),
                        body: Some(format!(
                            "Hay {} sesiones de chat con mensajes sin respuesta por más de {} minutos.",
                            new_ids.len(), THRESHOLD_MINUTES
                        )),
                        link: Some("/panel?seccion=chat".to_string()),
                        reference_type: Some("chat".to_string()),
                        reference_id: new_ids.first().copied(),
                    };
                    let _ = NotificationRepository::create(&pool, &notif).await;
                }

                /* Registrar timestamp de notificación por sesión */
                let now = std::time::Instant::now();
                for sid in &new_ids {
                    last_notified.insert(*sid, now);
                }
            }
            Err(e) => {
                tracing::error!("[unanswered-chat] Error buscando mensajes sin responder: {e}");
            }
        }
    }
}
