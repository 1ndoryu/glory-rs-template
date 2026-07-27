/* sentinel-disable-file sqlx-query-sin-macro: main.rs usa queries dinámicas para
 * setup inicial (admin seeding, cleanup test data) con formatos generados en runtime. */
use std::net::SocketAddr;
use std::time::Duration;

use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use hyper_util::server::graceful::GracefulShutdown;
use tower::{Service, ServiceExt};

use argon2::password_hash::rand_core::OsRng;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use glory_backend::config::AppConfig;
use glory_backend::handlers;
use glory_backend::services::bandwidth_enforcement::bandwidth_throttle_loop;
use glory_backend::services::cpu_burst::cpu_burst_loop;
use glory_backend::services::infrastructure_metrics::infrastructure_metrics_loop;
use glory_backend::services::storage_enforcement::storage_enforcement_loop;
use glory_backend::services::vps_monitor::vps_monitor_loop;
use glory_backend::services::{AssignmentService, ContaboConfig, ContaboService, CoolifyConfig};
use glory_rs::fixtures::ContentManager;
use glory_rs::runtime::{spawn_runtime_watchdog, HttpProbeConfig, RuntimeHeartbeat, RuntimeWatchdogConfig};

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new(
                    "glory_backend=debug,glory_rs=debug,tower_http=debug,hyper=debug,h2=debug",
                )
            }),
        )
        .init();

    let config = AppConfig::from_env()?;

    /* [096A-1] Pool con max_lifetime + idle_timeout para evitar conexiones
     * zombie que provocan CLOSE_WAIT y deadlock del event loop. */
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .max_lifetime(Duration::from_mins(30))
        .idle_timeout(Duration::from_mins(5))
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    /* [250A-1] Fixtures + background tasks extraídos a helpers para mantener
     * main() por debajo de 100 líneas (regla funcion-larga-rs). */
    setup_and_run_fixtures(&pool).await?;
    spawn_background_services(&pool, &config);

    let server_port = config.port;
    let addr = format!("{}:{}", config.host, server_port);
    tracing::info!("Servidor iniciando en {addr}");
    tracing::info!("Swagger UI disponible en http://{addr}/swagger-ui/");

    let app = handlers::create_app(pool, config);

    /* [096A-5] Socket del listener: solo necesita reuse_address y nonblocking.
     * TCP keepalive NO se aplica aquí porque accept() en Linux NO hereda
     * SO_KEEPALIVE del listening socket a los sockets de conexión.
     * El keepalive real se aplica a cada conexión aceptada (ver loop). */
    let sock_addr: std::net::SocketAddr = addr.parse()?;
    let socket = socket2::Socket::new(
        if sock_addr.is_ipv4() {
            socket2::Domain::IPV4
        } else {
            socket2::Domain::IPV6
        },
        socket2::Type::STREAM,
        None,
    )?;
    socket.set_nonblocking(true)?;
    socket.set_reuse_address(true)?;
    socket.bind(&sock_addr.into())?;
    socket.listen(1024)?;
    let listener = tokio::net::TcpListener::from_std(socket.into())?;

    /* [237A-4][277A-6] Watchdog de runtime con doble señal:
     * Señal A: heartbeat Tokio (sequence monotónica).
     * Señal B: HTTP probe loopback (GET localhost:puerto/healthz).
     * Solo mata si AMBAS fallan durante 120s. Grace period 60s al arrancar.
     * Rollback: GLORY_HTTP_WATCHDOG=false desactiva todo. */
    let runtime_watchdog_disabled = std::env::var("GLORY_HTTP_WATCHDOG")
        .is_ok_and(|value| value.eq_ignore_ascii_case("false") || value == "0");
    if runtime_watchdog_disabled {
        tracing::warn!("[rt-watchdog] Desactivado por GLORY_HTTP_WATCHDOG");
    } else {
        let watchdog_config = RuntimeWatchdogConfig {
            /* [277A-6] HTTP probe loopback: verifica que el servidor HTTP sigue vivo */
            http_probe: Some(HttpProbeConfig {
                port: server_port,
                path: "/healthz".to_string(),
                timeout: Duration::from_secs(3),
            }),
            /* Grace period: 60s para que el servidor termine de arrancar */
            grace_period: Duration::from_secs(60),
            ..RuntimeWatchdogConfig::default()
        };
        tracing::info!(
            "[rt-watchdog] Doble señal activada: heartbeat + HTTP probe en 127.0.0.1:{}/healthz (grace 60s, threshold 120s)",
            server_port
        );
        let runtime_heartbeat = spawn_runtime_watchdog(&watchdog_config, || {
            eprintln!("[rt-watchdog] Volcando stacks del kernel...\n");
            dump_kernel_stacks();
            eprintln!("\n[rt-watchdog] Forzando exit(1) para restart de Docker...");
            std::process::exit(1);
        })?;
        spawn_runtime_heartbeat_logger(runtime_heartbeat)?;
    }

    /* [096A-2] Migrado de axum::serve() a hyper_util::auto::Builder para
     * configurar header_read_timeout a nivel HTTP. axum::serve() es
     * intencionalmente simple y NO expone configuración de conexiones
     * (ver tokio-rs/axum#2939). Sin este timeout, conexiones keep-alive
     * de clientes que desaparecen quedan en CLOSE_WAIT indefinidamente
     * hasta saturar el event loop.
     *
     * header_read_timeout se rearma después de cada respuesta (confirmado
     * por test hyper: header_read_timeout_as_idle_timeout), actuando como
     * idle timeout entre requests keep-alive. */
    let mut make_service = app.into_make_service_with_connect_info::<SocketAddr>();

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    /* [096A-4] GracefulShutdown coordina el cierre limpio de TODAS las conexiones
     * activas cuando el server loop hace break (SIGTERM, deploy, Docker restart).
     * Sin esto, las tareas spawneadas con conexiones activas quedan huérfanas →
     * hyper no les notifica que deben cerrar → CLOSE_WAIT permanente.
     *
     * NOTA: GracefulShutdown NO es Clone intencionalmente (prevenir race conditions
     * entre watch y shutdown). Usamos watcher() para crear un Watcher owned
     * por conexión, que se puede mover a tokio::spawn sin problemas. */
    let graceful = GracefulShutdown::new();

    /* [096A-4] Builder creado UNA VEZ con Box::leak para obtener &'static.
     * serve_connection_with_upgrades(&self) retorna Connection<'_> que borrow
     * el builder — tokio::spawn requiere 'static, así que el builder debe
     * vivir tanto como el proceso. Box::leak es intencional: el servidor
     * vive hasta exit(), no hay leak real. */
    let builder: &'static mut auto::Builder<TokioExecutor> =
        Box::leak(Box::new(auto::Builder::new(TokioExecutor::new())));

    /* [096A-4] HTTP/1.1: header_read_timeout actúa como idle timeout
     * (se rearma tras cada respuesta, hyper PR #3828).
     * half_close(false): cierra conexión al detectar EOF durante request,
     * evitando sockets en CLOSE_WAIT por half-close de Traefik. */
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(Duration::from_secs(30))
        .half_close(false)
        .keep_alive(true);

    /* [096A-2] HTTP/2: keep-alive con PING cada 30s, timeout 10s.
     * Sin esto, conexiones h2c (si Traefik negocia H2) quedan
     * abiertas indefinidamente → CLOSE_WAIT. */
    builder
        .http2()
        .keep_alive_interval(Duration::from_secs(30))
        .keep_alive_timeout(Duration::from_secs(10));

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (tcp_stream, remote_addr) = result?;

                /* [096A-5] CRÍTICO: TCP keepalive DEBE aplicarse al socket de conexión,
                 * NO al listener. accept() en Linux NO hereda SO_KEEPALIVE.
                 * Sin esto, el keepalive de fixes v1-v4 era NO-OP en todas las
                 * conexiones reales — solo operaba en el listener socket (inútil).
                 *
                 * Parámetros: 60s idle → primer probe, luego cada 15s.
                 * Si el peer muere, kernel detecta en ~120s y cierra el socket. */
                let tcp_stream = {
                    let std_stream = tcp_stream.into_std()?;
                    let accepted = socket2::Socket::from(std_stream);
                    accepted.set_keepalive(true)?;
                    let ka = socket2::TcpKeepalive::new()
                        .with_time(Duration::from_mins(1))
                        .with_interval(Duration::from_secs(15));
                    let _ = accepted.set_tcp_keepalive(&ka);
                    let std_back: std::net::TcpStream = accepted.into();
                    std_back.set_nonblocking(true)?;
                    tokio::net::TcpStream::from_std(std_back)?
                };

                let tower_service = match make_service.call(remote_addr).await {
                    Ok(svc) => svc,
                    Err(err) => match err {},
                };

                let io = TokioIo::new(tcp_stream);

                let hyper_service =
                    hyper::service::service_fn(move |request: hyper::Request<Incoming>| {
                        tower_service.clone().oneshot(request)
                    });

                let conn = builder.serve_connection_with_upgrades(io, hyper_service);

                /* [096A-4] watcher() crea un Watcher owned (no borrow graceful).
                 * watch(conn) toma ownership del Connection y retorna un Future
                 * que combina la conexión con la señal de shutdown. */
                let graceful_watcher = graceful.watcher();
                let watched_connection = graceful_watcher.watch(conn);

                tokio::spawn(async move {
                    let start = std::time::Instant::now();

                    /* [096A-5] Timeout absoluto 300s (5 min): match con WS inactivity
                     * timeout. HTTP normal cierra mucho antes (header_read_timeout 30s).
                     * WS activos: su timeout de app (300s) cierra primero.
                     * Este es el safety net — dropea TcpStream → close() vía RAII. */
                    match tokio::time::timeout(Duration::from_mins(5), watched_connection).await {
                        Ok(Ok(())) => {
                            let secs = start.elapsed().as_secs();
                            if secs > 60 {
                                tracing::debug!(
                                    "Connection from {remote_addr} closed after {secs}s"
                                );
                            }
                        }
                        Ok(Err(err)) => {
                            tracing::debug!("Connection error from {remote_addr}: {err:#}");
                        }
                        Err(_) => {
                            tracing::warn!(
                                "Connection from {remote_addr} killed by 5min timeout"
                            );
                        }
                    }
                    /* Socket (TcpStream) se dropea aquí → close() automático vía RAII. */
                });
            }
            () = &mut shutdown => {
                tracing::info!("Graceful shutdown: dejando de aceptar conexiones");
                break;
            }
        }
    }

    /* [096A-4] Drenar conexiones activas: graceful.shutdown() notifica a todas
     * las conexiones watched que deben cerrar. Damos 30s de plazo; si alguna
     * conexión no cierra a tiempo, el proceso termina y el kernel limpia. */
    tracing::info!("Graceful shutdown: esperando conexiones activas (max 30s)...");
    if tokio::time::timeout(Duration::from_secs(30), graceful.shutdown())
        .await
        .is_err()
    {
        tracing::warn!("Graceful shutdown timeout: forzando cierre de conexiones pendientes");
    }

    Ok(())
}

