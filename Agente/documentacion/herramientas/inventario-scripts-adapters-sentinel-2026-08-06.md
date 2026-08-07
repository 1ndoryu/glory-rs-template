# Inventario de scripts y adapters frente a Sentinel

> Fecha de corte: 2026-08-07
> Iniciativas canónicas: `Agente/planes/plan-migracion-scripts-adapters-sentinel-2026-08-06.md` y `Agente/planes/plan-preflight-recuperacion-sentinel-2026-08-07.md`

## Decisión

El plano universal debe vivir en Sentinel Core. El consumidor conserva únicamente un adapter pequeño y scripts que encapsulan dominio, proveedor, base de datos, generación o rescate operacional. No se copia `scripts/quality` a otros proyectos y no se retiran wrappers por estética.

## Estado por capa

| Capa | Ubicación | Estado | Decisión |
|---|---|---|---|
| Core universal | upstream Sentinel | SNT-16c preparado en `88e8ac7`, rama remota de trabajo; falta release estable | No fijar `main` ni prometer adopción hasta release/tag y lock reproducible. |
| Preflight/doctor | upstream Sentinel `src/core/diagnose.ts` | SNT-16d inicial | Diagnostica sourcePath, CLI, checkout dirty, gitlink y lock; falta conectarlo al gate y cubrir recovery. |
| Manifest de stages | upstream Sentinel `src/core/` | SNT-16c validado | Envelope schema 1, legacy compatible, paths físicos contenidos y exit no cero fail-closed. |
| Adapter del consumidor | `scripts/quality/adapter-manifest.mjs`, adapters | SNT-15 cerrado | Sigue como frontera local. |
| Gate transitorio | `scripts/quality/task-check.mjs` | Se conserva | No se reemplaza por `sentinel check` hasta release y paridad real. |
| Scripts de dominio | `scripts/run-with-db.mjs`, codegen, preparación DB | Se conservan | Encapsulan Rust/PostgreSQL y no entran al core universal. |
| Analyzers | Sentinel + VarSense | Se conservan separados | VarSense es analyzer, no gate ni reporter paralelo. |

## Evidencia

- Sentinel SNT-16c: compilación TypeScript y suite disponible **497 PASS, 1 pending** en el worktree de tarea.
- Doctor SNT-16d focalizado: detecta source/CLI ausentes y lock divergente; **3 PASS** junto con CLI/task coordinator.
- El consumidor primario sigue en `20c13a2` / `0.5.0`; el worktree de tarea apunta provisionalmente a `88e8ac7` y no se ha fijado el lock primario.
- La instalación de VarSense dejó una modificación accidental de `package-lock.json`; fue restaurada en el worktree de tarea. No quedan cambios en ese submódulo.
- `npm run quality:lock --check` aún no es demostrable en el worktree porque el CLI VarSense no está compilado y el entorno no incluye el guard auxiliar esperado.

## Política de permanencia

Conservar scripts de dominio/proveedor, adapters externos estables, experiencia humana/IDE, bootstrap reproducible o un segundo consumidor real. Migrar solo capacidades universales con más de un caso o claramente agnósticas. No migrar ni copiar scripts históricos o de producción ajena.

## Siguiente bloque

1. Conectar `assertWorkspaceReady` al gate y definir la excepción explícita para proyectos sin `quality-tools` durante transición.
2. Implementar `task recover --dry-run/real` con PID, TTL, namespace y worktree limpio.
3. Publicar release Sentinel SNT-16c/SNT-16d, compilar desde clon limpio y regenerar/verificar locks.
4. Ejecutar fixtures Node/Rust con envelope y legacy mediante CLI real y comparar paridad multi-shell/editor.
5. Solo después evaluar adelgazar `task-check`; no retirar scripts antes de dos releases.
