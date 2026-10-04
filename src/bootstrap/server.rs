/* [01AA-4-F3r] Extraído de `main.rs`: listener TCP + loop hyper con graceful
 * shutdown + watchdog de runtime. `run()` orquesta; `serve()` arma graceful +
 * builder; `accept_loop()` contiene solo el loop (los tres por debajo de 100
 * efectivas, regla funcion-larga-rs). Sin cambio de comportamiento. */

use std::net::SocketAddr;
use std::time::Duration;

use glory_rs::runtime::{
    spawn_runtime_watchdog, HttpProbeConfig, RuntimeWatchdogConfig,
};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use hyper_util::server::graceful::GracefulShutdown;
use tower::{Service, ServiceExt};

use super::diagnostics::{dump_kernel_stacks, spawn_runtime_heartbeat_logger};

type MakeService =
    axum::routing::IntoMakeServiceWithConnectInfo<axum::Router, SocketAddr>;

/* Entrypoint del servidor HTTP: bindea el socket, arma el watchdog y delega
 * en serve(). Drena con graceful shutdown (SIGTERM/SIGINT → 30s de plazo). */
pub async fn run(
    app: axum::Router,
    addr: &str,
    server_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = bind_listener(addr)?;

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
    let make_service = app.into_make_service_with_connect_info::<SocketAddr>();

    serve(listener, make_service).await
}

/* Arma graceful + builder hyper (timeouts 096A-2/096A-4) y corre el accept
 * loop; al salir drena conexiones activas con 30s de plazo. */
async fn serve(
    listener: tokio::net::TcpListener,
    make_service: MakeService,
) -> Result<(), Box<dyn std::error::Error>> {
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

    accept_loop(listener, make_service, graceful, builder).await?;

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

/* Accept loop: aplica keepalive por conexión (096A-5), sirve via hyper y
 * spawnea cada conexión con timeout absoluto de 5 min. Sale con shutdown. */
async fn accept_loop(
    listener: tokio::net::TcpListener,
    mut make_service: MakeService,
    graceful: GracefulShutdown,
    builder: &'static mut auto::Builder<TokioExecutor>,
) -> Result<(), Box<dyn std::error::Error>> {
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

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

    Ok(())
}

/* [096A-5] Socket del listener: solo necesita reuse_address y nonblocking.
 * TCP keepalive NO se aplica aquí porque accept() en Linux NO hereda
 * SO_KEEPALIVE del listening socket a los sockets de conexión.
 * El keepalive real se aplica a cada conexión aceptada (ver loop). */
fn bind_listener(addr: &str) -> Result<tokio::net::TcpListener, Box<dyn std::error::Error>> {
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
    Ok(tokio::net::TcpListener::from_std(socket.into())?)
}

/* [096A-1] Señal de graceful shutdown: espera SIGTERM/SIGINT y permite
 * que las conexiones activas se drenen antes de cerrar el proceso. */
async fn shutdown_signal() {
    let ctrl_c = async {
        /* [259A-1] Sin expect: si no se puede instalar el handler, el servidor
         * sigue corriendo sin esa señal (pending eterno) en vez de paniquear. */
        if tokio::signal::ctrl_c().await.is_err() {
            tracing::error!("No se pudo instalar handler Ctrl+C; shutdown graceful sin Ctrl+C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!(
                    "No se pudo instalar handler SIGTERM: {e}; shutdown graceful sin SIGTERM"
                );
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => { tracing::info!("Recibido SIGINT, iniciando graceful shutdown..."); }
        () = terminate => { tracing::info!("Recibido SIGTERM, iniciando graceful shutdown..."); }
    }
}
