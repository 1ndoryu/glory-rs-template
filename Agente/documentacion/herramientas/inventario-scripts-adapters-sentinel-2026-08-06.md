# Inventario de scripts y adapters frente a Sentinel

> Fecha de corte: 2026-08-06
> Iniciativa canónica: `Agente/planes/plan-migracion-scripts-adapters-sentinel-2026-08-06.md`

## Decisión

El plano universal debe vivir en Sentinel Core. El consumidor conserva únicamente un adapter pequeño y scripts que encapsulan dominio, proveedor, base de datos, generación o rescate operacional. No se copia `scripts/quality` a otros proyectos y no se retiran wrappers por estética.

## Estado por capa

| Capa | Ubicación | Estado | Decisión |
|---|---|---|---|
| Core universal | `tools/sentinel/src/core/` | SNT-16 en implementación upstream | Scheduler, scope, cache, leases, reportes, runner y contrato declarativo de stages migran aquí. |
| Manifest de stages | `tools/sentinel/src/core/stageManifest.ts` | SNT-16 añadido localmente, pendiente release | Envelope `schemaVersion: 1`, lista legacy compatible, schema estricto, argv y contención física. |
| Adapter del consumidor | `scripts/quality/adapter-manifest.mjs`, adapters | SNT-15 cerrado | Sigue siendo frontera local hasta release upstream y paridad multi-proyecto. |
| Gate transitorio | `scripts/quality/task-check.mjs` | Se conserva | No se reemplaza por `sentinel check` hasta dos releases y observe real. |
| Scripts de dominio | `scripts/run-with-db.mjs`, codegen, preparación DB | Se conservan | Encapsulan stack Rust/PostgreSQL y no deben entrar al core universal. |
| Analyzers | Sentinel + VarSense | Se conservan separados | VarSense valida CSS/tokens/clases; no crea gate ni reporter paralelo. |

## Contrato upstream SNT-16

`sentinel check --stages` acepta:

```json
{
  "schemaVersion": 1,
  "stages": [
    {
      "name": "frontend",
      "executable": "node",
      "args": ["scripts/check.mjs", "{reportPath}"],
      "reportPath": "frontend.json",
      "expectedSchemaVersion": "1",
      "timeoutMs": 120000,
      "cwd": "."
    }
  ]
}
```

- Lista legacy sigue aceptándose como compatibilidad temporal.
- Claves desconocidas, names duplicados, timeout fuera de `1..30 min`, traversal y rutas absolutas fuera fallan cerrado.
- `reportPath` relativo se resuelve contra el `reportRoot`; `cwd` y `--stages` relativos se resuelven contra el workspace.
- Manifest, reportRoot, reportes y cwd se comprueban físicamente contra symlink/junction escape.
- El transporte usa argv estructurado y `shell:false`.
- El stage debe producir el reporte estructurado que ya consume Sentinel: `schemaVersion`, `entries` y findings válidos.

## Evidencia y límites

SNT-16 todavía no está adoptado ni publicado: el submódulo contiene cambios no committeados sobre `20c13a2` y el consumidor sigue fijando `20c13a2`. No actualizar `quality-tools.json`, `sentinel.lock.json`, la skill global ni retirar scripts hasta publicar un commit/tag upstream, regenerar el lock, compilar desde un clon limpio y ejecutar fixtures en al menos dos proyectos.

Rollback: descartar el gitlink nuevo y conservar `task-check`/adapter local; el formato legacy mantiene compatibilidad con el core actual. Un manifest v1 inválido nunca se degrada a PASS.

## Bloqueos siguientes

1. Publicar upstream el cambio revisado y fijar commit/hash/capacidades en el consumidor.
2. Añadir fixtures de dos proyectos y observación de paridad de decisión, severidad, file, line y mensaje.
3. Ejecutar CI/multi-shell y documentar rollback/GC.
4. Solo después reducir adapter y evaluar la skill global al final de Fase 4.
