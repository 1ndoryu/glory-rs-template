# Plan — Migración de scripts a Sentinel Core y adapters por proyecto

> **Fecha:** 2026-08-06
> **Estado:** Fase 2 local cerrada como transición; Fase 3 (reducción del adapter) bloqueada hasta schema upstream, fixtures multi-proyecto, paridad, rollback y releases consecutivos
> **Ámbito:** calidad, coordinación de tareas y wrappers de desarrollo; no modifica todavía la skill global ni elimina scripts
> **Relación:** complementa `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md` y `Agente/planes/plan-sentinel-orquestacion-tareas-worktrees-2026-08-06.md`
> **Fuente canónica de esta iniciativa:** este documento

## 1. Decisión ejecutiva

El problema no es simplemente que haya muchos archivos. El checkout mezcla cuatro responsabilidades con vidas útiles distintas: plano universal, adaptación por proyecto, compatibilidad temporal y operación específica/histórica.

> **Sentinel Core/CLI es el único dueño del plano universal. Cada proyecto aporta un manifiesto declarativo y un adapter pequeño para su stack. Los scripts específicos permanecen únicamente cuando encapsulan una operación de dominio real que no puede generalizarse sin inventar una abstracción. Los aliases se mantienen durante una ventana de migración medible y luego se retiran por evidencia, no por limpieza estética.**

No se copia `scripts/quality` a otros repositorios ni se elimina mientras no existan paridad, rollback y dos releases. La personalización objetivo es configuración declarativa + pocos adapters versionados, con uno o dos comandos públicos para agentes.

## 2. Evidencia de la situación actual

Sentinel `0.5.0` (`20c13a216e879303fcf5be7469a2821391b2ec0d`) expone `check --stages`, contratos de etapas y coordinación de tareas. El consumidor conserva `scripts/quality` como implementación probada y frontera de transición; GC, runbook multi-OS, schema upstream y adopción multi-proyecto siguen pendientes.

## 3. Modelo objetivo

| Capa | Vive en | Dueño | Contiene | No contiene |
|---|---|---|---|---|
| Sentinel Core | upstream | tooling | lifecycle, scheduler, scope, cache, leases, report schema, redaction, runtime/shims, GC | rutas, dominio, reglas visuales |
| Project adapter | raíz del consumidor | proyecto | manifest de comandos/stages, checks de stack, VarSense y custom | scheduler, cooldown, claims, worktrees, reporte final |
| Scripts específicos | `scripts/` | producto/operaciones | DB/Cargo, codegen, rescates autorizados, fixtures | coordinación universal |
| Legacy/experimental | historial/carpeta legacy | mantenimiento | compatibilidad acotada | nuevas capacidades |

## 4. Fases ejecutables

### Fase 0 — Baseline, ownership y seguridad (`SNT-12`) — cerrada

Inventario versionado, baseline de Sentinel/VarSense, separación de cambios ajenos y auditoría focalizada de secretos. No se movieron ni eliminaron scripts.

### Fase 1 — Contrato mínimo de adapter (`SNT-13`) — transición local cerrada

`quality-adapter.json` define versión/protocolo, capabilities, transporte argv, stages, perfiles, timeouts, environment allowlist, output schema y exit-code mapping. `adapter-manifest.mjs` valida strict keys, placeholders, task IDs, paths dentro de workspace/report root, symlink/junction ancestors y entrypoint regular. La salida de `stage-process` es JSON schema v1.

### Fase 2 — Core/adapter slice (`SNT-14`) — cerrada en transición local

- el manifest es dueño de los nombres y perfiles de stages; la selección full/perfil y la implementación se comparan fail-closed;
- `stage-definitions`, `stages` y `stage-process` consumen el manifest; el camino legacy sigue explícito y probado solo como compatibilidad temporal;
- `runner.mjs` hereda únicamente una allowlist base no sensible; el manifest también rechaza variables sensibles (`DATABASE_URL`, tokens, keys, passwords y equivalentes), por lo que no se transportan credenciales por declaración implícita;
- frontend y Rust reciben la misma política declarativa de entorno; transportes conservan argv y `shell:false` salvo el shim CMD/BAT existente;
- `task-check` sigue siendo el orquestador de transición y carga el manifest mediante `stageDefinitions`; no se afirma todavía delegación completa a `sentinel check`.

**Evidencia SNT-14 (2026-08-06):** suite dirigida = **19 tests, 18 PASS, 1 skip, 0 fail**; el skip corresponde al observe end-to-end sin CLI Sentinel provisionado/limpio. `node --check` y `git diff --check` PASS. El gate consumidor fue limitado por la ausencia de `.env`/`DATABASE_URL` y `frontend/node_modules`; no se copian secretos ni se falsea el resultado.

