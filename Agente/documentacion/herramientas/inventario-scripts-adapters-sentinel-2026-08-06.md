# Inventario — scripts, adapters y Sentinel

> **Fecha de corte:** 2026-08-06  
> **Tarea:** SNT-12  
> **Workspace inventariado:** worktree Sentinel `task/157fb8a2b2a4e1dc/SNT-12`  
> **Estado:** Fase 0 — baseline e inventario  
> **Fuente canónica:** `Agente/planes/plan-migracion-scripts-adapters-sentinel-2026-08-06.md`

## 1. Alcance y evidencia

El inventario se construyó desde archivos versionados, no desde el contenido accidental de una máquina:

```text
git ls-files scripts
```

Resultado de la línea base:

- 122 archivos bajo `scripts/`.
- 8 scripts de raíz/compatibilidad.
- 70 scripts, wrappers, parches y utilidades directamente bajo `scripts/quality/`.
- 44 tests bajo `scripts/quality/tests/`.
- 9 adapters bajo `scripts/quality/adapters/` (incluidos en los 70 anteriores).
- Runtime Sentinel instalado y verificado: `0.5.0`, commit `20c13a2`, hash de artefacto verificado.
- VarSense fijado en el submódulo `tools/varsense`, commit `e836092`.
- Rama primaria del consumidor: `wandorius`.
- El commit base de esta tarea es `58761d06`; el checkout principal no se modifica desde el `start` del worktree.

**Límite de la evidencia:** el grep de seguridad sobre los scripts versionados no encontró tokens literales,
URLs Bearer ni credenciales en el estado actual. Esto no sustituye un escáner de secretos ni demuestra que no
haya secretos históricos en commits, artefactos ignorados o variables de entorno. Ningún secreto se copia a
este inventario.

## 2. Consumers directos

| Consumidor | Referencia | Destino de migración | Propietario provisional |
|---|---|---|---|
| Root `package.json` | `dev`, `self-check`, `quality:*`, `task:*`, `check:*`, `test`, `codegen`, `roadmap` | alias/adapters; no copiar el core | tooling del consumidor — por asignar |
| `frontend/package.json` | `test`, `test:changed` → `../scripts/quality/run-frontend-tests.mjs` | adapter Vite/Vitest | frontend tooling — por asignar |
| `.github/workflows/quality.yml` | setup, quality tests, DB CI, codegen, `task:check`, export de métricas | integración CI de Sentinel + adapter del consumidor | CI/tooling — por asignar |
| `README.md` / `AGENTS.md` | camino de agente y comandos públicos | documentación corta + enlace al contrato global | documentación del consumidor — por asignar |
| `sentinel.config.json` | política v2, guard, rama, analyzers | configuración declarativa única | maintainer del consumidor — por asignar |
| `quality.config.json` | perfiles, timeouts, cachés, retención, presupuestos | transición; extraer lo universal a Sentinel | maintainer del consumidor — por asignar |
| `quality-tools.json` / `sentinel.lock.json` | versiones, commits, capacidades y hashes | lock del consumidor; conservar mientras exista adapter local | release/tooling — por asignar |

## 3. Matriz de scripts de raíz