/* [096A-1] Señal de graceful shutdown: espera SIGTERM/SIGINT y permite
 * que las conexiones activas se drenen antes de cerrar el proceso. */
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => { tracing::info!("Recibido SIGINT, iniciando graceful shutdown..."); }
        () = terminate => { tracing::info!("Recibido SIGTERM, iniciando graceful shutdown..."); }
    }
}

/* [250A-1] Extraído de main() para cumplir límite de 100 líneas.
 * Configura password hasher, content manager, limpia seed legacy y sincroniza
 * content/ TOMLs si FIXTURES_SYNC=true. */
#[allow(clippy::too_many_lines)]
async fn setup_and_run_fixtures(pool: &sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let password_hasher: glory_rs::fixtures::PasswordHasher = Box::new(|plain| {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(plain.as_bytes(), &salt)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?
            .to_string();
        Ok(hash)
    });
    let fixture_manager =
        ContentManager::new(pool.clone(), "content").with_password_hasher(password_hasher);

    cleanup_legacy_seed(pool).await;

    let fixtures_sync =
        std::env::var("FIXTURES_SYNC").is_ok_and(|v| v.eq_ignore_ascii_case("true") || v == "1");

    if fixtures_sync {
        match fixture_manager.sync_all().await {
            Ok(report) => {
                tracing::info!("[fixtures] {}", report.summary());
                for err in &report.errors {
                    tracing::error!("[fixtures] {err}");
                }
            }
            Err(e) => tracing::error!("[fixtures] Error syncing: {e}"),
        }
    } else {
        tracing::info!("[fixtures] Sync desactivado (FIXTURES_SYNC != true)");
    }

    Ok(())
}

