# Plan — Preflight reproducible y recuperación segura de Sentinel

> **Fecha:** 2026-08-07
> **Estado:** diseñado; implementación inicial en la rama upstream SNT-16d
> **Dependencia:** SNT-16c (manifest versionado y compatibilidad legacy)

## Problema

Un gate puede fallar tarde si un submódulo no está inicializado, su CLI no está compilado, el checkout fue modificado por una instalación interrumpida o `quality-tools.json` y `sentinel.lock.json` apuntan a commits distintos. Una interrupción también puede dejar claims/worktrees activos aunque el proceso original ya no exista.

## Objetivo

Bloquear antes de ejecutar cuando el entorno no es reproducible y ofrecer recuperación explícita, segura y auditable de tareas expiradas. Sentinel debe diagnosticar; nunca reparar silenciosamente ni borrar un worktree vivo o sucio.

## Fases

### SNT-16d — Doctor/preflight fail-closed

- [x] `sentinel doctor` expone `ready`, problemas codificados y estado de cada herramienta.
- [x] Detectar sourcePath ausente, CLI compilado ausente, checkout Git inválido/sucio, gitlink divergente y commits inconsistentes con el lock.
- [ ] Conectar `assertWorkspaceReady` al gate real después de adaptar fixtures de transición/no-policy.
- [ ] Añadir JSON estable y códigos de salida no cero cuando `ready=false`.
- [ ] Cubrir instalación incompleta, lock divergente y checkout modificado con fixtures.

### SNT-16e — Recuperación de tareas interrumpidas

- [ ] `task status` distingue activa, expirada, proceso vivo, worktree sucio y metadatos inválidos.
- [ ] `task recover <id>` exige toma expirada, PID muerto, namespace válido y worktree limpio.
- [ ] `recover --dry-run` solo inspecciona; la recuperación real valida todo antes de cleanup.
- [ ] Nunca borrar recursos ajenos, worktrees vivos, ramas divergentes ni cambios no commiteados.
- [ ] Registrar motivo, agente, timestamp y resultado en metadata de coordinación.

## Criterios de salida

- Doctor falla cerrado antes de iniciar un gate no reproducible.
- Fixtures positivas/negativas y suite upstream pasan.
- Consumidor conserva rollback a Sentinel 0.5.0 hasta release publicada y lock regenerado.
- No se eliminan `scripts/quality` ni scripts de dominio antes de dos releases con paridad.