### Fase 2b — Endurecimiento del slice (`SNT-15`) — cerrado localmente

**Implementado en el adapter del consumidor, sin tocar Sentinel upstream ni retirar scripts:**

- `manifestStageNames`, `adapterStageNames` y `assertImplementedStages` validan el manifest completo antes de leer stages o perfiles;
- cada definición de stage acepta únicamente `timeoutMs`, evitando claves silenciosas/typos;
- `readAdapterManifest` valida que `quality-adapter.json` y su entrypoint sean archivos regulares contenidos físicamente en el workspace, rechazando symlink/junction escape;
- la allowlist efectiva normaliza duplicados case-insensitive para Windows (`PATH`/`Path`) y rechaza nombres sensibles también en la base heredada;
- `stageDefinitions` usa el manifest como fuente única de selección/paridad en el camino real; el fallback legacy solo queda disponible sin `projectRoot` para tests/compatibilidad explícita;
- se añadieron fixtures de manifest enlazado, schema estricto, allowlist base, selección real desde disco, manifest inválido y stage sin factory.

**Evidencia SNT-15 (2026-08-06):** `node --check` de los dos módulos editados PASS; suite focalizada **18/18 PASS**; `git diff --check` PASS. No se declara gate completo ni observe end-to-end: faltan artefactos/entorno del consumidor en el worktree (CLI VarSense compilado, `.env`/BD y `frontend/node_modules`).

### Fase 3 — Reducir el adapter de wandori.us (`SNT-16`)

- [ ] Convertir las etapas de Rust, frontend, docs, custom y VarSense a manifest + adapters delgados.
- [ ] Reemplazar `task-check.mjs` por delegación a `sentinel check`, conservando el alias npm.
- [ ] Hacer que `run-with-db` solo resuelva DB/target y ejecute el comando permitido; el lease/guard universal debe venir de Sentinel.
- [ ] Retirar imports locales de cooldown, report, cache y takeover del adapter del consumidor.
- [ ] Mantener únicamente comandos públicos mínimos.

**Bloqueo actual:** Sentinel `0.5.0` tiene `check --stages`, pero no existe aún un schema upstream versionado para `quality-adapter.json` ni fixtures externos suficientes para justificar retirar el orquestador local.

### Fase 4 — Retirada controlada y simplificación (`SNT-17`)

- [ ] Dos releases consecutivos multi-shell/CI.
- [ ] GC/runbook y rollback reproducible.
- [ ] Retirar físicamente solo archivos sin referencias y con rollback documentado.
- [ ] Actualizar la skill global únicamente al final, con evidencia, versión, fixture y confirmación en sesión nueva.

## 5. Política de permanencia para scripts

Conservar scripts que encapsulen dominio/proveedor, adapter externo estable, experiencia humana/IDE, bootstrap reproducible o un segundo consumidor real. Migrar solo capacidades universales que aparezcan en más de un caso o sean claramente agnósticas. Archivar scripts históricos, de incidentes o de producción ajena; no migrarlos ni copiarlos.

## 6. Seguridad y no-sorpresas

No shell concatenado en manifests; paths contenidos y sin symlink/junction escapes; errores de herramienta fail-closed; no borrar `scripts/quality`; no desplegar ni ejecutar rescates/producción. El allowlist base no expone credenciales y el schema local rechaza cualquier variable sensible; un futuro transporte de secretos requerirá un contrato upstream explícito, capacidad auditable y revisión separada.

## 7. Definition of Done global

- [ ] inventario con owner real;
- [ ] schema y fixtures publicados en Sentinel;
- [ ] dos proyectos usan el mismo core sin copiar `scripts/quality`;
- [ ] VarSense solo analyzer;
- [ ] adapter wandorius delgado;
- [ ] cinco gates con paridad exacta y errores fail-closed;
- [ ] dos releases consecutivos multi-shell/CI;
- [ ] rollback y cleanup/GC probados;
- [x] documentación local refleja el estado real de Fase 1/Fase 2/SNT-15;
- [ ] skill global actualizada solo al final, si la evidencia lo justifica;
- [ ] cero secretos/procesos/locks/worktrees/ramas propias pendientes.

## 8. Cierre de la skill global

La skill global no se modifica durante Fases 0–3. Si una regla resulta generalizable, primero se crea prevención/fixture y se actualiza solo durante Fase 4 con copia, diff, versión/fecha, suite, publicación/fijación cuando aplique y confirmación en una sesión nueva.