| Archivo | Uso observado | Destino | Riesgo / criterio | Owner provisional |
|---|---|---|---|---|
| `scripts/branch-db.mjs` | deriva BD y `CARGO_TARGET_DIR` por rama; crea BD local | **adapter específico Rust/PostgreSQL** | no generalizar a Sentinel; validar SQL identifier y secretos solo por env | backend tooling — por asignar |
| `scripts/check-roadmap.mjs` | watcher/problem matcher de roadmap | **específico del consumidor** | reglas de `roadmap.md`/`Agente`; no pertenece al core | documentación — por asignar |
| `scripts/codegen-local.mjs` | export OpenAPI + Orval sin servidor | **adapter específico Rust/OpenAPI** | debe seguir usando argv separado; no es gate universal | API tooling — por asignar |
| `scripts/dev.mjs` | launcher compatible hacia `glory-rs/scripts/dev.mjs` | **alias/launcher compartido** | no duplicar coordinación; conservar hasta un runtime launcher estable | runtime de desarrollo — por asignar |
| `scripts/emit-openapi.ps1` | wrapper Windows de export OpenAPI y limpieza de target | **específico de stack/Windows; revisar** | referencia a `clean-cargo-target.ps1` no presente en el inventario actual; debe corregirse o retirarse antes de recomendarlo | API tooling — por asignar |
| `scripts/prepare-ci-db.mjs` | aplica migraciones a BD CI | **adapter Rust/PostgreSQL de CI** | no mover a Sentinel Core; no imprimir credenciales | CI/backend — por asignar |
| `scripts/run-with-db.mjs` | ejecuta Cargo con DB/target por rama, lease pesado y markers | **dividir**: DB/target queda adapter; lease/guard/banner migra a Sentinel | actualmente mezcla responsabilidades; migrar solo tras contrato de runner/lease | backend + tooling — por asignar |
| `scripts/self-check.ps1` | alias PowerShell a `npm run task:check` | **alias temporal** | no duplica suite; retirar tras dos releases de paridad y guía actualizada | tooling — por asignar |

## 4. Matriz de `scripts/quality/`

### 4.1 Candidatos universales para Sentinel Core

La decisión es migrar capacidades, no copiar archivos. Cada fila requiere fixture upstream, contrato versionado,
rollback y paridad antes de retirar el wrapper local.

| Archivo | Capacidad | Destino | Owner provisional |
|---|---|---|---|
| `atomic-file.mjs` | escritura atómica | Sentinel Core / utilidad de runtime | Sentinel upstream — por asignar |
| `branch-identity.mjs` | identidad segura de rama/CI | Sentinel Core | Sentinel upstream — por asignar |
| `cache.mjs` | caché por scope/política | Sentinel Core | Sentinel upstream — por asignar |
| `heavy-run-guard.mjs` | cooldown, lease y override auditado | Sentinel Core | Sentinel upstream — por asignar |
| `lock.mjs` | lock de ejecución del gate | Sentinel Core | Sentinel upstream — por asignar |
| `redaction.mjs` | redacción de secretos | Sentinel Core | Sentinel upstream — por asignar |
| `report-retention.mjs` | TTL/cuota/poda segura de reportes | Sentinel Core | Sentinel upstream — por asignar |
| `reporter.mjs` | contrato JSON, Markdown y exit code | Sentinel Core + template declarativo | Sentinel upstream — por asignar |
| `runner.mjs` | procesos, timeout, cancelación y env | Sentinel Core | Sentinel upstream — por asignar |
| `scope.mjs` | scope-manifest, hashes y dependencias | Sentinel Core | Sentinel upstream — por asignar |
| `stage-runner.mjs` | scheduler de stages y drenaje | Sentinel Core | Sentinel upstream — por asignar |
| `target-maintenance.mjs` | mantenimiento de artefactos pesados | Sentinel Core mediante provider | Sentinel upstream/consumer — por asignar |
| `target-maintenance-stage.mjs` | integración best-effort de mantenimiento | Sentinel Core | Sentinel upstream — por asignar |
| `stage-process.mjs` | transporte estructurado de una etapa | Sentinel Core / adapter protocol | Sentinel upstream — por asignar |
| `structured-tool` (ver adapters) | validación de salida de analyzer | Sentinel Core | Sentinel upstream — por asignar |

### 4.2 Configuración y compatibilidad del consumidor

