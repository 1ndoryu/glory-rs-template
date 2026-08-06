# Inventario de scripts y adapters frente a Sentinel

> Fecha de corte: 2026-08-07
> Iniciativa canónica: `Agente/planes/plan-migracion-scripts-adapters-sentinel-2026-08-06.md`

## Decisión

El plano universal debe vivir en Sentinel Core. El consumidor conserva únicamente un adapter pequeño y scripts que encapsulan dominio, proveedor, base de datos, generación o rescate operacional. No se copia `scripts/quality` a otros proyectos y no se retiran wrappers por estética.

## Estado por capa

| Capa | Ubicación | Estado | Decisión |
|---|---|---|---|
| Core universal | upstream Sentinel | Contrato SNT-16 no publicado desde este checkout | No fijar ni prometer capacidades ausentes en el gitlink. |
| Manifest de stages | `tools/sentinel/src/core/` | Consumidor sigue en `20c13a2` limpio | El diseño upstream queda pendiente de commit/release recuperable. |
| Adapter del consumidor | `scripts/quality/adapter-manifest.mjs`, adapters | SNT-15 cerrado | Sigue como frontera local. |
| Gate transitorio | `scripts/quality/task-check.mjs` | Se conserva | No se reemplaza por `sentinel check` hasta release y paridad real. |
| Scripts de dominio | `scripts/run-with-db.mjs`, codegen, preparación DB | Se conservan | Encapsulan Rust/PostgreSQL y no entran al core universal. |
| Analyzers | Sentinel + VarSense | Se conservan separados | VarSense es analyzer, no gate ni reporter paralelo. |
| Fixtures SNT-16b | `scripts/quality/tests/fixtures/`, `snt-16b-parity.test.mjs` | Local, 2/2 PASS | Solo contrato/normalización; no sustituye dos proyectos ejecutados por Sentinel. |

## Contrato objetivo upstream

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

La lista legacy debe mantenerse temporalmente. La validación objetivo es estricta, fail-closed, con argv estructurado, timeouts acotados y contención física de manifest, reportRoot, reportes y cwd. `reportPath` relativo se resuelve contra `reportRoot`; `cwd` contra workspace.

## Evidencia y límites actuales

- Consumidor y submódulo Sentinel limpios; Sentinel fijado en `20c13a216e879303fcf5be7469a2821391b2ec0d` (`0.5.0`).
- `ef9c751` no está en refs ni objetos recuperables del submódulo; no se cambia el gitlink ni se inventa un hash.
- Fixture dirigida SNT-16b: **2 PASS, 0 FAIL**. Comprueba normalización y que severidad/mensaje forman parte de la identidad.
- Suite consumidor: **223 PASS, 3 FAIL de entorno, 1 skip**. Los fallos requieren CLI Sentinel/VarSense y configuración local provisionada.
- No hay evidencia de compilación upstream en esta sesión: el guard intenta cargar `quality-command-guard.mjs` ausente desde el worktree.

## Rollback y permanencia

Rollback inmediato: conservar gitlink `20c13a2`, `task-check` y adapter SNT-15. No borrar scripts ni modificar la skill global. La retirada exige un release upstream publicado, clon limpio compilable, al menos dos proyectos reales, cinco gates comparables, matriz multi-shell/CI, dos releases consecutivos y rollback/GC probado.

## Siguiente bloque

1. Obtener/publicar el cambio upstream en el repositorio autorizado.
2. Compilar y ejecutar suite desde clon limpio.
3. Ejecutar fixtures Node/Rust con envelope y legacy mediante el CLI real y comparar decisión, estado, severidad, `ruleId`, file, line y message.
4. Fijar commit/capabilities/hash en `quality-tools.json` y `sentinel.lock.json`, repetir gate y solo entonces evaluar adelgazar `task-check`.
