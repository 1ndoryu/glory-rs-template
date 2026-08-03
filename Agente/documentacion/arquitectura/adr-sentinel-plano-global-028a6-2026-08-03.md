# ADR — Sentinel como plano global de calidad (028A-6)

> **Fecha:** 2026-08-03
> **Estado:** aceptado para migración incremental

## Contexto

El repositorio tiene un quality gate operativo en `scripts/quality` y una copia fijada de Sentinel/VarSense bajo `.quality-tools`. El plan 028A-6 propone mover política, guard, scope, scheduler, caché, analyzers y reporter a un runtime global instalable fuera de cualquier checkout.

Los repositorios upstream y una instalación global administrada no forman parte de este checkout. Por tanto, instalar shims en perfiles del usuario o declarar paridad upstream como PASS desde este proyecto sería inseguro y no verificable.

## Decisión

1. Sentinel será el plano futuro de control; VarSense seguirá siendo un analyzer especializado invocado por Sentinel.
2. La migración se ejecuta por contratos y adapters, nunca sustituyendo `task-check` antes de que exista paridad.
3. `sentinel.config.json` v1 conserva su significado actual de analyzer. La política v2 se valida mediante un contrato separado durante la transición; una configuración v1 no activa enforcement v2 accidentalmente.
4. `sentinel doctor --migrate --dry-run` es la primera operación de migración. No escribe archivos, no instala herramientas y no modifica perfiles.
5. El guard conserva defaults legacy hasta que exista una política v2 válida. Una política inválida no bloquea comandos desconocidos; `doctor` y CI deben reportarla como error.
6. La instalación global, los shims persistentes, los leases, la sincronización de repos upstream y la matriz multi-shell quedan bloqueados hasta disponer del runtime global versionado y sus fixtures.

## Consecuencias

- Se puede probar el contrato y la migración sin romper `task:check` ni otros proyectos.
- Durante la transición existen dos formatos: analyzer v1 y política v2 propuesta. La futura migración aplicada deberá usar backup, hash y rollback.
- No se afirma que el enforcement global esté terminado: el gate actual sigue siendo la autoridad de cierre.

## Evidencia de este bloque

- `scripts/quality/policy.mjs`: validación estricta, descubrimiento y migración v1→v2 en memoria.
- `scripts/quality/sentinel-doctor.mjs`: diagnóstico y dry-run sin escrituras.
- `scripts/quality/policy-defaults.mjs`: catálogo único de comandos bloqueables para el guard de transición.
- `scripts/quality/quality-command-guard.mjs`: consume política v2 válida; mantiene fallback legacy seguro.
- `scripts/quality/tests/policy.test.mjs` y tests del guard: fixtures de rutas, claves desconocidas, modos y migración.
- `scripts/quality/lock-generator.mjs`: generación/verificación local del lock sin instalación, comparación estructural ignorando `generatedAt`, backup `.bak` y escritura atómica.
- `scripts/quality/sentinel-doctor.mjs --lock`: diagnóstico/generación explícita; `--check` no escribe y `--write` no modifica analyzers.
- `scripts/quality/tests/lock-generator.test.mjs`: 6 fixtures de parseo, generación, no-escritura, mismatch, backup y symlink/tamper.

## Gates pendientes

- Runtime global versionado instalado y verificable.
- Runtime global versionado con `artifactSha256` real; el adaptador local mantiene `artifactSha256: null`.
- `realpath`/canonicalización verifican que lockfile, install root, backup y checkouts permanezcan dentro del workspace; el generador local añade escritura atómica y backup probado.
- Paridad CLI/LSP/VS Code y matriz PowerShell/CMD/Bash/CI.
- Lease de procesos hijos, rollback de perfiles y segundo proyecto sin política.