| Archivo | Capacidad | Destino | Criterio |
|---|---|---|---|
| `args.mjs` | CLI de `task:check` | alias del consumidor | conservar mientras exista `npm run task:check`; no crear otro parser global |
| `policy.mjs` | validación/discovery de `sentinel.config.json` | Sentinel Core parcialmente; config queda en consumidor | la política universal debe ser del runtime; las reglas del proyecto quedan en JSON |
| `policy-defaults.mjs` | defaults de comandos del guard | Sentinel Core / configuración v2 | separar catálogo universal de comandos del proyecto |
| `policy-decision.mjs` | estados `observe/enforce/pass-through` | Sentinel Core | mantener wrapper solo durante transición |
| `profile-contract.mjs` | perfiles → stages del consumidor | adapter de proyecto | Sentinel debe consumir perfiles declarativos, no rutas `frontend/src` |
| `preflight.mjs` | comprobar lock/tools/config del consumidor | dividir | lock/runtime a Sentinel; source paths y stack al adapter |
| `lockfile.mjs` | validar submódulos/tools/lock | Sentinel Core + lock adapter | no permitir rutas absolutas ni checkout sucio |
| `lock-generator.mjs` | generar lock del consumidor | maintainer adapter temporal | reemplazar por `sentinel lock` cuando exista contrato estable |
| `setup.mjs` | inicializar submódulos y compilar analyzers | bootstrap de consumidor | parte de instalación global migra a Sentinel; submódulos siguen siendo consumer setup |
| `sentinel-doctor.mjs` | diagnóstico/migración local | alias de `sentinel doctor` | retirar cuando el CLI global cubra el mismo JSON |
| `quality-command-guard.mjs` | guard local y fallback legacy | Sentinel Guard | retirar solo tras matriz multi-shell y dos releases |
| `source-path.mjs` | resolver source paths de analyzers | adapter/lock del consumidor | no permitir paths fuera del workspace |

### 4.3 Adapters de stack y reglas del consumidor

| Archivo | Capacidad | Destino | Owner provisional |
|---|---|---|---|
| `adapters/common.mjs` | normalización de findings y logs | adapter compartido temporal; normalización base a Sentinel | consumer tooling — por asignar |
| `adapters/custom.mjs` | reglas de arquitectura wandori.us | adapter específico | producto/arquitectura — por asignar |
| `adapters/docs.mjs` | roadmap, planes y docs en español | adapter específico | documentación — por asignar |
| `adapters/frontend.mjs` | type-check, test/build/budgets Vite | adapter específico Node/Vite | frontend tooling — por asignar |
| `adapters/rust.mjs` | fmt/check/clippy/test y DB | adapter específico Rust/PostgreSQL | backend tooling — por asignar |
| `adapters/sentinel.mjs` | invocación Sentinel analyzer | adapter de analyzer | Sentinel/consumer — por asignar |
| `adapters/structured-tool.mjs` | contrato común de invocación JSON | Sentinel Core candidate | Sentinel upstream — por asignar |
| `adapters/varsense-contract.mjs` | argv/capabilities de VarSense | adapter VarSense | VarSense/consumer — por asignar |
| `adapters/varsense.mjs` | ejecutar VarSense dentro del gate | adapter VarSense; sin gate propio | VarSense/consumer — por asignar |
| `custom-rules.mjs` | reglas custom locales | adapter específico | producto/arquitectura — por asignar |
| `frontend-test-selection.mjs` | selección de tests por imports | adapter Vitest | frontend tooling — por asignar |
| `performance-budget.mjs` | límites gzip/build | adapter específico frontend | frontend tooling — por asignar |
| `run-frontend-tests.mjs` | wrapper incremental Vitest | adapter específico frontend | frontend tooling — por asignar |
| `stage-definitions.mjs` | selección de stages del consumidor | adapter declarativo temporal | consumer tooling — por asignar |
| `stages.mjs` | genera contrato `--stages` para observe | compatibilidad de migración | retirar cuando `sentinel check` consuma manifest nativo |
| `varsense-parity.mjs` | compara gate vs CLI VarSense | diagnóstico/transición | retirar después de paridad sostenida |

### 4.4 Mantenimiento, diagnóstico, CI y transición

