/* [01AA-4-F3r] Extraído de `main.rs`: arranque de todas las tareas de fondo.
 * Sin cambio de comportamiento. */

use sqlx::PgPool;

use crate::services::bandwidth_enforcement::bandwidth_throttle_loop;
use crate::services::cpu_burst::cpu_burst_loop;
use crate::services::infrastructure_metrics::infrastructure_metrics_loop;
use crate::services::storage_enforcement::storage_enforcement_loop;
use crate::services::vps_monitor::vps_monitor_loop;
use crate::services::{AssignmentService, ContaboConfig, ContaboService, CoolifyConfig};

use super::loops::{session_cleanup_loop, unanswered_messages_loop};

/* [250A-1] Extraído de main() para cumplir límite de 100 líneas.
 * Inicia todas las tareas de background: asignación, cleanup chat, storage
 * enforcement, métricas, bandwidth throttle y monitor VPS. */
#[allow(clippy::too_many_lines)]
/* [01AA-4-F1] Loops dependientes de Coolify fuera de spawn_background_services
 * (~40 líneas): metrics/throttle/cpu-burst solo arrancan si hay config. */
fn spawn_coolify_loops(
    pool: &sqlx::PgPool,
    coolify_config: Option<CoolifyConfig>,
    coolify_config_vps1: Option<CoolifyConfig>,
) {
    if coolify_config.is_some() || coolify_config_vps1.is_some() {
        let metrics_pool = pool.clone();
        /* [259A-1] build() falla solo con configuracion invalida (fija aqui):
         * salida explicita en arranque, nunca panic. */
        let metrics_client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                eprintln!("[fatal] metrics HTTP client: {e}");
                std::process::exit(1);
            }
        };
        let metrics_vps1 = coolify_config_vps1.clone();
        let metrics_default = coolify_config.clone();
        tokio::spawn(async move {
            infrastructure_metrics_loop(
                metrics_pool,
                metrics_client,
                metrics_vps1,
                metrics_default,
            )
            .await;
        });

        let throttle_pool = pool.clone();
        let throttle_vps1 = coolify_config_vps1.clone();
        let throttle_default = coolify_config.clone();
        tokio::spawn(async move {
            bandwidth_throttle_loop(throttle_pool, throttle_vps1, throttle_default).await;
        });

        let cpu_burst_pool = pool.clone();
        /* [07AA-7] Mover (no clonar) el último uso: el valor se consume y
         * clippy::needless_pass_by_value queda satisfecho sin &Option. */
        let cpu_burst_vps1 = coolify_config_vps1;
        let cpu_burst_default = coolify_config;
        tokio::spawn(async move {
            cpu_burst_loop(cpu_burst_pool, cpu_burst_vps1, cpu_burst_default).await;
        });
    } else {
        tracing::warn!("[infra-metrics] Coolify no configurado — sampler desactivado");
    }
}

pub fn spawn_background_services(pool: &PgPool) {
    let bg_pool = pool.clone();
    tokio::spawn(async move {
        AssignmentService::auto_assign_loop(bg_pool).await;
    });

    let chat_cleanup_pool = pool.clone();
    tokio::spawn(async move {
        session_cleanup_loop(chat_cleanup_pool).await;
    });

    /* [20CA-8] Background task: detectar mensajes sin responder >20min y notificar */
    let unanswered_pool = pool.clone();
    tokio::spawn(async move {
        unanswered_messages_loop(unanswered_pool).await;
    });

    /* [237A-7d] Background task: worker de alertas de chat (outbox → SMTP + WhatsApp) */
    let alert_pool = pool.clone();
    let alert_email = crate::services::EmailConfig::from_env();
    /* [259A-1] build() falla solo con configuracion invalida (fija aqui):
     * salida explicita en arranque, nunca panic. */
    let alert_client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            eprintln!("[fatal] alert worker HTTP client: {e}");
            std::process::exit(1);
        }
    };
    tokio::spawn(async move {
        crate::services::chat_alert_worker::run_chat_alert_worker(
            alert_pool,
            alert_email,
            alert_client,
        )
        .await;
    });

    /* [237A-9] Background task: worker de response cycles (fallback IA 10min)
     * Solo necesita el pool: persiste mensajes directamente vía ChatRepository.
     * El broadcast WS se omite (ChatHub se crea después en AppState). */
    let cycle_pool = pool.clone();
    tokio::spawn(async move {
        crate::services::response_cycle_worker::run_response_cycle_worker(cycle_pool).await;
    });

    let coolify_config = CoolifyConfig::from_env();
    let coolify_config_vps1 = CoolifyConfig::from_env_with_prefix("COOLIFY_VPS1_");

    if let Some(coolify_config) = coolify_config.clone() {
        let enforcement_pool = pool.clone();
        tokio::spawn(async move {
            storage_enforcement_loop(enforcement_pool, coolify_config).await;
        });
    } else {
        tracing::warn!(
            "[storage-enforcement] Coolify no configurado — enforcement de storage desactivado"
        );
    }

    spawn_coolify_loops(pool, coolify_config, coolify_config_vps1);

    /* [277A-7] Background task: worker de retry de reembolsos fallidos (backoff exponencial) */
    let refund_pool = pool.clone();
    let refund_stripe_key = std::env::var("STRIPE_SECRET_KEY").ok();
    tokio::spawn(async move {
        crate::services::RefundService::run_refund_retry_loop(refund_pool, refund_stripe_key).await;
    });

    if let Some(contabo_config) = ContaboConfig::from_env() {
        let monitor_pool = pool.clone();
        let monitor_service = ContaboService::new(contabo_config, reqwest::Client::new());
        tokio::spawn(async move {
            vps_monitor_loop(monitor_pool, monitor_service).await;
        });
    } else {
        tracing::debug!("[vps-monitor] Contabo no configurado — monitor proveedor desactivado");
    }
}
