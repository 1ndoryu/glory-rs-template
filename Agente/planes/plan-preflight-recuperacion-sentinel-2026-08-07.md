# Plan — Preflight reproducible y recuperación segura de Sentinel

> **Fecha:** 2026-08-07
> **Estado:** implementación local/upstream verificada; adopción estable pendiente de release upstream y clon limpio
> **Dependencia:** SNT-16c (manifest versionado y compatibilidad legacy)

## Problema

Un gate puede fallar tarde si un submódulo no está inicializado, su CLI no está compilado, el checkout fue modificado por una instalación interrumpida o `quality-tools.json` y `sentinel.lock.json` apuntan a commits distintos. Una interrupción también puede dejar claims/worktrees activos aunque el proceso original ya no exista.

## Objetivo

Bloquear antes de ejecutar cuando el entorno no es reproducible y ofrecer recuperación explícita, segura y auditable de tareas expiradas. Sentinel debe diagnosticar; nunca reparar silenciosamente ni borrar un worktree vivo o sucio.

## Resultado por fase

### SNT-16d — Doctor/preflight fail-closed — implementado en upstream

- [x] `sentinel doctor` expone `ready`, problemas codificados y estado de cada herramienta.
- [x] Detecta sourcePath/sourcePathEnv ausente, CLI compilado ausente, respuesta `--version`, checkout Git inválido/sucio, gitlink divergente y commits/versiones inconsistentes con el lock.
- [x] `assertWorkspaceReady` está conectado al gate real; los dry-runs conservan el cálculo de alcance sin ejecutar etapas.
- [x] El diagnóstico mantiene compatibilidad con el campo legado `tools.<name>.commit` y ofrece salida legible/JSON desde el CLI existente.
- [x] Fixtures cubren política/lock, ausencia de instalación, CLI ausente y mismatch; compilación TypeScript PASS.

### SNT-16e — Recuperación de tareas interrumpidas — implementado en upstream

- [x] `task recover <id>` y `--dry-run` tienen contrato CLI explícito, sin reutilizar `--force`.
- [x] La recuperación exige toma expirada, PID muerto, task-id/agent seguros, namespace interno, heads de rama/worktree consistentes y worktree limpio.
- [x] `recover --dry-run` solo inspecciona; la recuperación real valida antes de delegar el cleanup existente.
- [x] Nunca borra recursos ajenos, worktrees vivos, ramas divergentes ni cambios no commiteados.
- [x] La recuperación real escribe auditoría JSON con agente, tarea, estado anterior, timestamp, staleForMs y resultado bajo `.sentinel/recovery/`.
- [ ] Ampliar `task status` con estado derivado `expired/processAlive/worktreeClean` para observabilidad directa (siguiente bloque, no necesario para el guard fail-closed).

## Evidencia

- Commits upstream de tarea: `e1493c3` (gate/recovery), `ff0649c` (doctor reforzado); el worktree consumidor fija el gitlink a `ff0649c`.
- `tsc` sin errores.
- Suite upstream: **499 passing, 1 pending**.
- Focalizados doctor/recovery/CLI: PASS.
- Generador de lock: `--write` y después `--check --json`: PASS; configured/checkout/lock usan `ff0649c7a1b88596d42921f865a6e6871acfe0db`.
- Limitación real: el wrapper `npm run compile` intenta cargar un `quality-command-guard.mjs` que no existe en el checkout upstream. La compilación directa y la suite sí fueron ejecutadas; no se declara PASS del wrapper ausente.

## Bloqueadores de adopción estable

- Publicar el commit upstream en una rama/tag de release permitido; una rama de trabajo no es release.
- Validar desde clon limpio con dependencias provisionadas, `--version`, lock y suite.
- Ejecutar dos proyectos consumidores independientes con envelope y legacy y comparar decisión, hallazgos, severidad y mensaje.

## Criterios de salida

- Doctor falla cerrado antes de iniciar un gate no reproducible.
- Fixtures positivas/negativas y suite upstream pasan.
- Consumidor conserva rollback a Sentinel 0.5.0 hasta release publicada y lock regenerado.
- No se eliminan `scripts/quality` ni scripts de dominio antes de dos releases con paridad.