| Archivo | Capacidad | Destino | Criterio |
|---|---|---|---|
| `bench-baseline.mjs` | benchmark del gate | maintainer/CI | no enseñar al agente normal; migrar solo si Sentinel ofrece profile |
| `bench-fixtures.mjs` | fixtures de benchmark | tests/CI | conservar hasta reemplazo de Sentinel benchmark |
| `export-ci-metrics.mjs` | export de métricas CI | CI adapter | mantener sin código fuente ni secretos |
| `index-maintenance.mjs` | TTL/cuota de índices VarSense | adapter/etapa best-effort | generalizar solo con provider de índices |
| `observe-compare.mjs` | doble vía old/new | transición | retirar tras dos releases y paridad |
| `quality-profile.mjs` | p50/p95 de reportes | diagnóstico/alias `sentinel profile` | no gate ni camino normal |
| `report-cleanup.mjs` | limpieza explícita de reportes | Sentinel Core CLI | mantener wrapper destructivo con confirmación durante transición |
| `report-reader.mjs` | leer namespace canónico/legacy | Sentinel Core CLI | retirar lector legacy tras ventana declarada |
| `report-retention-stage.mjs` | integración de retención | Sentinel Core | no cambiar decisión del gate |
| `reminders.mjs` | mensajes contextuales en español | template del consumidor | core solo debe entregar estados/remediation estructurados |
| `install-global-runtime.mjs` | bridge de instalación global | alias temporal | sustituir por `sentinel install/update/uninstall` |
| `install-global-guard.ps1` | wrapper de instalación PowerShell | legacy/deprecate | no duplicar instalador global |
| `run-p0-checks.ps1` | checks manuales de P0 | maintainer/manual | conservar solo si tiene uso documentado |
| `run-p0-checks.sh` | checks manuales de P0 | maintainer/manual | conservar solo si tiene uso documentado |
| `check-sentinel-extended.sh` | checks extendidos históricos | legacy/manual | revisar referencias; no es camino normal |
| `check-dom-abstraction.sh` | regla/check frontend local | adapter custom | migrar a Sentinel solo con segundo consumidor |
| `check-singleton-state.sh` | regla/check frontend local | adapter custom | migrar a Sentinel solo con segundo consumidor |
| `check-window-refs.sh` | regla/check frontend local | adapter custom | migrar a Sentinel solo con segundo consumidor |

### 4.5 Shims y wrappers duplicados

| Archivo | Destino | Retirada |
|---|---|---|
| `cargo.cmd` | runtime Sentinel; mantener shim de compatibilidad | dos releases con matriz verde |
| `node.cmd` | runtime Sentinel | dos releases con matriz verde |
| `npm.cmd` | runtime Sentinel | dos releases con matriz verde |
| `npx.cmd` | runtime Sentinel | dos releases con matriz verde |
| `global-cargo-guard.ps1` | runtime Sentinel Guard | retirar tras PowerShell 5/7 |
| `global-quality-guard.sh` | runtime Sentinel Guard | retirar tras Bash/Git Bash |

## 5. Tests del adapter y del orquestador

Los siguientes archivos no son runtime reutilizable; son evidencia de contrato. Se conservan hasta que la
capacidad correspondiente tenga fixture equivalente en Sentinel. Owner provisional: consumer tooling.

| Tests exactos | Destino |
|---|---|
| `tests/args.test.mjs`, `tests/atomic-file.test.mjs`, `tests/runner.test.mjs`, `tests/stage-runner.test.mjs`, `tests/structured-tool.test.mjs` | mover fixtures/contratos al core; mantener regresión local durante transición |
| `tests/branch-identity.test.mjs`, `tests/branch-isolation.integration.test.mjs`, `tests/cache.test.mjs`, `tests/lock.test.mjs`, `tests/lockfile.test.mjs`, `tests/report-retention.test.mjs`, `tests/report-retention-stage.test.mjs`, `tests/report-reader.test.mjs` | Sentinel Core; conservar integración del consumidor hasta paridad |
| `tests/heavy-run-guard.test.mjs`, `tests/target-maintenance.test.mjs`, `tests/index-maintenance.test.mjs` | scheduler/provider de artefactos; no retirar sin fixtures multi-proyecto |
| `tests/policy.test.mjs`, `tests/policy-decision.test.mjs`, `tests/policy-identity.test.mjs`, `tests/preflight.test.mjs`, `tests/source-path.test.mjs`, `tests/profile-contract.test.mjs` | contrato de política/adapter; parte upstream, parte local |
| `tests/quality-command-guard.test.mjs`, `tests/task-takeover.test.mjs` | Sentinel Guard/Task Coordinator; deben desaparecer como segunda autoridad |
| `tests/reporter.test.mjs`, `tests/redaction.test.mjs`, `tests/reminders.test.mjs` | reporter core + template español |
| `tests/docs-adapter.test.mjs`, `tests/custom-rules.test.mjs`, `tests/frontend-test-selection.test.mjs`, `tests/rust-adapter.test.mjs`, `tests/varsense-contract.test.mjs`, `tests/varsense-parity.test.mjs`, `tests/performance-budget.test.mjs` | adapters específicos del consumidor |
| `tests/bench-baseline.test.mjs`, `tests/bench-fixtures.test.mjs`, `tests/export-ci-metrics.test.mjs`, `tests/observe-integration.test.mjs`, `tests/quality-profile.test.mjs`, `tests/report-cleanup.test.mjs` | diagnóstico/transición/CI |
| `tests/doctor-migration.test.mjs`, `tests/lock-generator.test.mjs` | bootstrap/lock del consumidor hasta `sentinel doctor/lock` |

