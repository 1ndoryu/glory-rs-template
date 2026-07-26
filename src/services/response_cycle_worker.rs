/* [237A-9] Worker de response cycles: fallback IA cuando el humano no responde.
 * Cada 10 segundos consulta ciclos expirados (deadline pasada + status='waiting').
 * Para cada ciclo expirado:
 *   1. Verifica que la sesión sigue en human_priority
 *   2. Persiste un mensaje de fallback IA directamente vía repositorio
 *   3. Marca el ciclo como answered_ai
 *   4. Restaura ai_mode='automatic' para que la IA siga respondiendo
 *
 * NOTA: Este worker NO broadcastea por WS directamente porque el ChatHub
 * se crea dentro de AppState (después de que este worker se spawnea).
 * El mensaje persistido será visible en el historial al reconectar o al
 * hacer fetch REST. Para sesiones activas, el timing_loop del visitante
 * retomará la IA automáticamente al restaurar ai_mode='automatic'.
 *
 * Ejecuta como tokio::spawn desde main.rs. Se detiene cuando el proceso termina. */

use std::time::Duration;

use sqlx::PgPool;

use crate::repositories::{ChatRepository, ResponseCycleRepository};

const TICK_INTERVAL: Duration = Duration::from_secs(10);

/// Punto de entrada del worker. Se spawnea desde main.rs.
pub async fn run_response_cycle_worker(pool: PgPool) {
    tracing::info!("Response cycle worker iniciado (tick cada {TICK_INTERVAL:?})");

    loop {
        tokio::time::sleep(TICK_INTERVAL).await;

        let expired = match ResponseCycleRepository::claim_expired(&pool).await {
            Ok(cycles) => cycles,
            Err(e) => {
                tracing::error!("Error consultando response cycles expirados: {e}");
                continue;
            }
        };

        if expired.is_empty() {
            continue;
        }

        tracing::info!("{} response cycles expirados, procesando...", expired.len());

        for (cycle_id, session_id) in expired {
            process_expired_cycle(&pool, cycle_id, session_id).await;
        }
    }
}

async fn process_expired_cycle(pool: &PgPool, cycle_id: uuid::Uuid, session_id: uuid::Uuid) {
    /* Verificar que la sesión sigue en human_priority */
    let session = match ChatRepository::find_session_by_id(pool, session_id).await {
        Ok(Some(s)) => s,
        _ => {
            tracing::warn!("Session {session_id} no encontrada, cancelando cycle {cycle_id}");
            return;
        }
    };

    if session.ai_mode != "human_priority" {
        tracing::debug!("Session {session_id} ya no está en human_priority, cancelando cycle");
        let _ = ResponseCycleRepository::cancel_for_session(pool, session_id).await;
        return;
    }

    /* Persistir mensaje de fallback IA directamente vía repositorio.
     * El broadcast WS se omitirá; el mensaje será visible al reconectar
     * o en el próximo fetch REST del historial. */
    tracing::info!("Generando fallback IA para session {session_id} (cycle {cycle_id})");

    let fallback_text = "No hemos recibido respuesta del equipo en este momento. \
         Mientras tanto, ¿puedo ayudarte con algo más?";

    match ResponseCycleRepository::save_fallback_if_still_claimed(
        pool,
        cycle_id,
        session_id,
        fallback_text,
    )
    .await
    {
        Ok(Some(_msg)) => {
            /* [237A-9] Per plan rule 6: tras el fallback, la sesión continúa en
             * human_priority. El siguiente mensaje del cliente abrirá otra espera
             * de 10 min. No restaurar 'automatic' — eso violaría la política de
             * que el humano tiene prioridad permanente hasta otro toggle explícito. */
            tracing::info!(
                "Fallback IA para session {session_id}, modo permanece en human_priority"
            );
        }
        Ok(None) => {
            tracing::info!(
                "Fallback cancelado: hubo respuesta humana o cambió el ciclo {cycle_id}"
            );
        }
        Err(e) => {
            tracing::error!("Error guardando fallback IA para session {session_id}: {e}");
        }
    }
}
