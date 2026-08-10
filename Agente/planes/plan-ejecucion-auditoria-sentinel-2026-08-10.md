# Plan — Ejecución de la corrección: Auditoría completa de Glory Sentinel y el quality gate

> **Fecha:** 2026-08-10
> **Rama objetivo:** `wandorius`
> **Estado:** EN EJECUCIÓN — Fase 0 iniciada (2026-08-10)
> **ID operativo:** `108A-1` (tomada por `buffy`)
> **Fuente del plan:** `Agente/documentacion/herramientas/auditoria-sentinel-completa-2026-08-10.md` §14
> (Plan integral de corrección por fases F0–F9). Este documento es solo seguimiento operativo; el
> detalle, checklists, gate y DoD viven en la auditoría §14. No duplica decisiones.
> **Autorización:** el usuario pidió ejecutar todas las tareas de la auditoría en orden
> (2026-08-10). Push/publicación remota requieren autorización explícita adicional (Fase 8).

## Alcance y no-goals (heredados de la auditoría §14.1)

- Ejecutar las fases F0→F9 **en orden**, cerrando cada fase con su gate antes de avanzar.
- No añadir fast paths, cache compartida, `task:close` ni nueva coordinación a `scripts/quality`.
- No retirar legacy sin paridad, rollback y adopción en dos consumidores.
- No tocar `tools/sentinel` en sitio dentro del gate del consumidor (rechazo por lock); los cambios
  upstream van en worktree exclusivo y solo se adoptan tras release publicado.
- No ejecutar deploy, push remoto ni escrituras externas sin autorización explícita.

## Estado registrado antes de editar (Fase 0, checklist)

- **Rama:** `wandorius` (checkout `glory-rust-template`), `ahead 1` de `origin/wandorius`; no se
  hace push.
- **Pins:** Sentinel `v0.6.0` / `44dc8fa00c9ac498e64cad0d6a4edd16afa752d8` (submódulo
  `tools/sentinel`); VarSense `e8360927ee92c4067f1f501dd77b951c8bc4f61d` (submódulo
  `tools/varsense`); glory-rs `ec33d5200ff587543ae1611971ca196b50f2b17a`.
- **Cambios preexistentes ajenos (preservados, ownership resuelto por esta tarea):**
  - `roadmap.md`: bloque 098A-1 añadido por el agente anterior (aprobado por el usuario 09-08).
  - `scripts/quality/task-check.mjs`: fragmento de medición F0 de 098A-1 **incompleto** — usa
    `preflightStartedAt` sin declararlo (`ReferenceError` en todo `task:check`).
  - Untracked: `Agente/documentacion/herramientas/auditoria-sentinel-completa-2026-08-10.md` y
    `Agente/planes/plan-agilizar-ceremonia-cierre-calidad-2026-08-09.md` (historia; no se borran).
- **Reportes previos:** ninguno válido para `108A-1`; todo gate posterior al fragmento F0 de 098A-1
  fallaba con `ReferenceError`. Línea base a reconstruir tras el hotfix.
- **Guard pesado:** sin full activo ni cooldown (`quality:guard` → `projects: {}`, `active: null`).
- **Toma de tarea:** `108A-1` tomada por `buffy` (`T-1786338220802-ba8d5987`).

## Estado de fases

| Fase | Estado | Nota |
| --- | --- | --- |
| F0 — Contención urgente y baseline confiable | EN CURSO | hotfix P0 + phaseDurationMs + tests + doctor/lock + gate real + baseline |
| F1 — Corregir contratos de Sentinel | pendiente | requiere worktree upstream exclusivo de `tools/sentinel` y release publicado |
| F2 — Sentinel modular único | pendiente | depende de F1 |
| F3 — Rendimiento VarSense/setup/suites | pendiente | depende de F2 |
| F4 — Bootstrap `sentinel init` | pendiente | depende de F1–F3 (artifacts) |
| F5 — Migrar consumidor y consolidar gate | pendiente | depende de F4 |
| F6 — Escalabilidad local, seguridad, operación | pendiente | depende de F5 |
| F7 — Consolidar documentación | pendiente | depende de contratos publicados |
| F8 — Release, adopción y retirada legacy | pendiente | push/publicación requiere autorización explícita del usuario |
| F9 — Verificación final y cierre | pendiente | depende de F8 |

## Decisión sobre 098A-1 (absorbido)

- El plan `plan-agilizar-ceremonia-cierre-calidad-2026-08-09.md` queda como historia (no se borra).
- Su F0 (medición de fases) se completa dentro de esta Fase 0 (el hotfix es exactamente el fragmento
  incompleto que dejó).
- F1–F6 de 098A-1 NO se implementan en `scripts/quality`: se reubican según la tabla de la auditoría
  §14.1 (Core/planner/CLI) en fases posteriores de este plan.

## Checklist Fase 0 (seguimiento de ejecución)

- [x] Tarea `108A-1` creada y tomada por `buffy`; estado Git/rama/pins/reportes registrados arriba.
- [x] Corregir `preflightStartedAt` (ReferenceError P0) y conectar `phaseDurationMs`
      (preflight/maintenance/stage/report) a `metrics.json` sin cambiar decisiones.
- [x] Prueba de proceso del entry point real + caso negativo de variables de medición.
- [x] `node --check`, tests focalizados y suite `quality:test` completa (230/231 PASS, 1 skip).
- [x] `quality:doctor` y `quality:lock -- --check` PASS.
- [x] Contención de analizadores: excluir `**/.sentinel/**` (backups de worktrees), `**/.vscode-test/**`
      y `**/tools/**` de VarSense; `**/.sentinel/**` de Sentinel (causa: 1 GB de artifacts VS Code
      y backups con fixtures del analyzer rompían/crasheaban el análisis; `Invalid string length`).
- [x] Propagar los tokens de sanción del gate (`GLORY_QUALITY_GATE_TOKEN`/`GLORY_HEAVY_RUN_TOKEN`)
      al entorno de las etapas (allowlist del runner): shims globales bloqueaban cargo fmt y
      run-with-db clippy/test chocaban con el lease pesado del propio gate.
- [x] Inventario inicial de entrypoints/imports/reglas/etapas custom + marca de compatibilidad
      temporal de `task:check` (congelar features) — actualizado en
      `Agente/documentacion/herramientas/inventario-scripts-adapters-sentinel-2026-08-06.md`.
- [x] Corrección preventiva de la skill global `quality-gate-setup` v1.1.0 (retirada la orden de
      copiar `scripts/quality`, inventario legacy, sin prometer migración inexistente; backup
      `.bak-2026-08-10`).
- [x] Gate real `task:check -- 108A-1`: full automático estructurado (FAIL, 100,5 s, todas las
      etapas) y **PASS local-light** (3,4 s; sentinel+docs) tras las correcciones. Baseline en
      `Agente/prevencion/bench-ceremonia-2026-08-09.md`.
- [ ] Gate **full** definitivo tras cooldown (~11:23Z) o con `--allow-heavy --heavy-reason`
      (requiere autorización explícita del usuario en el mismo turno, regla 028A-16).
- [x] Actualizar `roadmap.md` (098A-1 absorbido por 108A-1) y estado §14 de la auditoría.
- [ ] Commit coherente del bloque Fase 0 y `task:release`.

## Siguiente paso verificable

1. Inventario + skill `quality-gate-setup` + roadmap + estado §14 auditoría.
2. Gate real `task:check -- 108A-1` PASS frío y warm; baseline en
   `Agente/prevencion/bench-ceremonia-2026-08-09.md`.
3. Commit coherente y `task:release`.
