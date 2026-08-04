# Matriz de paridad Sentinel/VarSense — 2026-08-01

## Versiones fijadas

| Herramienta | Versión | Commit | CLI | LSP/VS Code | Fixture/gate |
| --- | --- | --- | --- | --- | --- |
| Glory Sentinel | 0.4.0 | `7ad3b766207bb28d89d38a938ee14fbad9f4cd49` | PASS | Core editor-agnóstico PASS | `npm run test:unit`, `task:check` |
| VarSense | 2.2.0 | `4167868dd5d0e7674d5565ade399a57796d69cf3` | `scan`, `orphan-classes`, `all` | Core/LSP/VS Code PASS | 46 pruebas, `npm test` |

> **Sync 038A-5 (2026-08-04):** los commits previos (`107be9b6`/`b1aa3f06`) resultaron inexistentes en los repos dev y en `origin`; las features solo vivían en la copia instalada. Se portaron los commits reales a `main` (sentinel `7ad3b76`, varsense `4167868`, pusheados a `1ndoryu/*`) y la copia instalada quedó re-sincronizada a esos commits (ver plan 028A-6, sección de inventario).

## Contratos

El gate de este checkout consume la copia instalada bajo `.quality-tools` y la
valida contra `quality-tools.json` + `sentinel.lock.json`. Los repositorios de
desarrollo upstream (`main`) no son una fuente implícita de runtime ni se
consideran sincronizados sin evidencia de commit/hash.

- Los dos CLIs escriben JSON con `schemaVersion: 1`, `entries`, severidad y rango estable.
- Sentinel añade `remediation`, `confidence` y `analyzerVersion` al contrato core; los adapters traducen sin importar APIs del editor.
- VarSense `all` comparte un `CachedNodeDocumentProvider` para que scan e orphan-classes reutilicen el snapshot de archivos dentro de una ejecución.
- `scripts/quality/adapters/varsense.mjs` invoca `all` una sola vez; no mantiene dos procesos ni dos reportes como camino normal.
- Sentinel recibe boundaries por `portableBoundaries`; el proyecto configura sus excepciones en `sentinel.config.json`.
- VarSense `all` añade `token-duplicate` y `token-unused` sobre el mismo snapshot de documentos.
- `varsense all` es la única etapa normal del gate; `scan` y `orphan-classes` se conservan para compatibilidad CLI y no crean un segundo scheduler/reporte.

## Política de migración

- `sentinel.lock.json` fija versiones, protocolo, commits e identidad SHA-256 de los analizadores; el runtime de transición es `project-adapter` y mantiene `artifactSha256: null` hasta existir un runtime global instalable.
- Los reportes, cachés y locks del gate se particionan por `branch-key-v1` bajo `.quality-reports/branches/<branch-key>/`; la retención usa TTL/cuotas y la poda destructiva requiere confirmación explícita.

- El patch downstream de clases dinámicas se retiró de `quality-tools.json`; la capacidad está fijada en el commit upstream de VarSense.
- Un cambio de schema, ruleId o severidad requiere actualizar esta matriz, fixtures de equivalencia y el fingerprint de caché antes de cambiar el manifest.
- El empaquetado `.vsix` y la instalación en el editor se ejecutan solo después de compile, lint, smoke LSP y suite; nunca se reinicia VS Code automáticamente.

## Nota de sincronización

Los commits fijados en `quality-tools.json` son la fuente que consume este gate. Los repositorios de desarrollo upstream pueden estar detrás; este workspace no los sincroniza ni declara paridad de `main` sin un checkout verificable. La migración 028A-6 añade un contrato local de política v2, mantiene el formato Sentinel v1 como configuración del analizador durante la transición y no simula la instalación del runtime global.

## Pendientes explícitos

- Benchmark small/medium/full con memoria RSS comparable en Windows/Linux CI.
- Paridad visual del panel VS Code frente a CLI/LSP para los nuevos metadatos.
- Publicar releases upstream en sus repositorios remotos; el sync de `main` con las features fijadas se completó el 2026-08-04 (038A-5); la publicación de tags/releases formales queda pendiente.
- Instalar el runtime global, exigir `artifactSha256` real y ejecutar la matriz multi-shell/multi-proyecto; estas capacidades no se declaran implementadas en este repositorio.
