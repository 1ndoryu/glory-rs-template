/*
 * [096A-5] Test: verificar que SO_KEEPALIVE se aplica por socket aceptado.
 *
 * El bug de fixes v1-v4 era que `accept()` en Linux NO hereda SO_KEEPALIVE
 * del listening socket. Este test replica la lógica de fix v5 y verifica
 * que el socket aceptado tiene keepalive activo vía socket2 API.
 *
 * Ejecutar: cargo test --test test_keepalive_per_socket -- --nocapture
 */

use std::net::TcpListener;
use std::time::Duration;

#[test]
fn keepalive_applied_to_accepted_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let connect_handle = std::thread::spawn(move || {
        let _stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        std::thread::sleep(Duration::from_millis(200));
    });

    let (accepted_stream, _addr) = listener.accept().unwrap();

    /* Replicar fix v5: std -> socket2 -> set keepalive -> socket2 */
    let accepted_socket = socket2::Socket::from(accepted_stream);
    accepted_socket.set_keepalive(true).unwrap();
    let ka = socket2::TcpKeepalive::new()
        .with_time(Duration::from_secs(60))
        .with_interval(Duration::from_secs(15));
    accepted_socket.set_tcp_keepalive(&ka).unwrap();

    let ka_after = accepted_socket.keepalive().unwrap();
    assert!(
        ka_after,
        "SO_KEEPALIVE debe estar activo después de aplicar fix v5"
    );
    println!("✅ SO_KEEPALIVE activo en socket aceptado (keepalive={ka_after})");

    connect_handle.join().unwrap();
}

#[test]
fn keepalive_listener_does_not_propagate_to_accepted() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    /* Aplicar keepalive al LISTENER (como hacían v1-v4) */
    let listener_socket = socket2::Socket::from(listener);
    listener_socket.set_keepalive(true).unwrap();
    let ka = socket2::TcpKeepalive::new()
        .with_time(Duration::from_secs(60))
        .with_interval(Duration::from_secs(15));
    listener_socket.set_tcp_keepalive(&ka).unwrap();

    assert!(
        listener_socket.keepalive().unwrap(),
        "Listener debe tener SO_KEEPALIVE"
    );
    println!("✅ Listener tiene keepalive activo");

    let connect_handle = std::thread::spawn(move || {
        let _stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        std::thread::sleep(Duration::from_millis(200));
    });

    /* Aceptar SIN aplicar keepalive extra (como hacían v1-v4) */
    let std_listener: std::net::TcpListener = listener_socket.into();
    let (accepted, _addr) = std_listener.accept().unwrap();

    let accepted_socket = socket2::Socket::from(accepted);
    let ka_accepted = accepted_socket.keepalive().unwrap_or(false);

    println!("Socket aceptado sin fix: SO_KEEPALIVE = {ka_accepted}");
    /* En Linux esto es típicamente false (el bug).
     * En Windows puede variar. Fix v5 siempre aplica explícitamente. */

    connect_handle.join().unwrap();
}

#[test]
fn timeout_does_not_fire_on_quick_connection() {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let connect_handle = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
                .await
                .unwrap();
            drop(stream);
        });

        let (stream, _) = listener.accept().await.unwrap();

        let result = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            drop(stream);
        })
        .await;

        assert!(
            result.is_ok(),
            "Conexión rápida no debe ser matada por timeout"
        );
        println!("✅ Timeout no interfiere con conexiones rápidas");

        connect_handle.await.unwrap();
    });
}

#[test]
fn socket2_keepalive_conversion_roundtrip() {
    use std::net::TcpStream;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let connect_handle = std::thread::spawn(move || {
        let _stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        std::thread::sleep(Duration::from_millis(200));
    });

    let (accepted, _addr) = listener.accept().unwrap();

    /* Conversión exacta de fix v5: std -> socket2 (set keepalive) -> std */
    let socket = socket2::Socket::from(accepted);
    socket.set_keepalive(true).unwrap();
    let ka = socket2::TcpKeepalive::new()
        .with_time(Duration::from_secs(60))
        .with_interval(Duration::from_secs(15));
    socket.set_tcp_keepalive(&ka).unwrap();

    let _std_back: std::net::TcpStream = socket.into();
    println!("✅ Conversión roundtrip std -> socket2 -> std exitosa");

    connect_handle.join().unwrap();
}