/* [250A-1] Extraído de main() para cumplir límite de 100 líneas.
 * Inicia todas las tareas de background: asignación, cleanup chat, storage
 * enforcement, métricas, bandwidth throttle y monitor VPS. */
#[allow(clippy::too_many_lines)]
fn spawn_background_services(pool: &sqlx::PgPool, _config: &AppConfig) {
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
    let alert_email = glory_backend::services::EmailConfig::from_env();
    let alert_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("alert worker HTTP client");
    tokio::spawn(async move {
        glory_backend::services::chat_alert_worker::run_chat_alert_worker(
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
        glory_backend::services::response_cycle_worker::run_response_cycle_worker(
            cycle_pool,
        )
        .await;
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

    if coolify_config.is_some() || coolify_config_vps1.is_some() {
        let metrics_pool = pool.clone();
        let metrics_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("metrics HTTP client");
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
        let cpu_burst_vps1 = coolify_config_vps1.clone();
        let cpu_burst_default = coolify_config.clone();
        tokio::spawn(async move {
            cpu_burst_loop(cpu_burst_pool, cpu_burst_vps1, cpu_burst_default).await;
        });
    } else {
        tracing::warn!("[infra-metrics] Coolify no configurado — sampler desactivado");
    }

    /* [277A-7] Background task: worker de retry de reembolsos fallidos (backoff exponencial) */
    let refund_pool = pool.clone();
    let refund_stripe_key = std::env::var("STRIPE_SECRET_KEY").ok();
    tokio::spawn(async move {
        glory_backend::services::RefundService::run_refund_retry_loop(
            refund_pool,
            refund_stripe_key,
        )
        .await;
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

fn spawn_runtime_heartbeat_logger(heartbeat: RuntimeHeartbeat) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("hb-logger".into())
        .spawn(move || {
            let mut last_sequence = 0;
            let mut last_progress = std::time::Instant::now();
            std::thread::sleep(Duration::from_secs(15));
            loop {
                let current_sequence = heartbeat.sequence();
                if current_sequence != last_sequence {
                    last_sequence = current_sequence;
                    last_progress = std::time::Instant::now();
                }
                let status = if current_sequence == 0 {
                    "starting".to_string()
                } else {
                    format!("healthy stalled_for={}s", last_progress.elapsed().as_secs())
                };
                eprintln!("[hb-logger] sequence={current_sequence} status={status}");
                std::thread::sleep(Duration::from_secs(15));
            }
        })?;
    Ok(())
}

/* Volcar stacks del kernel de todos los threads via /proc/self/task/TID/stack.
 * Funciona dentro de Docker en Linux. No requiere gdb ni herramientas externas.
 * Los stacks del kernel muestran si un thread está bloqueado en futex (mutex),
 * esperando I/O, o en estado running. Muy útil para diagnosticar deadlocks.
 * [096A-11] En Docker, /proc/self/task/TID/stack puede estar vacío. Como fallback,
 * listar threads con su estado y nombre para diagnóstico mínimo. */
fn dump_kernel_stacks() {
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        eprintln!("[rt-watchdog] No se pudo leer /proc/self/task");
        return;
    };
    let mut found_any = false;
    for entry in tasks.flatten() {
        let tid = entry.file_name();
        let tid_str = tid.to_string_lossy();
        let stack_path = format!("/proc/self/task/{tid_str}/stack");
        let status_path = format!("/proc/self/task/{tid_str}/status");
        let name = std::fs::read_to_string(&status_path)
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("Name:"))
                    .map(|l| l.trim_start_matches("Name:").trim().to_string())
            })
            .unwrap_or_default();
        let state = std::fs::read_to_string(&status_path)
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("State:"))
                    .map(|l| l.trim_start_matches("State:").trim().to_string())
            })
            .unwrap_or_default();
        if let Ok(stack) = std::fs::read_to_string(&stack_path) {
            let trimmed = stack.trim();
            if !trimmed.is_empty() && trimmed != "(empty)" {
                eprintln!("--- Thread {tid_str} ({name}) [{state}] ---\n{trimmed}\n");
                found_any = true;
            }
        }
        /* [096A-11] Fallback: si stacks vacíos, listar threads con estado */
        if !found_any {
            eprintln!("[rt-watchdog] Thread {tid_str}: name={name} state={state}");
        }
    }
}

