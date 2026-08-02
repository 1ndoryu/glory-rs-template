use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::serve;
use futures_util::{SinkExt, StreamExt};
use glory_backend::handlers::create_router_with_state;
use glory_backend::services::game_ticket::GameTicketStore;
use glory_backend::services::game_ws::GameWsState;
use glory_backend::AppState;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const TEST_SECRET: &str = "game-ws-tcp-test-secret";

fn test_state() -> AppState {
    test_state_with_capacity(8)
}

fn test_state_with_capacity(max_connections: usize) -> AppState {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://test:test@127.0.0.1:5432/game_ws_tcp")
        .expect("DATABASE_URL de prueba sintácticamente válido");
    AppState {
        pool,
        upload_dir: "target/game-ws-tcp-test-uploads".to_string(),
        resend_api_key: None,
        email_from: "test@example.invalid".to_string(),
        stripe_secret_key: None,
        stripe_webhook_secret: None,
        game_ticket_secret: Some(TEST_SECRET.to_string()),
        game_ticket_store: GameTicketStore::default(),
        game_ws_state: GameWsState::with_max_connections(max_connections),
        site_url: "http://localhost:3000".to_string(),
        login_rate_limit: Arc::new(Mutex::new(
            HashMap::<String, (u8, std::time::Instant)>::new(),
        )),
        auth_action_rate_limit: Arc::new(Mutex::new(
            HashMap::<String, (u8, std::time::Instant)>::new(),
        )),
    }
}

async fn spawn_server(state: AppState) -> (String, oneshot::Sender<()>, JoinHandle<()>) {
    let app = create_router_with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener TCP efímero");
    let address = listener.local_addr().expect("dirección del listener");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server_handle = tokio::spawn(async move {
        serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("servidor WebSocket debe finalizar limpiamente");
    });
    (
        format!("ws://{address}/api/game/ws"),
        shutdown_tx,
        server_handle,
    )
}

fn join_message(ticket: &str) -> Message {
    Message::Text(
        serde_json::json!({
            "type": "join",
            "v": 1,
            "payload": {
                "ticket": ticket,
                "clientVersion": "game-01"
            }
        })
        .to_string(),
    )
}

async fn read_error<S>(socket: &mut tokio_tungstenite::WebSocketStream<S>) -> Value
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    while let Some(message) = socket.next().await {
        match message.expect("frame WebSocket válido") {
            Message::Text(text) => return serde_json::from_str(&text).expect("error JSON"),
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => {}
        }
    }
    panic!("el servidor debe enviar un error antes de cerrar");
}

#[tokio::test]
async fn valid_join_over_tcp_returns_map_unavailable_and_closes() {
    let state = test_state();
    let ticket = state
        .game_ticket_store
        .issue(Uuid::new_v4(), 30, TEST_SECRET)
        .expect("ticket válido");
    let (url, shutdown, server_handle) = spawn_server(state).await;
    let (mut socket, _) = connect_async(url).await.expect("upgrade WebSocket válido");

    socket
        .send(join_message(&ticket))
        .await
        .expect("join debe enviarse");
    let error = read_error(&mut socket).await;
    assert_eq!(error["type"], "error");
    assert_eq!(error["v"], 1);
    assert_eq!(error["payload"]["code"], "map_unavailable");
    assert_eq!(error["payload"]["fatal"], true);
    assert!(matches!(
        socket.next().await,
        Some(Ok(Message::Close(_))) | None
    ));

    let _ = shutdown.send(());
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn replayed_ticket_over_tcp_is_rejected_without_reopening_identity() {
    let state = test_state();
    let ticket = state
        .game_ticket_store
        .issue(Uuid::new_v4(), 30, TEST_SECRET)
        .expect("ticket válido");
    let (url, shutdown, server_handle) = spawn_server(state).await;

    let (mut first, _) = connect_async(&url).await.expect("primer upgrade");
    first
        .send(join_message(&ticket))
        .await
        .expect("primer join");
    let first_error = read_error(&mut first).await;
    assert_eq!(first_error["payload"]["code"], "map_unavailable");
    assert!(matches!(
        first.next().await,
        Some(Ok(Message::Close(_))) | None
    ));

    let (mut replay, _) = connect_async(url).await.expect("segundo upgrade");
    replay
        .send(join_message(&ticket))
        .await
        .expect("join replay");
    let replay_error = read_error(&mut replay).await;
    assert_eq!(replay_error["payload"]["code"], "unauthorized");
    assert!(matches!(
        replay.next().await,
        Some(Ok(Message::Close(_))) | None
    ));

    let _ = shutdown.send(());
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn malformed_first_message_over_tcp_is_rejected() {
    let (url, shutdown, server_handle) = spawn_server(test_state()).await;
    let (mut socket, _) = connect_async(url).await.expect("upgrade WebSocket válido");

    socket
        .send(Message::Text(
            serde_json::json!({
                "type": "move",
                "v": 1,
                "payload": { "sequence": 1, "direction": { "x": 0, "z": 0 } }
            })
            .to_string(),
        ))
        .await
        .expect("mensaje inválido debe enviarse");
    let error = read_error(&mut socket).await;
    assert_eq!(error["payload"]["code"], "invalid_message");
    assert!(matches!(
        socket.next().await,
        Some(Ok(Message::Close(_))) | None
    ));

    let _ = shutdown.send(());
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn second_tcp_upgrade_is_rejected_when_global_capacity_is_full() {
    let (url, shutdown, server_handle) = spawn_server(test_state_with_capacity(1)).await;
    let (first_socket, _) = connect_async(&url).await.expect("primer upgrade");

    let second_result = connect_async(url).await;
    match second_result {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status().as_u16(), 409);
        }
        other => panic!("el segundo upgrade debe recibir HTTP 409, recibió {other:?}"),
    }

    drop(first_socket);
    let _ = shutdown.send(());
    server_handle.await.expect("server shutdown");
}
