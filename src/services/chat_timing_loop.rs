/* [259A-4d] Maquina de estados de timing por sesion: recibe eventos, bufferea
 * mensajes y decide cuando responder. `session_timing_loop` se ejecuta como
 * tokio::spawn por cada sesion activa (ver `register_session` en el hub). */

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

use super::chat_timing::{
    TimingEvent, TimingSessionDeps, LISTEN_TIMEOUT, MAX_ACCUMULATION, TYPING_COOLDOWN, WAIT_TIMEOUT,
};
use super::chat_timing_escalation::generate_context_summary;
use super::chat_timing_response::generate_ai_response;

/* Máquina de estados del timing por sesión.
 * Recibe eventos, bufferea mensajes, decide cuándo responder.
 * Se ejecuta como tokio::spawn por cada sesión activa. */
pub(crate) async fn session_timing_loop(
    session_id: Uuid,
    visitor_name: Option<String>,
    mut rx: mpsc::Receiver<TimingEvent>,
    deps: TimingSessionDeps,
    ai_semaphore: Arc<Semaphore>,
) {
    let mut buffer: Vec<String> = Vec::new();
    let mut is_typing = false;
    let mut irrelevant_count: u32 = 0;

    tracing::info!(%session_id, "session_timing_loop: iniciado");
    loop {
        /* Estado IDLE: esperar primer mensaje */
        let Some(event) = rx.recv().await else {
            tracing::info!(%session_id, "session_timing_loop: channel cerrado, saliendo");
            break; /* channel cerrado, sesión terminó */
        };

        tracing::info!(%session_id, ?event, "session_timing_loop: evento recibido");

        match event {
            TimingEvent::Disconnect => break,
            TimingEvent::TypingStart => {
                is_typing = true;
                continue;
            }
            TimingEvent::TypingStop => {
                is_typing = false;
                continue;
            }
            TimingEvent::Message(content) => {
                buffer.push(content);
            }
        }

        /* Acumular mensajes hasta timeout */
        let first_msg = Instant::now();
        accumulate_messages(&mut rx, &mut buffer, &mut is_typing, first_msg).await;

        if buffer.is_empty() {
            continue;
        }

        /* Estado RESPONDING */
        let combined = buffer.join("\n");
        buffer.clear();

        tracing::info!(%session_id, chars = combined.len(), "Iniciando respuesta IA");
        /* [096A-7] Adquirir permit del semáforo antes de llamar a IA.
         * Limita peticiones IA concurrentes para proteger pool DB y APIs. */
        /* [096A-8] Timeout en semaphore: si 3 permits ocupados >30s, abortar en vez de bloquear.
         * Sin timeout, una 4ta sesión espera indefinidamente → canal mpsc se llena → WS se congela. */
        let permit =
            match tokio::time::timeout(Duration::from_secs(30), ai_semaphore.acquire()).await {
                /* [259A-1] Semaphore cerrado (permits liberados): responder sin limite
                 * en vez de paniquear; el cierre solo ocurre en shutdown. */
                Ok(Ok(permit)) => Some(permit),
                Ok(Err(e)) => {
                    tracing::warn!(
                    "Semaforo IA cerrado para sesion {session_id}: {e}; respondiendo sin limite"
                );
                    None
                }
                Err(_) => {
                    tracing::warn!("Timeout 30s esperando semáforo IA para sesión {session_id}");
                    continue;
                }
            };
        irrelevant_count = generate_ai_response(
            session_id,
            visitor_name.as_deref(),
            &combined,
            irrelevant_count,
            &deps,
        )
        .await;
        drop(permit);
    }

    /* [T-3] Al cerrar sesión, generar resumen de contexto para futuras conversaciones.
     * Usa modelo pequeño para resumir el historial y lo guarda en visitor_profiles.
     * [096A-7] Wrap en timeout 60s: fire-and-forget no debe vivir indefinidamente. */
    let summary_pool = deps.pool.clone();
    let summary_config = deps.ai_config.clone();
    let summary_http = deps.http_client.clone();
    tokio::spawn(async move {
        let _ = tokio::time::timeout(
            Duration::from_secs(60),
            generate_context_summary(
                summary_pool,
                summary_config,
                summary_http,
                session_id,
                deps.visitor_id,
            ),
        )
        .await;
    });
}

/* Acumula mensajes del buffer hasta que expira el timeout.
 * Gestiona transiciones WAITING ↔ LISTENING según typing indicators. */
pub(crate) async fn accumulate_messages(
    rx: &mut mpsc::Receiver<TimingEvent>,
    buffer: &mut Vec<String>,
    is_typing: &mut bool,
    first_msg: Instant,
) {
    loop {
        let deadline = if *is_typing {
            LISTEN_TIMEOUT
        } else {
            WAIT_TIMEOUT
        };

        let max_remaining = MAX_ACCUMULATION
            .checked_sub(first_msg.elapsed())
            .unwrap_or(Duration::ZERO);

        let effective_timeout = deadline.min(max_remaining);

        tokio::select! {
            () = tokio::time::sleep(effective_timeout) => {
                /* Timeout expirado. Si typing activo, esperar cooldown extra */
                if *is_typing {
                    tokio::time::sleep(TYPING_COOLDOWN).await;
                    drain_pending(rx, buffer, is_typing);
                }
                return; /* listo para responder */
            }
            Some(evt) = rx.recv() => {
                match evt {
                    TimingEvent::Message(c) => { buffer.push(c); }
                    TimingEvent::TypingStart => { *is_typing = true; }
                    TimingEvent::TypingStop => { *is_typing = false; }
                    TimingEvent::Disconnect => { buffer.clear(); return; }
                }
                /* Continuar acumulando */
            }
        }
    }
}

/* Drena eventos pendientes sin bloquear (para el cooldown post-typing) */
pub(crate) fn drain_pending(
    rx: &mut mpsc::Receiver<TimingEvent>,
    buffer: &mut Vec<String>,
    is_typing: &mut bool,
) {
    while let Ok(evt) = rx.try_recv() {
        match evt {
            TimingEvent::Message(c) => buffer.push(c),
            TimingEvent::TypingStart => *is_typing = true,
            TimingEvent::TypingStop => *is_typing = false,
            TimingEvent::Disconnect => {
                buffer.clear();
                return;
            }
        }
    }
}
