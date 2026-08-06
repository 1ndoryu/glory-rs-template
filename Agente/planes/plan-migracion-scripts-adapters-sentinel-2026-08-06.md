# Plan — Migración de scripts a Sentinel Core y adapters por proyecto

> **Fecha:** 2026-08-06
> **Estado:** SNT-16b local preparado con fixtures agnósticos, pero adopción upstream bloqueada: el commit de Sentinel no está publicado ni recuperable en el submódulo actual; no se retiran scripts ni se actualiza la skill global
> **Ámbito:** calidad, coordinación de tareas y wrappers de desarrollo; migración reversible y por evidencia
> **Relación:** complementa `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md` y `Agente/planes/plan-sentinel-orquestacion-tareas-worktrees-2026-08-06.md`
> **Fuente canónica de esta iniciativa:** este documento

## 1. Decisión ejecutiva

Sentinel Core/CLI es el único dueño del plano universal. Cada proyecto aporta un manifiesto declarativo y un adapter pequeño para su stack. Los scripts específicos permanecen únicamente cuando encapsulan una operación de dominio real que no puede generalizarse sin inventar una abstracción. Los aliases se mantienen durante una ventana de migración medible y luego se retiran por evidencia, no por limpieza estética.

No se copia `scripts/quality` a otros repositorios ni se elimina mientras no existan paridad, rollback y dos releases. La personalización objetivo es configuración declarativa + pocos adapters versionados, con uno o dos comandos públicos para agentes.

## 2. Situación verificable

- Consumidor fijado en Sentinel `20c13a216e879303fcf5be7469a2821391b2ec0d` / `0.5.0`.
- El submódulo de la tarea se restauró limpio a ese gitlink; el parche upstream explorado no quedó en el checkout.
- `ef9c751` no existe en los objetos/referencias locales consultados y no se recupera sin fuente upstream autorizada.
- No se modifican `quality-tools.json`, `sentinel.lock.json`, `conducta-global`, ni scripts públicos por esta limitación.

## 3. Modelo objetivo

| Capa | Vive en | Dueño | Contiene | No contiene |
|---|---|---|---|---|
| Sentinel Core | upstream | tooling | lifecycle, scheduler, scope, cache, leases, report schema, redaction, runtime/shims, GC | rutas, dominio, reglas visuales |
| Project adapter | raíz del consumidor | proyecto | manifest de comandos/stages, checks de stack, VarSense y custom | scheduler, cooldown, claims, worktrees, reporte final |
| Scripts específicos | `scripts/` | producto/operaciones | DB/Cargo, codegen, rescates autorizados, fixtures | coordinación universal |

## 4. Fases ejecutables

### Fase 0 — Baseline, ownership y seguridad (`SNT-12`) — cerrada
Inventario versionado, baseline de Sentinel/VarSense y separación de cambios ajenos. No se movieron ni eliminaron scripts.

### Fase 1 — Contrato mínimo de adapter (`SNT-13`) — transición local cerrada
`quality-adapter.json` define contrato estricto, transporte argv, stages, profiles, timeouts, allowlist, output schema y exit-code mapping.

### Fase 2 — Core/adapter slice (`SNT-14`) — cerrada en transición local
El manifest local gobierna stages; `task-check` sigue como orquestador transitorio y el camino legacy está probado como compatibilidad.

### Fase 2b — Endurecimiento (`SNT-15`) — cerrada localmente
Schema estricto, selección/paridad desde disco, contención física y rechazo de allowlists sensibles. Evidencia: 19/19 focalizadas, `node --check` y `git diff --check` PASS.

### Fase 3a — Contrato upstream de stages (`SNT-16`) — bloqueada para adopción
- [x] Diseño documentado: envelope versionado `schemaVersion: 1`, legacy temporal, argv estructurado, report schema y contención física.
- [ ] Implementación upstream en un commit recuperable y publicado.
- [ ] Compilación y suite upstream desde checkout limpio.
- [ ] Fixtures upstream unitarias e integración real del CLI.
- [ ] Release/tag, `quality-tools.json` y `sentinel.lock.json` actualizados únicamente tras evidencia.

**Rollback:** conservar Sentinel 0.5.0 y `task-check`/adapter local; no retirar duplicaciones.

### Fase 3b — Fixtures multi-proyecto y paridad (`SNT-16b`) — slice local de pruebas preparado
- [x] Añadir dos reportes agnósticos independientes (Node y Rust) con el mismo hallazgo normalizado.
- [x] Comparar decisión, `ruleId`, severidad, archivo, línea y mensaje; distinguir cambios de severidad/mensaje.
- [ ] Ejecutar el mismo envelope y legacy mediante un Sentinel upstream publicado.
- [ ] Verificar CLI/core/LSP/editor y matriz multi-shell en CI.
- [ ] Fijar commit, capabilities y hash en `quality-tools.json`/`sentinel.lock.json` solo después de release.

**Evidencia SNT-16b:** fixture local dirigida **2/2 PASS**. Esto valida solamente la normalización de contrato; no demuestra ejecución de Sentinel ni paridad CLI/LSP/editor.

### Fase 4 — Reducción y retirada controlada (`SNT-17`)
- [ ] Dos releases consecutivos multi-shell/CI.
- [ ] GC/runbook y rollback reproducible.
- [ ] Retirar físicamente solo archivos sin referencias y con rollback documentado.
- [ ] Actualizar la skill global al final, con copia, diff, versión/fecha, suite, publicación/fijación y confirmación en sesión nueva.

## 5. Política de permanencia para scripts

Conservar scripts de dominio/proveedor, adapters externos estables, experiencia humana/IDE, bootstrap reproducible o un segundo consumidor real. Migrar solo capacidades universales con más de un caso o claramente agnósticas. No migrar ni copiar scripts históricos o de producción ajena.

## 6. Seguridad y no-sorpresas

No shell concatenado en manifests; paths contenidos y sin symlink/junction escapes; errores de herramienta fail-closed; no borrar `scripts/quality`; no desplegar ni ejecutar rescates/producción.

## 7. Definition of Done global

- [ ] owner real del inventario;
- [ ] schema y fixtures publicados en Sentinel;
- [ ] dos proyectos usan el mismo core sin copiar `scripts/quality`;
- [ ] VarSense solo analyzer;
- [ ] adapter wandorius delgado;
- [ ] cinco gates con paridad exacta y errores fail-closed;
- [ ] dos releases consecutivos multi-shell/CI;
- [ ] rollback y cleanup/GC probados;
- [x] documentación local refleja SNT-12–SNT-16b sin falsear adopción;
- [ ] skill global actualizada solo al final;
- [ ] cero secretos/procesos/locks/worktrees/ramas propias pendientes.

## 8. Cierre de la skill global

La skill global no se modifica antes de que Sentinel publique el contrato, el consumidor lo fije y una sesión nueva confirme fixtures y gate. La política candidata es manifest versionado, paths físicos contenidos y errores fail-closed; por ahora queda documentada, no propagada.
