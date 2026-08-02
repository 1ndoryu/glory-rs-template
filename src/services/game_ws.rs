//! GAME-01 — Presupuesto de conexiones del transporte realtime.
//!
//! Este módulo solo limita handshakes/sockets activos en la primera instancia.
//! No representa salas, jugadores ni fanout; esas responsabilidades pertenecen
//! a la fase posterior del actor de sala.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub const GAME_WS_DEFAULT_MAX_CONNECTIONS: usize = 64;

#[derive(Clone)]
pub struct GameWsState {
    active_connections: Arc<AtomicUsize>,
    max_connections: usize,
}

impl Default for GameWsState {
    fn default() -> Self {
        Self::with_max_connections(GAME_WS_DEFAULT_MAX_CONNECTIONS)
    }
}

impl GameWsState {
    #[must_use]
    pub fn with_max_connections(max_connections: usize) -> Self {
        Self {
            active_connections: Arc::new(AtomicUsize::new(0)),
            max_connections,
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
