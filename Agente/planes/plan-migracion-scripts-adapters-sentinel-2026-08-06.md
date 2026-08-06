# Plan — Migración de scripts a Sentinel Core y adapters por proyecto

> **Fecha:** 2026-08-06
> **Estado:** SNT-16 upstream implementado localmente sobre Sentinel 0.5.0, pendiente commit/publicación/fijación y fixtures multi-proyecto; no se retiran scripts ni se actualiza la skill global
> **Ámbito:** calidad, coordinación de tareas y wrappers de desarrollo; migración reversible y por evidencia
> **Relación:** complementa `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md` y `Agente/planes/plan-sentinel-orquestacion-tareas-worktrees-2026-08-06.md`
> **Fuente canónica de esta iniciativa:** este documento

## 1. Decisión ejecutiva

El problema no es simplemente que haya muchos archivos. El checkout mezcla cuatro responsabilidades con vidas útiles distintas: plano universal, adaptación por proyecto, compatibilidad temporal y operación específica/histórica.

> **Sentinel Core/CLI es el único dueño del plano universal. Cada proyecto aporta un manifiesto declarativo y un adapter pequeño para su stack. Los scripts específicos permanecen únicamente cuando encapsulan una operación de dominio real que no puede generalizarse sin inventar una abstracción. Los aliases se mantienen durante una ventana de migración medible y luego se retiran por evidencia, no por limpieza estética.**

No se copia `scripts/quality` a otros repositorios ni se elimina mientras no existan paridad, rollback y dos releases. La personalización objetivo es configuración declarativa + pocos adapters versionados, con uno o dos comandos públicos para agentes.

## 2. Evidencia de la situación actual

Sentinel `0.5.0` (`20c13a216e879303fcf5be7469a2821391b2ec0d`) expone `check --stages`, contratos de etapas y coordinación de tareas. El consumidor conserva `scripts/quality` como implementación probada y frontera de transición; SNT-16 añade el contrato upstream local, pero aún no es una release adoptable.

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

`quality-adapter.json` define versión/protocolo, capabilities, transporte argv, stages, profiles, timeouts, environment allowlist, output schema y exit-code mapping. `adapter-manifest.mjs` valida strict keys, placeholders, task IDs, paths dentro de workspace/report root, symlink/junction ancestors y entrypoint regular. La salida de `stage-process` es JSON schema v1.

### Fase 2 — Core/adapter slice (`SNT-14`) — cerrada en transición local

El manifest local es dueño de nombres/perfiles de stages; `task-check` sigue siendo el orquestador de transición y el camino legacy está explícito y probado solo como compatibilidad temporal. Se mantienen allowlists no sensibles, argv y shell false, y no se declaran secretos por manifest.

### Fase 2b — Endurecimiento del slice (`SNT-15`) — cerrado localmente

Se añadieron schema estricto del adapter, selección/paridad real desde disco, protección física contra symlink/junction y rechazo de allowlists sensibles. Suite focalizada **18/18 PASS**, `node --check` y `git diff --check` PASS. Gate completo limitado por entorno ausente (BD/.env, frontend dependencies, VarSense CLI compilado).

### Fase 3a — Contrato upstream de stages (`SNT-16`) — implementación local pendiente publicación

- [x] Añadir `tools/sentinel/src/core/stageManifest.ts` con envelope `schemaVersion: 1` y compatibilidad explícita con lista legacy.
- [x] Reutilizar el tipo `StructuredToolDefinition` para evitar divergencia de contrato.
- [x] Validar claves estrictas, nombres únicos, timeout máximo, argv strings y report schema esperado.
- [x] Resolver `--stages`/`cwd` contra workspace y `reportPath` contra reportRoot.
- [x] Rechazar traversal, symlink/junction escape en manifest, reportRoot, reportes y cwd; permitir reportRoot aún no creado mediante ancestor existente.
- [x] Integrar el loader en el camino real `runCheck`, antes de ejecutar etapas.
- [x] Añadir fixtures unitarias e integración real de envelope, legacy, paths externos y symlink.
- [x] Documentar compatibilidad y rollback en el inventario del consumidor.
- [ ] Compilar y ejecutar suite upstream desde un checkout limpio con dependencias provisionadas.
- [ ] Revisar y committear el submódulo upstream en una rama propia; publicar el commit/tag antes de consumirlo.

**Evidencia local SNT-16:** `git diff --check` PASS. El gate directo impidió `npx tsc` por el guard (exit 78), y el worktree no tiene `node_modules` ni el helper `quality-command-guard.mjs`; por tanto no se declara compile/suite PASS. La modificación del submódulo está aislada en el worktree SNT-16 y el consumidor continúa fijando `20c13a2`.

### Fase 3b — Fixtures multi-proyecto y paridad (`SNT-16b`) — bloqueada por publicación

- [ ] Crear dos fixtures de proyecto sin rutas `wandorius` ni reglas de dominio.
- [ ] Ejecutar `sentinel check` con envelope y lista legacy; comparar decisión, estado, severidad, ruleId, file, line y message.
- [ ] Verificar CLI/core/LSP donde aplique y multi-shell en CI.
- [ ] Fijar commit, capacidades y hash en `quality-tools.json`/`sentinel.lock.json` solo después de release.

### Fase 4 — Reducción y retirada controlada (`SNT-17`)

- [ ] Dos releases consecutivos multi-shell/CI.
- [ ] GC/runbook y rollback reproducible.
- [ ] Retirar físicamente solo archivos sin referencias y con rollback documentado.
- [ ] Actualizar la skill global únicamente al final, con copia, diff, versión/fecha, suite, publicación/fijación y confirmación en sesión nueva.

## 5. Política de permanencia para scripts

Conservar scripts que encapsulen dominio/proveedor, adapter externo estable, experiencia humana/IDE, bootstrap reproducible o un segundo consumidor real. Migrar solo capacidades universales que aparezcan en más de un caso o sean claramente agnósticas. Archivar scripts históricos, de incidentes o de producción ajena; no migrarlos ni copiarlos.

## 6. Seguridad y no-sorpresas

No shell concatenado en manifests; paths contenidos y sin symlink/junction escapes; errores de herramienta fail-closed; no borrar `scripts/quality`; no desplegar ni ejecutar rescates/producción. El allowlist base no expone credenciales y el schema local rechaza variables sensibles; un futuro transporte de secretos requerirá un contrato upstream explícito, capacidad auditable y revisión separada.

## 7. Definition of Done global

- [ ] inventario con owner real;
- [ ] schema y fixtures publicados en Sentinel;
- [ ] dos proyectos usan el mismo core sin copiar `scripts/quality`;
- [ ] VarSense solo analyzer;
- [ ] adapter wandorius delgado;
- [ ] cinco gates con paridad exacta y errores fail-closed;
- [ ] dos releases consecutivos multi-shell/CI;
- [ ] rollback y cleanup/GC probados;
- [x] documentación local refleja el estado real de SNT-12–SNT-16;
- [ ] skill global actualizada solo al final, si la evidencia lo justifica;
- [ ] cero secretos/procesos/locks/worktrees/ramas propias pendientes.

## 8. Cierre de la skill global

La skill global no se modifica durante Fases 0–3. La política generalizable identificada es “manifest versionado, paths físicos contenidos y fallos fail-closed”, pero no se propaga a la skill hasta que Sentinel publique el contrato, el consumidor lo fije y una sesión nueva confirme fixtures y gate.
