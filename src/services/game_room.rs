//! GAME-01 — Actor server-authoritative de una sala realtime.
//!
//! El actor es dueño exclusivo de jugadores, secuencias y posiciones. El
//! transporte solo entrega mensajes ya parseados y recibe envelopes bounded.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

use crate::models::game_realtime::{
    assess_sequence, consume_rate_budget, GameRealtimeClientMessage, GameRealtimeEntity,
    GameRealtimeErrorCode, GameRealtimeErrorPayload, GameRealtimeHeartbeatAckPayload,
    GameRealtimeServerMessage, GameRealtimeSnapshotPayload, SequenceDecision,
    GAME_REALTIME_MAX_PLAYERS_PER_ROOM, GAME_REALTIME_PROTOCOL_VERSION,
};
use crate::services::game_room_map::GameRoomMap;

const ROOM_COMMAND_CAPACITY: usize = 64;
const ROOM_TICK_MILLIS: u64 = 100;
const ROOM_TICK_SECONDS: f64 = 0.1;
const ROOM_EMPTY_TTL_SECS: u64 = 300;
const ROOM_INTEREST_RADIUS: f64 = 32.0;
const PLAYER_RADIUS: f64 = 0.5;
const MAX_RATE_HISTORY: usize = 20;

#[must_use]
fn empty_room_expired(empty_since: u64, now: u64, ttl_secs: u64) -> bool {
    now.saturating_sub(empty_since) >= ttl_secs
}

#[derive(Clone)]
pub struct GameRoomState {
    map: Arc<RwLock<Option<Arc<GameRoomMap>>>>,
    room: Arc<tokio::sync::Mutex<Option<RoomHandle>>>,
    empty_ttl_secs: u64,
}

impl Default for GameRoomState {
    fn default() -> Self {
        Self::empty()
    }
}

impl GameRoomState {
    #[must_use]
    pub fn empty() -> Self {
        Self::empty_with_ttl(ROOM_EMPTY_TTL_SECS)
    }

    #[must_use]
    pub fn empty_with_ttl(empty_ttl_secs: u64) -> Self {
        Self {
            map: Arc::new(RwLock::new(None)),
            room: Arc::new(tokio::sync::Mutex::new(None)),
            empty_ttl_secs,
        }
    }

    #[must_use]
    pub fn with_map(map: GameRoomMap) -> Self {
        Self::with_map_and_ttl(map, ROOM_EMPTY_TTL_SECS)
    }

    #[must_use]
    pub fn with_map_and_ttl(map: GameRoomMap, empty_ttl_secs: u64) -> Self {
        Self {
            map: Arc::new(RwLock::new(Some(Arc::new(map)))),
            room: Arc::new(tokio::sync::Mutex::new(None)),
            empty_ttl_secs,
        }
    }

    pub async fn set_map(&self, map: Option<GameRoomMap>) {
        *self.map.write().await = map.map(Arc::new);
    }

    pub async fn has_map(&self) -> bool {
        self.map.read().await.is_some()
    }