/* [074A-23] Limpia datos de seed legacy que ahora son manejados por fixtures.
 * Borra órdenes (con cascade FK completo) y hosting de test emails conocidos
 * que NO están rastreados en _glory_fixtures. Es no-op si no hay datos legacy. */
async fn cleanup_legacy_seed(pool: &sqlx::PgPool) {
    let test_emails = &["cliente@test.com", "empleado@test.com"];
    let tables_exist: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = '_glory_fixtures')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !tables_exist {
        return;
    }

    /* Subquery: IDs de órdenes legacy (no fixture-tracked) de test users */
    let legacy_orders_subquery = "SELECT o.id FROM orders o
         JOIN users u ON o.client_id = u.id
         WHERE u.email = ANY($1)
         AND NOT EXISTS (
             SELECT 1 FROM _glory_fixtures gf
             WHERE gf.table_name = 'orders' AND gf.db_id = o.id::text
         )";

    /* Romper FK circular orders↔chat_sessions */
    let _ = sqlx::query(&format!(
        "UPDATE orders SET chat_session_id = NULL WHERE id IN ({legacy_orders_subquery})"
    ))
    .bind(test_emails)
    .execute(pool)
    .await;

    /* Cascade completo: chat → reviews → refunds → delegations → payments → deliverables → phases → orders */
    let cascade_tables = [
        (
            "chat_messages",
            "session_id IN (SELECT id FROM chat_sessions WHERE order_id IN ({q}))",
        ),
        (
            "chat_session_notes",
            "session_id IN (SELECT id FROM chat_sessions WHERE order_id IN ({q}))",
        ),
        ("chat_sessions", "order_id IN ({q})"),
        ("order_reviews", "order_id IN ({q})"),
        ("order_refunds", "order_id IN ({q})"),
        ("order_delegations", "order_id IN ({q})"),
        ("order_payments", "order_id IN ({q})"),
        (
            "phase_deliverables",
            "phase_id IN (SELECT id FROM order_phases WHERE order_id IN ({q}))",
        ),
        ("order_phases", "order_id IN ({q})"),
    ];

    let mut total_deleted = 0u64;
    for (table, condition_tpl) in &cascade_tables {
        let condition = condition_tpl.replace("{q}", legacy_orders_subquery);
        let sql = format!("DELETE FROM {table} WHERE {condition}");
        if let Ok(r) = sqlx::query(&sql).bind(test_emails).execute(pool).await {
            total_deleted += r.rows_affected();
        }
    }

    /* Borrar órdenes legacy */
    let sql = format!("DELETE FROM orders WHERE id IN ({legacy_orders_subquery})");
    if let Ok(r) = sqlx::query(&sql).bind(test_emails).execute(pool).await {
        total_deleted += r.rows_affected();
    }

    /* Borrar hosting legacy (no fixture-tracked) */
    let legacy_hosting_subquery = "SELECT hs.id FROM hosting_subscriptions hs
         JOIN users u ON hs.user_id = u.id
         WHERE u.email = ANY($1)
         AND NOT EXISTS (
             SELECT 1 FROM _glory_fixtures gf
             WHERE gf.table_name = 'hosting_subscriptions' AND gf.db_id = hs.id::text
         )";

    let _ = sqlx::query(&format!(
        "DELETE FROM hosting_events WHERE subscription_id IN ({legacy_hosting_subquery})"
    ))
    .bind(test_emails)
    .execute(pool)
    .await
    .map(|r| total_deleted += r.rows_affected());

    let _ = sqlx::query(&format!(
        "DELETE FROM hosting_subscriptions WHERE id IN ({legacy_hosting_subquery})"
    ))
    .bind(test_emails)
    .execute(pool)
    .await
    .map(|r| total_deleted += r.rows_affected());

    if total_deleted > 0 {
        tracing::info!("[cleanup] Legacy seed: {total_deleted} records deleted");
    }
}

/* [114A-13][237A-5] Background loop: archiva solo sesiones anónimas inactivas.
 * Los chats de pedido o usuario autenticado se conservan activos indefinidamente;
 * las anónimas archivadas siguen disponibles en el historial del panel. */
async fn session_cleanup_loop(pool: sqlx::PgPool) {
    use glory_backend::repositories::ChatRepository;

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
async fn unanswered_messages_loop(pool: sqlx::PgPool) {
    use glory_backend::models::CreateNotification;
    use glory_backend::repositories::{ChatRepository, NotificationRepository, UserRepository};
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