## 6. Hallazgos de seguridad y mantenimiento

1. **No se encontraron secretos literales en los scripts versionados actuales** mediante búsqueda focalizada.
   La evidencia es negativa y limitada; no autoriza a declarar una auditoría de secretos completa.
2. `scripts/prepare-ci-db.mjs` pasa `PGPASSWORD` únicamente por `env` y no lo imprime; mantener esa frontera.
3. `scripts/quality` maneja tokens efímeros de coordinación/leases; deben permanecer redactados y no entrar
   al reporte.
4. `scripts/emit-openapi.ps1` referencia `clean-cargo-target.ps1`, que no está presente en los scripts
   versionados actuales. Antes de recomendar ese wrapper se debe crear una tarea de corrección o retirarlo.
5. Los wrappers `task-check`, `task-takeover`, `run-with-db` y los shims todavía mezclan responsabilidades;
   no se deben eliminar por conteo de archivos.
6. No se encontraron archivos operativos antiguos con secretos en el checkout actual; cualquier recuperación
   desde commits históricos requiere autorización y rotación previa.

## 7. Referencias y dependencias críticas

```text
package.json
 ├─ task:check → scripts/quality/task-check.mjs
 ├─ task:take/release/status → scripts/quality/task-takeover.mjs
 ├─ check:back/test/dev:back → scripts/run-with-db.mjs → scripts/branch-db.mjs
 ├─ quality:* → scripts/quality/*
 └─ dev → glory-rs/scripts/dev.mjs (submódulo)

frontend/package.json
 └─ test/test:changed → scripts/quality/run-frontend-tests.mjs

.github/workflows/quality.yml
 ├─ quality:setup/test/task:check
 ├─ prepare-ci-db.mjs
 ├─ codegen-local.mjs
 └─ export-ci-metrics.mjs + branch-identity.mjs

scripts/quality/task-check.mjs
 ├─ preflight/policy/scope/cache/lock/runner/stages
 ├─ task-takeover/heavy-run/retention/target-maintenance
 └─ reporter + adapters/*
```

## 8. Criterio de cierre de este inventario

- [x] Todos los archivos versionados bajo `scripts/` tienen una clasificación o grupo explícito.
- [x] Los consumers directos de `package.json`, frontend y CI están identificados.
- [x] Se separaron capacidades candidatas a Core, adapters de proyecto, diagnóstico y wrappers legacy.
- [x] Se registraron owners provisionales sin inventar propietarios reales.
- [x] Se registraron riesgos de seguridad y una limitación concreta (`emit-openapi.ps1`).
- [ ] Asignar owners reales y frecuencia de uso mediante revisión del equipo.
- [ ] Confirmar con Sentinel upstream el contrato final del adapter antes de migrar capacidades.

**Conclusión:** Fase 0 queda lista como baseline técnico, pero no autoriza todavía la retirada física de
`scripts/quality`, la modificación de la skill global ni cambios upstream sin tarea separada.