    pub async fn join(
        &self,
        subject: Uuid,
        output: mpsc::Sender<GameRealtimeServerMessage>,
    ) -> Result<JoinedRoom, RoomJoinError> {
        let map = self
            .map
            .read()
            .await
            .clone()
            .ok_or(RoomJoinError::MapUnavailable)?;
        let mut room = self.room.lock().await;
        let handle = match room.as_ref() {
            Some(handle) if !handle.is_closed() => handle.clone(),
            _ => {
                let handle = RoomHandle::start(map.clone(), self.empty_ttl_secs);
                *room = Some(handle.clone());
                handle
            }
        };
        drop(room);
        match handle.join(subject, output.clone()).await {
            Err(RoomJoinError::Busy) if handle.is_closed() => {
                let mut room = self.room.lock().await;
                let replacement = match room.as_ref() {
                    Some(current) if !current.is_closed() => current.clone(),
                    _ => {
                        let replacement = RoomHandle::start(map, self.empty_ttl_secs);
                        *room = Some(replacement.clone());
                        replacement
                    }
                };
                drop(room);
                replacement.join(subject, output).await
            }
            result => result,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomJoinError {
    MapUnavailable,
    Full,
    Busy,
}

impl RoomJoinError {
    #[must_use]
    pub fn code(self) -> GameRealtimeErrorCode {
        match self {
            Self::MapUnavailable => GameRealtimeErrorCode::MapUnavailable,
            Self::Full => GameRealtimeErrorCode::RoomFull,
            Self::Busy => GameRealtimeErrorCode::ServerBusy,
        }
    }
}

#[derive(Clone)]
pub struct JoinedRoom {
    pub player_id: String,
    pub map_version: String,
    pub tick: u64,
    pub initial_snapshot: GameRealtimeSnapshotPayload,
    handle: RoomHandle,
}

impl JoinedRoom {
    pub fn send(&self, message: GameRealtimeClientMessage) -> Result<(), RoomSendError> {
        self.handle.send(RoomCommand::Client {
            player_id: self.player_id.clone(),
            message,
        })
    }

    pub async fn disconnect(&self) {
        let _ = self.handle.disconnect(self.player_id.clone()).await;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomSendError {
    Closed,
    Backpressure,
}

#[derive(Clone)]
struct RoomHandle {
    commands: mpsc::Sender<RoomCommand>,
    closed: Arc<AtomicBool>,
}

impl RoomHandle {
    fn start(map: Arc<GameRoomMap>, empty_ttl_secs: u64) -> Self {
        let (commands, receiver) = mpsc::channel(ROOM_COMMAND_CAPACITY);
        let handle = Self {
            commands,
            closed: Arc::new(AtomicBool::new(false)),
        };
        tokio::spawn(run_room(map, receiver, handle.clone(), empty_ttl_secs));
        handle
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    async fn join(
        &self,
        subject: Uuid,
        output: mpsc::Sender<GameRealtimeServerMessage>,
    ) -> Result<JoinedRoom, RoomJoinError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(RoomCommand::Join {
                subject,
                output,
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => RoomJoinError::Busy,
                mpsc::error::TrySendError::Closed(_) => {
                    self.closed.store(true, Ordering::Release);
                    RoomJoinError::Busy
                }
            })?;
        response.await.map_err(|_| RoomJoinError::Busy)?
    }
    fn send(&self, command: RoomCommand) -> Result<(), RoomSendError> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => RoomSendError::Backpressure,
                mpsc::error::TrySendError::Closed(_) => RoomSendError::Closed,
            })
    }

    async fn disconnect(&self, player_id: String) -> Result<(), RoomSendError> {
        self.commands
            .send(RoomCommand::Disconnect { player_id })
            .await
            .map_err(|_| RoomSendError::Closed)
    }
}

enum RoomCommand {
    Join {
        subject: Uuid,
        output: mpsc::Sender<GameRealtimeServerMessage>,
        reply: oneshot::Sender<Result<JoinedRoom, RoomJoinError>>,
    },
    Client {
        player_id: String,
        message: GameRealtimeClientMessage,
    },
    Disconnect {
        player_id: String,
    },
}

struct RoomPlayer {
    subject: Uuid,
    output: mpsc::Sender<GameRealtimeServerMessage>,
    position: (f64, f64),
    velocity: (f64, f64),
    direction: (f64, f64),
    last_sequence: Option<u64>,
    rate_history: Vec<u64>,
}

async fn run_room(
    map: Arc<GameRoomMap>,
    mut commands: mpsc::Receiver<RoomCommand>,
    handle: RoomHandle,
    empty_ttl_secs: u64,
) {
    let mut players = HashMap::<String, RoomPlayer>::new();
    let mut tick = 0_u64;
    let mut snapshot_sequence = 0_u64;
    let mut empty_since: Option<u64> = None;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(ROOM_TICK_MILLIS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            Some(command) = commands.recv() => match command {
                RoomCommand::Join { subject, output, reply } => {
                    let result = join_player(&map, &mut players, subject, output, handle.clone());
                    if result.is_ok() {
                        empty_since = None;
                    }
                    let _ = reply.send(result);
                }
                RoomCommand::Client { player_id, message } => {
                    handle_client_message(&mut players, &player_id, message, tick);
                }
                RoomCommand::Disconnect { player_id } => {
                    players.remove(&player_id);
                    if players.is_empty() {
                        empty_since = Some(now_secs());
                    }
                }
            },
            _ = interval.tick() => {
                tick = tick.saturating_add(1);
                update_players(&map, &mut players);
                snapshot_sequence = snapshot_sequence.saturating_add(1);
                broadcast_snapshot(&mut players, tick, snapshot_sequence);
                if players.is_empty() {
                    let since = empty_since.get_or_insert_with(now_secs);
                    if empty_room_expired(*since, now_secs(), empty_ttl_secs) {
                        break;
                    }
                } else {
                    empty_since = None;
                }
            }
            else => break,
        }
    }
    handle.closed.store(true, Ordering::Release);
}

fn join_player(
    map: &GameRoomMap,
    players: &mut HashMap<String, RoomPlayer>,
    subject: Uuid,
    output: mpsc::Sender<GameRealtimeServerMessage>,
    handle: RoomHandle,
) -> Result<JoinedRoom, RoomJoinError> {
    /* [297A-57] Reconexión persistente: el mismo subject reemplaza su conexión
     * previa en vez de ser rechazado. Al eliminar el RoomPlayer viejo, su
     * Sender se dropea y el handle_socket anterior se cierra solo (recv →
     * None), sin duplicar jugadores ni esperar el timeout del servidor. */
    let previous = players
        .iter()
        .find_map(|(id, player)| (player.subject == subject).then_some(id.clone()));
    if players.len() >= GAME_REALTIME_MAX_PLAYERS_PER_ROOM && previous.is_none() {
        return Err(RoomJoinError::Full);
    }
    if let Some(previous_id) = previous {
        players.remove(&previous_id);
    }
    let player_id = format!("p-{}", Uuid::new_v4().simple());
    let (x, z) = map.spawn_position(players.len());
    let initial = GameRealtimeSnapshotPayload {
        snapshot_sequence: 0,
        tick: 0,
        entities: vec![entity(&player_id, (x, z), (0.0, 0.0))],
    };
    players.insert(
        player_id.clone(),
        RoomPlayer {
            subject,
            output,
            position: (x, z),
            velocity: (0.0, 0.0),
            direction: (0.0, 0.0),
            last_sequence: None,
            rate_history: Vec::new(),
        },
    );
    Ok(JoinedRoom {
        player_id: player_id.clone(),
        map_version: map.map_version(),
        tick: 0,
        initial_snapshot: initial,
        handle,
    })
}

fn handle_client_message(
    players: &mut HashMap<String, RoomPlayer>,
    player_id: &str,
    message: GameRealtimeClientMessage,
    tick: u64,
) {
    let Some(player) = players.get_mut(player_id) else {
        return;
    };
    let now = now_millis();
    let Ok(history) = consume_rate_budget(&player.rate_history, now) else {
        send_error(
            player,
            GameRealtimeErrorCode::RateLimited,
            "rate limit realtime excedido",
            false,
        );
        return;
    };
    player.rate_history = history;
    if player.rate_history.len() > MAX_RATE_HISTORY {
        let keep_from = player.rate_history.len() - MAX_RATE_HISTORY;
        player.rate_history.drain(..keep_from);
    }
    match message {
        GameRealtimeClientMessage::Move { payload, .. } => {
            match assess_sequence(player.last_sequence, payload.sequence) {
                SequenceDecision::Accept => {
                    player.last_sequence = Some(payload.sequence);
                    player.direction = (payload.direction.x, payload.direction.z);
                }
                SequenceDecision::Replay => {
                    send_error(
                        player,
                        GameRealtimeErrorCode::SequenceReplay,
                        "secuencia realtime repetida",
                        false,
                    );
                }
                SequenceDecision::Jump => {
                    send_error(
                        player,
                        GameRealtimeErrorCode::SequenceJump,
                        "salto de secuencia realtime inválido",
                        false,
                    );
                }
            }
        }
        GameRealtimeClientMessage::Heartbeat { .. } => {
            send_message(
                player,
                GameRealtimeServerMessage::HeartbeatAck {
                    v: GAME_REALTIME_PROTOCOL_VERSION,
                    payload: GameRealtimeHeartbeatAckPayload { server_tick: tick },
                },
            );
        }
        GameRealtimeClientMessage::ClientAck { .. } => {}
        GameRealtimeClientMessage::Join { .. } => send_error(
            player,
            GameRealtimeErrorCode::InvalidMessage,
            "join solo puede ser el primer mensaje",
            true,
        ),
    }
}

fn update_players(map: &GameRoomMap, players: &mut HashMap<String, RoomPlayer>) {
    for player in players.values_mut() {
        let (position, velocity) = map.move_circle(
            player.position,
            player.direction,
            PLAYER_RADIUS,
            ROOM_TICK_SECONDS,
        );
        player.position = position;
        player.velocity = velocity;
    }
}

fn broadcast_snapshot(
    players: &mut HashMap<String, RoomPlayer>,
    tick: u64,
    snapshot_sequence: u64,
) {
    let mut entities = players
        .iter()
        .map(|(id, player)| entity(id, player.position, player.velocity))
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    let positions = players
        .iter()
        .map(|(id, player)| (id.clone(), player.position))
        .collect::<HashMap<_, _>>();
    let mut slow_players = Vec::new();
    for (id, player) in players.iter_mut() {
        let Some(owner_position) = positions.get(id) else {
            continue;
        };
        let visible = entities
            .iter()
            .filter(|candidate| {
                let dx = candidate.position.x - owner_position.0;
                let dz = candidate.position.z - owner_position.1;
                dx * dx + dz * dz <= ROOM_INTEREST_RADIUS * ROOM_INTEREST_RADIUS
            })
            .cloned()
            .collect();
        let message = GameRealtimeServerMessage::Snapshot {
            v: GAME_REALTIME_PROTOCOL_VERSION,
            payload: GameRealtimeSnapshotPayload {
                snapshot_sequence,
                tick,
                entities: visible,
            },
        };
        match player.output.try_send(message) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_) | mpsc::error::TrySendError::Closed(_)) => {
                slow_players.push(id.clone());
            }
        }
    }
    for id in slow_players {
        players.remove(&id);
    }
}

