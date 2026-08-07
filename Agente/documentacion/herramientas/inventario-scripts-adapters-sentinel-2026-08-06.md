# Inventario de scripts y adapters frente a Sentinel

> Fecha de corte: 2026-08-07
> Iniciativas canónicas: `Agente/planes/plan-migracion-scripts-adapters-sentinel-2026-08-06.md` y `Agente/planes/plan-preflight-recuperacion-sentinel-2026-08-07.md`

## Decisión

El plano universal debe vivir en Sentinel Core. El consumidor conserva únicamente un adapter pequeño y scripts que encapsulan dominio, proveedor, base de datos, generación o rescate operacional. No se copia `scripts/quality` a otros proyectos y no se retiran wrappers por estética.

## Estado por capa

| Capa | Ubicación | Estado | Decisión |
|---|---|---|---|
| Core universal | upstream Sentinel | SNT-16c/SNT-16d/SNT-16f publicados en la release **0.6.0** (`44dc8fa` en `origin/main` + tag `v0.6.0`) | Adopción estable pendiente solo de la matriz multi-proyecto con clon limpio. |
| Preflight/doctor | upstream Sentinel `src/core/diagnose.ts` | SNT-16f local verificado | Diagnostica submódulo/gitlink, CLI y `--version`, package metadata/dependencias/scripts, capacidades ausentes, symlink escapes, checkout/package-lock dirty, commits/versiones y lock. El gate real falla cerrado antes de las etapas. |
| Recuperación | upstream Sentinel `src/core/taskRecovery.ts` y CLI | SNT-16f local verificado | `task status` deriva expiración/PID/limpieza; `task recover --dry-run/real` exige tarea expirada, PID muerto, namespace, snapshots de metadata/HEAD y worktree limpio; la recuperación real escribe auditoría. |
| Manifest de stages | upstream Sentinel `src/core/` | SNT-16c validado | Envelope schema 1, legacy compatible, paths físicos contenidos y exit no cero fail-closed. |
| Adapter del consumidor | `scripts/quality/adapter-manifest.mjs`, adapters | SNT-15 cerrado | Sigue como frontera local. |
| Gate transitorio | `scripts/quality/task-check.mjs` | Se conserva | No se reemplaza por `sentinel check` hasta release y paridad real. |
| Scripts de dominio | `scripts/run-with-db.mjs`, codegen, preparación DB | Se conservan | Encapsulan Rust/PostgreSQL y no entran al core universal. |
| Analyzers | Sentinel + VarSense | Se conservan separados | VarSense es analyzer, no gate ni reporter paralelo. |

## Evidencia

- Sentinel SNT-16c/SNT-16d/SNT-16f: `tsc` sin errores y suite upstream **502 passing, 1 pending**; release **0.6.0** (`44dc8fa`) publicada en `origin/main` + tag `v0.6.0`.
- Doctor, recovery, capacidades, symlink escape, metadata estricta y contrato CLI focalizados: PASS; el caso de proceso vivo se bloquea y el dry-run de una toma expirada pasa en la evidencia local conservada.
- `node scripts/quality/lock-generator.mjs --write --json` y posteriormente `--check --json`: PASS en el checkout consumidor integrado; `quality-tools.json`, `sentinel.lock.json` y el gitlink coinciden con el commit publicado `44dc8fa00c9ac498e64cad0d6a4edd16afa752d8` (v0.6.0).
- El checkout consumidor fija gitlink/config/lock a `44dc8fa` (release 0.6.0 publicada en `origin/main` + tag `v0.6.0`); lock-check PASS y doctor `ready: true` con cero issues. La release anterior `20c13a2`/`0.5.0` queda como rollback disponible.
- El guard auxiliar esperado por `npm run compile` dentro del submódulo no forma parte de ese checkout; la compilación directa con `tsc` y las suites ejecutadas sí pasan. Esto queda como limitación de provisionamiento, no como PASS del script wrapper.

## Política de permanencia para scripts

Conservar scripts de dominio/proveedor, adapters externos estables, experiencia humana/IDE, bootstrap reproducible o un segundo consumidor real. Migrar solo capacidades universales con más de un caso o claramente agnósticas. No migrar ni copiar scripts históricos o de producción ajena.

## Siguiente bloque

1. ~~Publicar en upstream y crear release/tag compatible~~ → **Hecho:** release **0.6.0** (`44dc8fa`) publicada. Pendiente: matriz multi-proyecto con clon limpio.
2. Validar clon limpio, CLI real, dos proyectos consumidores y paridad envelope/legacy.
3. Actualizar el lock del consumidor primario solo con artefacto/release verificables.
4. Mantener scripts locales hasta dos releases consecutivas verdes; después medir y retirar solo archivos sin referencias.
5. Actualizar la skill global únicamente cuando la release, lock, gate y una sesión nueva aporten evidencia.
