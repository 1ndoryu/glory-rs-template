# Plan — Reinicio coordinado del Bosque (decisión 8)

> **Fecha:** 2026-08-05 · **Epic:** GAME-01 · **Tarea planificada:** 297A-78
> **Estado:** planificación; contrato `server_restart` y UX del cliente cerrados.
> **Fuente:** `Agente/documentacion/producto/decisiones-pendientes-bosque-2026-08-05.md` §8 y
> ADR `Agente/documentacion/arquitectura/adr-bosque-mundo-unico-reinicio-coordinado-2026-08-05.md`.

## Objetivo y límites

- Al publicar una versión nueva del mapa, el servidor avisa a todos los jugadores
  que **el mundo se reiniciará en 5 minutos** y, tras la cuenta atrás, migra de
  forma coordinada a la versión nueva. No se mantienen salas con snapshots
  inmutables antiguos (reemplaza la política de "solo salas nuevas").
- **Fuera de alcance:** matchmaking, réplicas, rollback automático (se revisa en
  el runbook), cuentas atrás configurables por admin (fija 300 s).

## Ya cerrado (05-ago)

- [x] Contrato `server_restart` en ambos stacks (`game_realtime.rs` / `game-realtime.ts`)
  con motivo bounded (200) y cuenta atrás 1..=3600 s, validación fail-closed y tests.
- [x] Cliente: callback `onServerRestart` en `game-realtime-client.ts` + banner de
  cuenta atrás (`game-restart-notice.ts`, show/hide/destroy, 6 tests); la reconexión
  tras el cierre del socket ya usa backoff con jitter (297A-57), y el join recarga la
  versión activa de la BD (297A-65).

## Fase 1 — Broadcast del aviso (backend)

- [ ] `RoomCommand::Broadcast { message }` en `run_room`: enviar el mensaje a todos
  los players activos (`try_send` al output, sin bloquear el actor ni el tick).
- [ ] `GameRoomState::announce_restart(reason, seconds)`: iterar los rooms activos y
  difundir `GameRealtimeServerMessage::ServerRestart` (v:1).
- [ ] `GameWsState::announce_restart(reason, seconds)` como passthrough del wrapper.
- [ ] Tests: el broadcast llega a todos los players de cada sala; sala vacía no falla;
  room inexistente es no-op.

**Gate F1:** cargo check + tests de `game_room` (en el full CI por cooldown).

## Fase 2 — Trigger de publicación y migración coordinada

- [ ] `publish_map` (handler): tras `GameMapService::publish` exitoso, `tokio::spawn`
  un task que difunde `announce_restart("publicación de versión nueva", 300)` y espera
  la cuenta atrás.
- [ ] Al expirar: drenar las salas (`RoomCommand::Shutdown` / cerrar actores): los
  sockets se cierran y el cliente reintenta con backoff; el join recarga la versión
  activa nueva de la BD.
- [ ] Publicaciones concurrentes durante la cuenta atrás: la primera gana (no
  acumular tasks); documentar sin over-engineering.
- [ ] Sin jugadores conectados: el aviso es no-op; el primer join tras el drenaje crea
  la sala con la versión nueva.
- [ ] Tests con reloj inyectado/cuenta corta (p. ej. 1 s) + prueba TCP de reconexión.

**Gate F2:** cargo check + tests Rust (full CI) + prueba TCP.

## Fase 3 — Verificación y cierre

- [ ] `task:check -- 297A-78 --full` (tras cooldown, sin excepción manual — SNT-11).
- [ ] Flujo real en navegador: publicar → banner con cuenta atrás → socket cierra →
  reconexión → mundo nuevo renderiza.
- [ ] Runbook de rollback actualizado con la política de migración coordinada.
- [ ] Decisiones §8, roadmap y ADR coherentes.

**Gate F3 / DoD:** gate local-light PASS, full CI PASS (clippy + tests Rust), suite
frontend completa verde, navegador verificado y documentación sincronizada.

## Pruebas obligatorias

- Frontend: `game-restart-notice.test.ts` + suite completa + type-check.
- Backend: tests de `game_room` (broadcast/shutdown), prueba TCP de reconexión.
- Gate: `npm run task:check -- 297A-78` y `--full` tras cooldown.
