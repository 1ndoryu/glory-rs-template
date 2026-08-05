//! GAME-01 — Presupuesto de conexiones del transporte realtime.
//!
//! Este módulo limita handshakes/sockets activos y conserva el estado de la sala
//! realtime en la primera instancia. El actor de sala posee jugadores, mapa y
//! fanout; este wrapper expone sus límites al handler.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::game_room::{GameRoomState, RoomJoinError};
use super::game_room_map::GameRoomMap;

pub const GAME_WS_DEFAULT_MAX_CONNECTIONS: usize = 64;
const GAME_WS_DEFAULT_ROOM_TTL_SECS: u64 = 300;

#[derive(Clone)]
pub struct GameWsState {
    active_connections: Arc<AtomicUsize>,
    max_connections: usize,
    room_state: GameRoomState,
}

impl Default for GameWsState {
    fn default() -> Self {
        Self::with_max_connections(GAME_WS_DEFAULT_MAX_CONNECTIONS)
    }
}

impl GameWsState {
    #[must_use]
    pub fn with_max_connections(max_connections: usize) -> Self {
        Self::with_max_connections_and_room_ttl(max_connections, GAME_WS_DEFAULT_ROOM_TTL_SECS)
    }

    /// Crea el estado con un TTL de sala explícito. El constructor productivo
    /// anterior conserva 300 segundos; los benchmarks pueden usar `0`.
    #[must_use]
    pub fn with_max_connections_and_room_ttl(max_connections: usize, room_ttl_secs: u64) -> Self {
        Self {
            active_connections: Arc::new(AtomicUsize::new(0)),
            max_connections,
            room_state: GameRoomState::empty_with_ttl(room_ttl_secs),
        }
    }

    /// Reserva una conexión sin superar el límite global.
    #[must_use]
    pub fn try_acquire(&self) -> Option<GameWsConnectionGuard> {
        loop {
            let current = self.active_connections.load(Ordering::Acquire);
            if current >= self.max_connections {
                return None;
            }
            if self
                .active_connections
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Some(GameWsConnectionGuard {
                    active_connections: Arc::clone(&self.active_connections),
                });
            }
        }
    }

    #[must_use]
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn room_state(&self) -> GameRoomState {
        self.room_state.clone()
    }

    pub fn set_room_map(&self, map: Option<GameRoomMap>) {
        self.room_state.set_map(map);
    }

    pub fn has_room_map(&self) -> bool {
        self.room_state.has_map()
    }

    #[must_use]
    pub fn room_join_error_code(
        error: RoomJoinError,
    ) -> crate::models::game_realtime::GameRealtimeErrorCode {
        error.code()
    }
}

pub struct GameWsConnectionGuard {
    active_connections: Arc<AtomicUsize>,
}

impl Drop for GameWsConnectionGuard {
    fn drop(&mut self) {
        self.active_connections.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::GameWsState;

    #[test]
    fn enforces_capacity_and_releases_on_guard_drop() {
        let state = GameWsState::with_max_connections(1);
        let guard = state.try_acquire().expect("first connection");
        assert_eq!(state.active_connections(), 1);
        assert!(state.try_acquire().is_none());
        drop(guard);
        assert_eq!(state.active_connections(), 0);
        assert!(state.try_acquire().is_some());
    }

    #[test]
    fn zero_capacity_fails_closed() {
        let state = GameWsState::with_max_connections(0);
        assert!(state.try_acquire().is_none());
        assert_eq!(state.active_connections(), 0);
    }
}
