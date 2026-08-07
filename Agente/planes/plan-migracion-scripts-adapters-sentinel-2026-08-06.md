# Plan — Migración de scripts a Sentinel Core y adapters por proyecto

> **Fecha:** 2026-08-06
> **Estado:** SNT-16c upstream preparado en rama publicada `028A-6/stage-manifest-contract`, con contrato versionado, compatibilidad legacy y hardening físico; adopción estable aún bloqueada por falta de release/tag en `origin/main`, lock reproducible del consumidor y validación desde clon limpio. SNT-16d añade preflight fail-closed y diagnóstico de capacidades; no se retiran scripts ni se actualiza la skill global todavía.
> **Ámbito:** calidad, coordinación de tareas y wrappers de desarrollo; migración reversible y por evidencia
> **Relación:** complementa `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md`, `Agente/planes/plan-sentinel-orquestacion-tareas-worktrees-2026-08-06.md` y `Agente/planes/plan-preflight-recuperacion-sentinel-2026-08-07.md`
> **Fuente canónica de esta iniciativa:** este documento

## 1. Decisión ejecutiva

Sentinel Core/CLI es el único dueño del plano universal. Cada proyecto aporta un manifiesto declarativo y un adapter pequeño para su stack. Los scripts específicos permanecen únicamente cuando encapsulan una operación de dominio real que no puede generalizarse sin inventar una abstracción. Los aliases se mantienen durante una ventana de migración medible y luego se retiran por evidencia, no por limpieza estética.

No se copia `scripts/quality` a otros repositorios ni se elimina mientras no existan paridad, rollback y dos releases. La personalización objetivo es configuración declarativa + pocos adapters versionados, con uno o dos comandos públicos para agentes.

## 2. Situación verificable

- Consumidor primario sigue fijado en Sentinel `20c13a216e879303fcf5be7469a2821391b2ec0d` / `0.5.0`.
- SNT-16c existe como commit recuperable `88e8ac7a4b92ba7a31eb44a85bd87802f47d15c3` y rama remota `028A-6/stage-manifest-contract`, pero no está integrado en `origin/main` ni etiquetado como release.
- La compilación TypeScript y la suite upstream disponible pasan en el worktree de tarea: `497 passing, 1 pending`. El script `npm run test:unit` completo está condicionado por el guard auxiliar externo ausente en ese checkout.
- SNT-16d añade diagnóstico read-only en `src/core/diagnose.ts` de sourcePath, gitlink, checkout dirty, CLI compilado/respondiente y commits configurados/lock; falta conectarlo al gate real y cubrir recovery.
- El worktree de tarea apuntó provisionalmente `quality-tools.json` a `88e8ac7`; el consumidor primario no cambia gitlink ni lock hasta release y hash reproducible.
- No se modifica la skill global ni se eliminan scripts públicos.

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

### Fase 3a — Contrato upstream de stages (`SNT-16`) — implementación preparada, adopción bloqueada
- [x] Diseño documentado: envelope versionado `schemaVersion: 1`, legacy temporal, argv estructurado, report schema y contención física.
- [x] Implementación upstream en commit recuperable `88e8ac7` y rama remota `028A-6/stage-manifest-contract`.
- [x] Compilación TypeScript y suite Sentinel disponible: `497 passing, 1 pending` en el worktree de tarea.
- [x] Fixtures upstream unitarias e integración real del CLI.
- [ ] Integrar en `origin/main`, crear release/tag y validar clon limpio.
- [ ] Actualizar `quality-tools.json` y `sentinel.lock.json` solo tras release, hash y CLI provisionado.

**Rollback:** conservar Sentinel 0.5.0 y `task-check`/adapter local; no retirar duplicaciones.

### Fase 3b — Fixtures multi-proyecto y paridad (`SNT-16b`) — slice local de pruebas preparado
- [x] Añadir dos reportes agnósticos independientes (Node y Rust) con el mismo hallazgo normalizado.
- [x] Comparar decisión, `ruleId`, severidad, archivo, línea y mensaje; distinguir cambios de severidad/mensaje.
- [ ] Ejecutar el mismo envelope y legacy mediante un Sentinel upstream publicado en dos proyectos independientes.
- [ ] Verificar CLI/core/LSP/editor y matriz multi-shell/CI.
- [ ] Fijar commit, capabilities y hash en `quality-tools.json`/`sentinel.lock.json` solo después de release.

**Evidencia SNT-16b:** fixture local dirigida **2/2 PASS**. El upstream añade ejecución real envelope/legacy en sus fixtures, pero aún faltan dos proyectos consumidores independientes y paridad CLI/LSP/editor/multi-shell en CI.

### Fase 3c — Preflight y recuperación (`SNT-16d`) — implementación inicial
- [x] Diagnóstico read-only `sentinel doctor` expone `ready`, códigos accionables y comprobación de sourcePath, gitlink, checkout dirty, CLI compilado/respondiente y commits del lock.
- [ ] Conectar `assertWorkspaceReady` al gate real sin romper fixtures de no-policy ni el modo de transición local.
- [ ] Añadir recuperación explícita de tareas expiradas: `status` diagnostica; `recover` valida PID muerto, estado stale, worktree limpio y namespace antes de cleanup.
- [ ] Añadir fixtures de instalación incompleta, lock divergente, CLI ausente, checkout modificado y reinicio del agente.
- [ ] Validar clon limpio y documentar rollback.

**Gate SNT-16d:** doctor JSON bloquea con evidencia antes de compilar/ejecutar; ningún cleanup automático toca un proceso vivo, worktree sucio o path ajeno.

### Fase 4 — Reducción y retirada controlada (`SNT-17`)
- [ ] Dos releases consecutivos multi-shell/CI.
- [ ] GC/runbook y rollback reproducible.
- [ ] Retirar físicamente solo archivos sin referencias y con rollback documentado.
- [ ] Actualizar la skill global al final, después de release, locks, gate y una sesión nueva.

## 5. Política de permanencia para scripts

Conservar scripts de dominio/proveedor, adapters externos estables, experiencia humana/IDE, bootstrap reproducible o un segundo consumidor real. Migrar solo capacidades universales con más de un caso o claramente agnósticas. No migrar ni copiar scripts históricos o de producción ajena.

## 6. Seguridad y no-sorpresas

No shell concatenado en manifests; paths contenidos y sin symlink/junction escapes; errores de herramienta fail-closed; no borrar `scripts/quality`; no desplegar ni ejecutar rescates/producción.

## 7. Definition of Done global

- [ ] owner real del inventario;
- [x] schema y fixtures publicados en Sentinel rama de trabajo; release estable pendiente;
- [ ] dos proyectos usan el mismo core sin copiar `scripts/quality`;
- [ ] VarSense solo analyzer;
- [ ] adapter wandorius delgado;
- [ ] cinco gates con paridad exacta y errores fail-closed;
- [ ] dos releases consecutivos multi-shell/CI;
- [ ] rollback y cleanup/GC probados;
- [x] documentación local refleja SNT-12–SNT-16d sin falsear adopción;
- [ ] skill global actualizada solo al final, después de release, locks, gate y sesión nueva;
- [ ] cero secretos/procesos/locks/worktrees/ramas propias pendientes.

## 8. Cierre de la skill global

La skill global no se modifica antes de que Sentinel publique el contrato en una release, el consumidor lo fije y una sesión nueva confirme fixtures y gate. La política candidata es manifest versionado, paths físicos contenidos, errores fail-closed, preflight de capacidades y recuperación segura; por ahora queda documentada, no propagada.