fn entity(id: &str, position: (f64, f64), velocity: (f64, f64)) -> GameRealtimeEntity {
    GameRealtimeEntity {
        id: id.to_string(),
        position: crate::models::game_realtime::GameRealtimeVector {
            x: position.0,
            z: position.1,
        },
        velocity: crate::models::game_realtime::GameRealtimeVector {
            x: velocity.0,
            z: velocity.1,
        },
        radius: PLAYER_RADIUS,
    }
}

fn send_error(player: &mut RoomPlayer, code: GameRealtimeErrorCode, message: &str, fatal: bool) {
    send_message(
        player,
        GameRealtimeServerMessage::Error {
            v: GAME_REALTIME_PROTOCOL_VERSION,
            payload: GameRealtimeErrorPayload {
                code,
                message: message.to_string(),
                fatal,
            },
        },
    );
}

fn send_message(player: &mut RoomPlayer, message: GameRealtimeServerMessage) {
    let _ = player.output.try_send(message);
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> GameRoomMap {
        GameRoomMap::from_parts(
            "forest".to_string(),
            1,
            super::super::game_room_map::RoomBounds {
                min_x: 0.0,
                max_x: 32.0,
                min_z: 0.0,
                max_z: 32.0,
            },
            Vec::new(),
            vec![super::super::game_room_map::RoomSpawn {
                x: 2.0,
                z: 2.0,
                radius: 1.0,
            }],
        )
        .expect("map fixture")
    }

    #[test]
    fn empty_room_ttl_is_bounded_and_saturating() {
        assert!(!empty_room_expired(100, 399, 300));
        assert!(empty_room_expired(100, 400, 300));
        assert!(empty_room_expired(0, u64::MAX, 300));
    }

    #[tokio::test]
    async fn room_accepts_join_and_moves_authoritatively() {
        let state = GameRoomState::with_map(map());
        let (output, mut messages) = mpsc::channel(32);
        let joined = state.join(Uuid::new_v4(), output).await.expect("join");
        assert!(joined.player_id.starts_with("p-"));
        assert_eq!(joined.map_version, "forest@1");
        joined
            .send(GameRealtimeClientMessage::Move {
                v: 1,
                payload: crate::models::game_realtime::GameRealtimeMovePayload {
                    sequence: 1,
                    direction: crate::models::game_realtime::GameRealtimeVector { x: 1.0, z: 0.0 },
                },
            })
            .expect("move");
        let snapshot = tokio::time::timeout(std::time::Duration::from_secs(1), messages.recv())
            .await
            .expect("snapshot timeout")
            .expect("snapshot");
        assert!(matches!(
            snapshot,
            GameRealtimeServerMessage::Snapshot { .. }
        ));
        joined.disconnect().await;
    }

    #[tokio::test]
    async fn empty_room_ttl_recreates_actor_after_disconnect() {
        let state = GameRoomState::with_map_and_ttl(map(), 0);
        let (first_output, first_messages) = mpsc::channel(32);
        let first = state
            .join(Uuid::new_v4(), first_output)
            .await
            .expect("first");
        drop(first_messages);
        first.disconnect().await;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let (second_output, _second_messages) = mpsc::channel(32);
        let second = state
            .join(Uuid::new_v4(), second_output)
            .await
            .expect("second");
        assert_ne!(first.player_id, second.player_id);
        second.disconnect().await;
    }

    #[tokio::test]
    async fn reconnect_replaces_previous_identity_without_duplication() {
        let state = GameRoomState::with_map(map());
        let subject = Uuid::new_v4();
        let (first_output, mut first_messages) = mpsc::channel(32);
        let first = state.join(subject, first_output).await.expect("first");

        /* [297A-57] La misma identidad reconecta: reemplaza la conexión previa
         * (id nuevo, sin duplicar jugadores). */
        let (second_output, mut second_messages) = mpsc::channel(32);
        let second = state.join(subject, second_output).await.expect("reconnect");
        assert_ne!(first.player_id, second.player_id);

        /* La conexión vieja dejó de recibir: al reemplazar, su Sender se
         * dropea y el canal se cierra (recv → Ok(None)). Se drena cualquier
         * snapshot en vuelo del primer tick y se verifica el cierre, no un
         * timeout (si el jugador viejo siguiera activo, llegaría un snapshot). */
        while first_messages.try_recv().is_ok() {}
        let outcome =
            tokio::time::timeout(std::time::Duration::from_millis(300), first_messages.recv())
                .await;
        assert!(
            matches!(outcome, Ok(None)),
            "la conexión vieja siguió recibiendo: {outcome:?}"
        );
        let snapshot =
            tokio::time::timeout(std::time::Duration::from_secs(1), second_messages.recv())
                .await
                .expect("snapshot")
                .expect("snapshot");
        assert!(matches!(
            snapshot,
            GameRealtimeServerMessage::Snapshot { .. }
        ));
        second.disconnect().await;
    }

    #[tokio::test]
    async fn room_rejects_ninth_player_and_keeps_reconnect_slots() {
        let state = GameRoomState::with_map(map());
        let subject = Uuid::new_v4();
        let mut receivers = Vec::new();
        /* [297A-57] El primer slot lo ocupa `subject` para poder probar su
         * reconexión después; el resto usa identidades frescas. */
        for index in 0..GAME_REALTIME_MAX_PLAYERS_PER_ROOM {
            let (output, messages) = mpsc::channel(32);
            receivers.push(messages);
            let id = if index == 0 { subject } else { Uuid::new_v4() };
            state.join(id, output).await.expect("capacity");
        }
        let (extra_output, extra_messages) = mpsc::channel(32);
        receivers.push(extra_messages);
        assert!(matches!(
            state.join(Uuid::new_v4(), extra_output).await,
            Err(RoomJoinError::Full)
        ));
        /* Una sala llena aún acepta la reconexión de un jugador presente. */
        let (reconnect_output, _reconnect_messages) = mpsc::channel(32);
        assert!(state.join(subject, reconnect_output).await.is_ok());
    }
}
