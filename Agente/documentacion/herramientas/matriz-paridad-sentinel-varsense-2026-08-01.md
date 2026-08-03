# Matriz de paridad Sentinel/VarSense — 2026-08-01

## Versiones fijadas

| Herramienta | Versión | Commit | CLI | LSP/VS Code | Fixture/gate |
| --- | --- | --- | --- | --- | --- |
| Glory Sentinel | 0.4.0 | `107be9b61a7ed4676ee89b101ecff4112a039fb2` | PASS | Core editor-agnóstico PASS | `npm run test:unit`, `task:check` |
| VarSense | 2.2.0 | `b1aa3f06ffbb96a55dd0156a99eae482f41311b8` | `scan`, `orphan-classes`, `all` | Core/LSP/VS Code PASS | 46 pruebas, `npm test` |

## Contratos

- Los dos CLIs escriben JSON con `schemaVersion: 1`, `entries`, severidad y rango estable.
- Sentinel añade `remediation`, `confidence` y `analyzerVersion` al contrato core; los adapters traducen sin importar APIs del editor.
- VarSense `all` comparte un `CachedNodeDocumentProvider` para que scan e orphan-classes reutilicen el snapshot de archivos dentro de una ejecución.
- `scripts/quality/adapters/varsense.mjs` invoca `all` una sola vez; no mantiene dos procesos ni dos reportes como camino normal.
- Sentinel recibe boundaries por `portableBoundaries`; el proyecto configura sus excepciones en `sentinel.config.json`.
- VarSense `all` añade `token-duplicate` y `token-unused` sobre el mismo snapshot de documentos.

## Política de migración

- El patch downstream de clases dinámicas se retiró de `quality-tools.json`; la capacidad está fijada en el commit upstream de VarSense.
- Un cambio de schema, ruleId o severidad requiere actualizar esta matriz, fixtures de equivalencia y el fingerprint de caché antes de cambiar el manifest.
- El empaquetado `.vsix` y la instalación en el editor se ejecutan solo después de compile, lint, smoke LSP y suite; nunca se reinicia VS Code automáticamente.

## Nota de sincronización

Los commits fijados en `quality-tools.json` son la fuente que consume este gate. Los repositorios de desarrollo upstream pueden estar detrás; este workspace no los sincroniza ni declara paridad de `main` sin un checkout verificable. La migración 028A-6 añade un contrato local de política v2 y mantiene el formato Sentinel v1 como configuración del analizador hasta que exista el runtime global.

## Pendientes explícitos

- Benchmark small/medium/full con memoria RSS comparable en Windows/Linux CI.
- Paridad visual del panel VS Code frente a CLI/LSP para los nuevos metadatos.
- Publicar releases upstream en sus repositorios remotos; este workspace solo fija commits instalados.
