# Plan 028A-6 — Sentinel como plano global de calidad agnóstico

> **Fecha:** 2026-08-02
> **Estado:** migración incremental en ejecución; el contrato local v2 y `doctor --migrate --dry-run` están implementados. La instalación global/upstream/multi-shell sigue bloqueada hasta disponer de los runtimes externos y sus fixtures.
>
> **ADR:** `Agente/documentacion/arquitectura/adr-sentinel-plano-global-028a6-2026-08-03.md`.
>
> **Regla de ejecución:** este plan es una iniciativa multi-release. No se ejecuta como un único cambio: cada fase debe cerrar su gate y no se marca una fase upstream/global con evidencia simulada.
> **Motivación:** el guard actual depende de `scripts/quality` dentro de este repositorio. Al cambiar de rama o de proyecto no debe desaparecer, bloquear comandos legítimos ni ejecutar reglas de wandori.us fuera de su alcance.

## Decisión arquitectónica corregida

No habrá un tercer producto llamado `GloryQuality`. **Sentinel será el único plano de control y orquestación de calidad**; VarSense seguirá siendo un analizador especializado que Sentinel ejecuta mediante un contrato de plugin.

1. **Sentinel Core/CLI global:** runtime agnóstico instalado fuera de los repositorios. Resuelve la política, intercepta comandos, aplica cooldown/locks, calcula alcance incremental, orquesta etapas, administra cachés y genera el reporte.
2. **Analizadores:** reglas nativas de Sentinel, VarSense para tokens/clases CSS y futuros analizadores. Cada analizador conserva su especialidad, pero no decide por separado el gate, el cooldown ni el reporte final.
3. **Política local:** configuración declarativa versionada por proyecto y rama. Define qué comandos, analizadores y perfiles aplican; nunca contiene código ejecutable.

El nombre público será `sentinel` (`sentinel check`, `sentinel guard`, `sentinel doctor`, `sentinel status`). `task:check`, `quality-command-guard`, `global-cargo-guard` y los scripts actuales serán adaptadores de migración y después se retirarán. VarSense podrá seguir teniendo CLI/LSP propios para uso de editor, pero en el flujo de agentes su única autoridad de cierre será Sentinel.

### Contrato de responsabilidades

| Capacidad                                              | Responsable único                        |
| ------------------------------------------------------ | ---------------------------------------- |
| Política por repositorio/rama                          | Sentinel Core                            |
| Intercepción de `cargo`, `npm`, `npx`, `rustfmt`, etc. | Sentinel Guard                           |
| Cooldown de 3 horas, locks y cuota de targets          | Sentinel Scheduler                       |
| Scope incremental y caché de etapas                    | Sentinel Orchestrator                    |
| Reglas de variables/clases CSS                         | VarSense Analyzer, invocado por Sentinel |
| Reglas generales de código                             | Sentinel Analyzers                       |
| Reporte Markdown/JSON y exit code                      | Sentinel Reporter                        |
| Configuración específica del proyecto                  | `sentinel.config.json`                   |

Así se mantienen dos herramientas reales —Sentinel y VarSense—, no tres controles superpuestos.

## Estado real que condiciona el diseño

- Sentinel `0.4.x` ya tiene CLI `analyze`, `--files-from`, configuración estricta y adapters de CLI/LSP/VS Code, pero todavía no es un scheduler ni un interceptor global.
- VarSense `2.2.x` ya tiene core/CLI/LSP y el comando `all`; su ejecución no debe ganar una segunda política de cooldown ni un reporte de cierre independiente.
- El scheduler actual (`scripts/quality`) es específico de wandori.us: decide alcance, caché, cooldown, Rust/frontend, documentación y reporte. Se migrará por etapas; no se copiará entero dentro de Sentinel de una sola vez.
- La primera implementación debe conservar `sentinel analyze` para no romper editores. `sentinel check` será el orquestador; `sentinel guard` solo decide si un comando directo está permitido; `sentinel doctor/status` diagnostican instalación y política.

### Taxonomía pública de comandos

| Comando                            | Responsabilidad                                               | ¿Ejecuta analizadores?      |
| ---------------------------------- | ------------------------------------------------------------- | --------------------------- |
| `sentinel analyze`                 | Análisis de archivos/workspace y salida normalizada           | Sí, uno o varios analyzers  |
| `sentinel check <task-id>`         | Gate completo/incremental, etapas, caché, reporte y exit code | Sí, mediante el orquestador |
| `sentinel guard <command>`         | Interceptar una validación directa y recomendar el gate       | No                          |
| `sentinel doctor/status`           | Diagnosticar política, versión, shims, locks y cachés         | No                          |
| `sentinel install/update/rollback` | Gestionar el runtime global versionado                        | No                          |

El alias `npm run task:check -- <task-id>` solo delegará a `sentinel check` durante la migración. No se deben mezclar `analyze` y `check` en un único comando ambiguo.

### Contrato mínimo de plugin de analizador

Sentinel define un contrato versionado y VarSense lo implementa mediante adapter, sin importar código de VS Code ni reglas del proyecto:

- **Entrada:** `workspaceRoot`, `scopeManifest`, `config`, `toolchain`, `abortSignal` y límites de tiempo/memoria.
- **Salida:** `schemaVersion`, `analyzerId`, `analyzerVersion`, findings normalizados (`ruleId`, `severity`, `confidence`, ruta relativa, posición, mensaje estable, remediation), métricas y razones de invalidación.
- **Estados:** `pass`, `findings`, `tool-error`, `timeout`, `cancelled`, `invalid-output`; un error de herramienta nunca se convierte en PASS.
- **Transporte:** CLI JSON/JSONL con argumentos separados como contrato portable; integración in-process solo si existe API versionada y aislamiento equivalente.
- **Compatibilidad:** Sentinel valida `protocolVersion` y la versión mínima de VarSense antes de ejecutar; la CLI/LSP de VarSense sigue siendo un adapter de presentación.

### Ubicación estable global

- Runtime versionado: `%LOCALAPPDATA%\GlorySentinel\versions\<version>\`.
- Alias activo: `%LOCALAPPDATA%\GlorySentinel\current\`.
- Shims `npm.cmd`, `npx.cmd`, `cargo.cmd`, `rustfmt` y CLI: `%LOCALAPPDATA%\GlorySentinel\bin\`.
- Estado/cooldown compartido: `C:\tmp\glory-sentinel\`, separado por raíz canónica del proyecto y `policyHash`.
- Los perfiles PowerShell y Bash solo cargarán `%LOCALAPPDATA%\GlorySentinel\current\profile.ps1`/`profile.sh`; nunca una ruta dentro de este repositorio.

El cambio de rama no altera el runtime global. Actualizar el runtime será una operación explícita (`sentinel install` o `sentinel update`) y tendrá backup/rollback.

## Política declarativa por proyecto

Cada proyecto que quiera enforcement añade `sentinel.config.json` en su raíz. No se ejecuta nada desde este archivo: se parsea como JSON estricto, con claves allowlisted y límites acotados. `.quality/guard-policy.json` queda como alias de migración temporal, no como segundo contrato.

Ejemplo para wandori.us:

```json
{
    "schemaVersion": 1,
    "mode": "enforce",
    "gate": {
        "command": ["sentinel", "check", "--"],
        "taskIdRequired": true
    },
    "guard": {
        "directCommands": {
            "npmScripts": ["test", "test:*", "type-check", "lint", "build"],
            "npxTools": ["vitest", "tsc", "eslint", "prettier"],
            "cargoSubcommands": ["check", "fmt", "test", "clippy", "bench"],
            "tools": ["rustfmt"]
        }
    },
    "analyzers": {
        "sentinel": {"profile": "project-default"},
        "varsense": {"enabled": true, "config": "varsense.config.json"}
    },
    "allow": ["dev", "preview", "codegen", "quality:*"]
}
```

### Identidad de rama, partición de reportes y retención

El gate no debe escribir todos los resultados en un único namespace creciente. La persistencia local se organizará por workspace y rama, con una identidad estable y segura:

- **Reportes canónicos:** `.quality-reports/branches/<branch-key>/<task-id>/latest.{md,json}` y sus logs/tool reports debajo del mismo namespace.
- **Caché y locks:** `.quality-reports/branches/<branch-key>/cache/` y `.quality-reports/branches/<branch-key>/locks/`; ningún PASS, lock o log de una rama se reutiliza silenciosamente en otra.
- **Identidad normal:** `git symbolic-ref --short HEAD` para ramas locales.
- **Detached HEAD/CI:** prioridad determinista: ref explícita entregada por el adapter, `GITHUB_HEAD_REF`/`GITHUB_REF_NAME`/`CI_COMMIT_REF_NAME` allowlisted, rama Git local y, finalmente, `detached-<full-sha>`. La ref y el SHA completo quedan en metadata; el nombre de directorio nunca contiene el SHA sin límite.
- **`branch-key-v1`:** seleccionar una `canonicalRef` UTF-8 (la ref allowlisted elegida o `detached:<full-sha>`), normalizarla a Unicode NFC, preservar mayúsculas, rechazar NUL/control y refs que no cumplan el contrato allowlisted. El hash es SHA-256 de los bytes UTF-8 de esa `canonicalRef`, expresado en hexadecimal minúsculo; codificar cada byte UTF-8 fuera de `[A-Za-z0-9._-]` como `_HH`, limitar el prefijo a 64 caracteres y añadir `--<hash[0:16]>`. El resultado total no supera 96 caracteres y siempre queda dentro de `[A-Za-z0-9._-]`; versión, `canonicalRef`, algoritmo, encoding y hash forman parte de metadata/fingerprint.
- **Compatibilidad de transición:** el `latest` histórico en `.quality-reports/<task-id>/` no será un puntero global escribible. Durante dos versiones solo se leerá si el JSON contiene metadata de rama (`branchKeyVersion`, `branchKey`, `canonicalRef` y `commit`) y coincide exactamente con la rama actual; un reporte antiguo sin metadata se considera ambiguo y no se reutiliza ni migra automáticamente. El writer canónico escribirá únicamente en `branches/<branch-key>/`. Después se retirará esa lectura.
- **Retención:** `quality.config.json` declarará TTL y cuotas allowlisted. Defaults iniciales de bajo consumo: 7 días, 512 MiB por workspace y 128 MiB por `branch-key`; la cuota incluye todos los archivos regulares de reportes, logs, tool reports, caché y locks, pero no `C:\tmp\glory-target`. La rama activa nunca se borra para cumplir cuota: si excede 128 MiB, el reporte marca `overQuota`, informa bytes y candidatos bloqueados, y la poda puede actuar sobre históricos elegibles; al cambiar de rama, su namespace deja de estar protegido y se poda en la siguiente ejecución.
- **Poda segura:** será explícita o ejecutable como etapa best-effort después del gate, tendrá `--dry-run`, informará bytes/candidatos eliminados y nunca cambiará el exit code ni el resultado del análisis. Un fallo de poda o una cuota excedida se registra como estado auditable y no bloquea el gate; se preservan `.tmp-*`/temporales de `writeAtomic`, locks activos, locks huérfanos hasta superar su TTL y comprobar que su PID no está activo, y archivos con escritura reciente. Los locks huérfanos elegibles cuentan para la cuota y pueden eliminarse solo después de ese criterio.
- **CI/artifacts:** los artifacts se publicarán con `branch-key`, task ID y commit corto; no se mezclará `latest` de una rama con otra en runners reutilizados.

### Versionado y migración de configuración

El proyecto ya usa `sentinel.config.json` v1 para reglas, includes, excludes y boundaries del analizador. No se puede reutilizar ese nombre introduciendo `gate` y `guard` sin contrato de migración.

- [x] Definir y validar localmente `sentinel.config.json` v2 como envelope con `mode`, `gate`, `guard`, `runtime`, `analyzers.sentinel` y `analyzers.varsense` (`policy.mjs`); publicar el JSON Schema final sigue ligado a Sentinel Core upstream.
- [x] Mapear automáticamente la configuración v1 actual a `analyzers.sentinel` sin cambiar severidades ni patrones; las claves desconocidas fallan en `sentinel doctor` antes de producir el preview.
- [x] Migrar en preview `quality.config.json` (timeouts, perfiles, cooldown, presupuestos), `varsense.config.json` y `quality-tools.json` mediante `sentinel doctor --migrate --dry-run`; devuelve `mapped`, conserva `legacyPreserved` y mantiene `writes: []` sin escribir.
- [x] Crear `sentinel.lock.json` para fijar runtime, protocolo, versión/commit/hash de Sentinel y VarSense; el runtime local queda explícitamente como `project-adapter`, sin simular instalación global.
- [x] Usar un formato mínimo estable para el lock: `schemaVersion`, runtime `{status, version, commit, identitySha256, artifactSha256}`, analyzers `{version, protocolVersion, commit, sha256}` y fecha de generación; nunca guardar secretos. `identitySha256` no sustituye el hash de artefacto: en `project-adapter`, `artifactSha256` debe existir explícitamente como `null`; un runtime global instalado exigirá hash real. Locks históricos sin el campo fallan cerrado y se regeneran con `quality:lock --write`.
- [x] Integrar preflight con validación de lock, versión/protocolo/commit, hash reproducible `git archive`, patch declarado y rechazo de checkouts modificados; solo se tolera `.quality-install.json` como metadata administrativa exacta.
- [x] Incluir la identidad del lockfile en el fingerprint de caché para invalidar PASS ante cambios de hashes fijados.
- [x] Definir precedencia local: la configuración del proyecto aporta la allowlist/patrones y los defaults locales; el perfil explícito de CLI/CI (`--profile` > `GLORY_QUALITY_PROFILE`) solo selecciona perfiles ejecutables (`rust`, `frontend`, `css`, `docs`) y prevalece sobre la autodetección. Nunca cambia severidades, política, lock ni enforcement (`args.mjs`, `scope.mjs`, `profile-contract.mjs`, `stage-definitions.mjs`). `full` de fingerprint queda separado de `executionFull`, Sentinel conserva `--files-from` en el perfil explícito y el reporte distingue ambos alcances; la integración del perfil en el runtime global y un perfil nominal configurable siguen pendientes.
- [x] Mantener lectura local de los formatos anteriores durante dos versiones de runtime como compatibilidad estrictamente read-only: el lector devuelve metadata `legacy-read-only`, `compatibilityVersion: 1`, `maxRuntimeVersions: 2`, `retireAfterCompatibilityVersion: 3`, warning visible y retiro `after-two-runtime-versions`; no escribe alias, no migra automáticamente y los reportes ambiguos no se reutilizan. La retirada efectiva del runtime global sigue pendiente.
- [x] Añadir generador local `quality:lock --check|--write` y `quality:doctor --lock`; `--check` es solo lectura, `--write` crea `.bak` y reemplaza atómicamente, sin instalar runtime ni mutar analyzers.
- [x] Rechazar lock/checkouts y backups por symlink/junction fuera del workspace; preservar el fallo cerrado ante cambios reales en `.quality-tools`.

**Gate:** una migración dry-run no modifica archivos; el lock local es estricto, reproducible y fail-closed ante divergencias. La generación aplicada local queda verificada con backup y escritura atómica; la instalación/rollback del runtime global sigue pendiente.

### Resolución de política

- [x] Buscar desde el directorio actual hacia arriba hasta la raíz del workspace (`discoverPolicy`); la resolución física del directorio inicial evita seguir una ruta lógica con junction/symlink.
- [x] Usar únicamente `sentinel.config.json` como fuente canónica; no inferir reglas leyendo `AGENTS.md` ni scripts arbitrarios. `discoverPolicy`/`loadPolicy` solo recorren ancestros buscando ese archivo, y la identidad/hash no cambia cuando se modifican documentos, `quality.config.json` o scripts auxiliares (`policy.mjs` + regresión canónica).
- [x] Canonicalizar la ruta antes de leerla y rechazar `sentinel.config.json` symlink/junction en el loader y en el guard; la configuración externa no se carga ni se sigue desde los shims (`policy.mjs`, `quality-command-guard.mjs` y tests).
- [x] Calcular `policyHash` desde la configuración descubierta y asociarlo al estado/reporte; el fingerprint de caché lo incluye para invalidar PASS cuando cambia la política. (`scripts/quality/policy.mjs`, `cache.mjs`, `reporter.mjs`)
- [ ] Asociar también la identidad a un runtime global instalado y a leases firmados. *(pendiente del runtime global)*
- [x] Si no existe política, la decisión local es `pass-through` y el guard permite trabajar sin bloquear (`policy-decision.mjs`, `quality-command-guard.mjs` + tests); el comportamiento del runtime global sigue pendiente.
- [x] Si existe una política inválida: no bloquear comandos desconocidos; mostrar una advertencia/error conciso y hacer fallar `sentinel doctor`/CI para que el proyecto corrija su configuración. El guard solo bloquea el comando protegido que detecta; no bloquea comandos desconocidos (`policy-decision`, `quality-command-guard`, `sentinel-doctor` + tests).
- [x] Si `mode` es `observe`, el guard registra `observed` y no bloquea la ejecución; la comparación dual contra Sentinel Core upstream sigue pendiente.
- [x] Si `mode` es `enforce`, el guard bloquea solo comandos declarados y devuelve código 78; el enforcement del launcher global sigue pendiente.
- [x] VarSense no crea cooldown, lock ni reporte paralelo: el adapter ejecuta una sola invocación `all`, usa el `reportRoot`/logs del gate y devuelve hallazgos mediante `structured-tool`; la metadata `varsenseScope` conserva el manifiesto solicitado y marca `varsense-cli-{version}-no-files-from` porque la versión fijada aún no acepta `--files-from`. La entrega efectiva de `files-from` queda pendiente de actualizar VarSense/upstream.

## Arquitectura por fases

### Fase 0 — ADR, contratos y compatibilidad

- [x] Crear ADR con Sentinel Core, el contrato de analizadores (incluido VarSense), la política local y la matriz `enforce/observe/pass-through`. (`adr-sentinel-plano-global-028a6-2026-08-03.md`)
- [x] Implementar validación estricta local de la política v2 y descubrimiento por ancestros en `scripts/quality/policy.mjs`.
- [x] Implementar `quality:doctor --migrate --dry-run`; no escribe archivos ni cambia perfiles.
- [x] Añadir fixtures de política válida, claves desconocidas, rutas fuera del workspace, modos, migración v1→v2 e identidad/hash (`scripts/quality/tests/policy.test.mjs`, `policy-identity.test.mjs` + guard/cache).
- [x] Centralizar los defaults de comandos bloqueables para que el guard de transición y la migración no mantengan catálogos divergentes.

- [ ] Crear JSON Schema publicado con Sentinel Core (la fuente de runtime global no está presente en este checkout).
- [x] Definir y validar localmente el contrato v2, errores allowlisted y límites de tamaño de strings/listas/rutas; publicar el JSON Schema queda ligado al runtime upstream.
- [x] Añadir al reporte local la identidad estable de política: `projectRoot`, `policyPath`, `policyHash`, `runtimeVersion`, `reason` y comando recomendado; se mantiene `schemaVersion: 1` por compatibilidad aditiva.
- [ ] Definir contrato final de salida de Sentinel Core con decisión/exitCode y transporte CLI/LSP. *(pendiente del runtime global)*
- [ ] Definir contrato final de plugin, taxonomía `analyze/check/guard/doctor` y matriz de compatibilidad Sentinel↔VarSense. El contrato local parcial de salida/error ahora valida `entries` y estados fail-closed; el contrato Core/upstream y `cancelled` siguen pendientes.
- [x] Definir compatibilidad Windows PowerShell 5/7, PowerShell Core, CMD, Bash/Git Bash (interactivo y `BASH_ENV`) y CI sin depender de variables específicas de VS Code: contrato documental en `Agente/documentacion/herramientas/matriz-shells-sentinel-2026-08-04.md` (shims con `shell: false`, exit codes/redirecciones, frontera de enforcement con bypass no interceptable, launcher del agente/CI, refs CI allowlisted). La ejecución de la matriz real queda en Fase 4.
- [x] Definir política de actualización, rollback y migración desde el guard actual: contrato documental en `Agente/documentacion/herramientas/politica-actualizacion-rollback-sentinel-2026-08-04.md` (versionado `%LOCALAPPDATA%\GlorySentinel`, flujo update/rollback con backup+hash+rename atómico, migración del guard actual, retirada tras dos releases y desinstalación solo de entradas administradas). La instalación/rollback efectivos quedan en Fase 1/5.

**Gate:** ADR aprobado; fixtures y doctor local pasan; los contratos documentales de shells y de actualización/rollback quedan definidos. La fase 0 queda parcialmente cerrada: el schema/runtime global, la salida final y la ejecución de la matriz multi-shell permanecen pendientes upstream.

### Fase 1 — Sentinel Core global instalable y estable *(bloqueada: runtime upstream ausente)*

- [ ] Extraer el clasificador, scheduler, scope, caché y reporter a Sentinel Core, sin imports de wandori.us ni de VarSense.
- [ ] Crear CLI global `sentinel check|guard|doctor|status|install|update|rollback`.
- [ ] Instalar versiones en `%LOCALAPPDATA%\GlorySentinel\versions` y cambiar `current` de forma atómica.
- [x] Ejecutar analyzers mediante adapters locales aislados con timeout, límite de salida y estados distinguibles `tool-error/timeout/invalid-output`; el contrato valida raíz, `entries`, `findings`, `ruleId`, `message` y severidad allowlisted. El estado `cancelled` del runtime global y la salida final Core quedan pendientes (`structured-tool.mjs`, `common.mjs` + tests).
- [ ] Generar shims con resolución del ejecutable real sin recursión; preservar argumentos, códigos de salida y redirecciones.
- [ ] Dot-sourcear únicamente la ruta global estable en ambos perfiles; crear backup antes de cualquier modificación.
- [ ] Mantener los wrappers del repositorio solo como adaptadores para desarrollo, no como dependencia del perfil global.

**Gate:** una rama que elimina `scripts/quality` no rompe el perfil ni el CLI global; `doctor` identifica la versión activa y el ejecutable real.

### Fase 2 — Resolución por workspace y rama *(contrato local parcial; enforcement global bloqueado)*

- [x] Implementar descubrimiento de raíz y política en cada comando, sin estado de proceso que sobreviva al cambio de rama: `discoverPolicy`/`loadPolicy` (`policy.mjs`) y `readV2GuardPolicy` (`quality-command-guard.mjs`) releen de disco en cada invocación, sin caché a nivel de módulo; regresión nueva en `policy.test.mjs` simula el cambio de política dentro del mismo proceso (enforce → observe → sin archivo) y verifica status/hash frescos (`policy.test.mjs`). El enforcement del runtime global sigue pendiente.
- [x] Diferenciar `no-policy`, `legacy-v1`, `observe`, `enforce`, `pass-through` e `invalid-policy` en el guard, doctor e identidad/reporte local (`scripts/quality/policy-decision.mjs` + fixtures); el enforcement global sigue pendiente.
- [x] Invalidar la caché local por `policyHash` además de modo, herramientas, configuración y archivos.
- [ ] Invalidar decisiones/cooldowns del runtime global por `projectRoot + policyHash + runtimeVersion`. *(pendiente del runtime global)*
- [ ] Mantener cooldown/locks solo para comandos declarados como pesados por la política; no compartirlos entre proyectos.
- [ ] Emitir leases efímeros firmados para que los procesos hijos iniciados por `sentinel check` puedan usar herramientas pesadas sin que el propio shim los bloquee; el lease debe estar ligado a PID, proyecto, comando, expiración y task ID.
- [x] Definir la frontera de enforcement: shims cubren shells normales; el launcher del agente/CI debe invocar `sentinel guard` antes de ejecutar procesos. Rutas absolutas y shells `--noprofile --norc` se registran como bypass no interceptable por un script de proyecto, no se presentan como cobertura completa. Contrato documental en `Agente/documentacion/herramientas/matriz-shells-sentinel-2026-08-04.md` §4; la ejecución de la matriz real queda en Fase 4.
- [x] Añadir al diagnóstico local la decisión estable (`action`, `mode`, `blocked`, `reason`) junto con raíz, hash y comando recomendado; diagnóstico de shims/PATH global queda pendiente del runtime externo.
- [x] Resolver la identidad de rama de forma segura para rama normal, detached HEAD, CI y nombres con `/`, espacios, unicode o longitud excesiva; añadir fixture determinista de `branch-key` (`scripts/quality/branch-identity.mjs`).
- [x] Particionar reportes, logs, caché y locks por `projectRoot + branch-key`; conservar el SHA/ref original en metadata y evitar colisiones entre ramas (`preflight.mjs`, `cache.mjs`, `lock.mjs`, `reporter.mjs`).
- [x] Implementar lectura compatible, solo lectura, del layout histórico `.quality-reports/<task-id>/`: el namespace canónico gana; el legacy solo se acepta con metadata exacta de rama, sin metadata queda ambiguo, y se rechazan traversal/symlinks/JSON canónico corrupto sin fallback (`scripts/quality/report-reader.mjs`, `scripts/quality/tests/report-reader.test.mjs`). El resultado legacy expone warning y contrato de retirada tras dos versiones; aplicar la retirada efectiva sigue pendiente del runtime global.
- [x] Añadir retención configurable por TTL/cuota (defaults: 7 días, 512 MiB por workspace, 128 MiB por rama), contando reportes/logs/tool reports/caché/locks; marcar `overQuota` sin borrar la rama activa, podar históricos/tareas/caché elegibles con `--dry-run`, respetar locks/temporales/escrituras recientes y eliminar locks huérfanos solo tras TTL + PID inactivo; registrar bytes/candidatos antes y después sin alterar el exit code (`report-retention.mjs`).
- [x] Exponer `quality:reports:cleanup:dry` y `quality:reports:cleanup`; el modo destructivo requiere `--cleanup --yes`.
- [x] Cubrir con fixture de integración el aislamiento de dos ramas para reportes, caché y locks, además de identidades CI/detached con commit compartido (`scripts/quality/tests/branch-isolation.integration.test.mjs`). Traversal/symlink, retención y poda tienen cobertura focal; la poda best-effort integrada en `task-check` conserva el resultado del gate y está cubierta por `report-retention-stage.test.mjs`. El cambio de rama dentro del mismo proceso, los locks entre namespaces y refs largas/peligrosas quedan cubiertos por la fixture; la concurrencia multi-proceso y la matriz CI real siguen pendientes del runtime global.

**Gate:** matriz con dos proyectos y dos ramas: el proyecto configurado bloquea lo declarado; el proyecto sin política pasa; cambiar de rama actualiza la decisión sin reiniciar el editor (regresión de rediscovery de política en el mismo proceso). Los reportes, locks y cachés quedan aislados por rama; la poda dry-run y aplicada respetan TTL/cuota, no toca una ejecución activa y no puede borrar fuera del workspace. La frontera de enforcement queda definida documentalmente; la matriz de shells real y el enforcement del launcher siguen en Fase 4.

### Fase 3 — Adaptador de wandori.us y VarSense *(pendiente después de Fase 1)*

- [ ] Añadir `sentinel.config.json` al proyecto con `sentinel check -- <TareaId>` como gate; conservar un alias temporal para `npm run task:check`.
- [ ] Migrar `quality-command-guard.mjs`, `global-cargo-guard.ps1`, `npm.cmd`, `npx.cmd` y `cargo.cmd` al runtime global de Sentinel sin duplicar reglas.
- [ ] Mantener `quality.config.json` solo para la transición de tiempos, alcance y cachés; la política de comandos y analizadores vive en Sentinel.
- [ ] Integrar VarSense como adaptador de analizador (`files-from`, hallazgos tipados, caché e invalidación), sin un gate ni scheduler propio.
- [ ] Actualizar `quality:install-guard` para instalar/copiar Sentinel y retirar rutas hardcodeadas del repositorio.
- [ ] Ejecutar VarSense desde Sentinel y demostrar paridad de hallazgos con su CLI/LSP, sin permitir que VarSense cierre la tarea por separado.
- [ ] Ejecutar primero en modo `observe` contra el gate actual y comparar reportes normalizados; activar `enforce` solo después de resolver diferencias, errores de herramienta y falsos positivos.
- [ ] Mantener compatibilidad temporal con el guard actual y emitir advertencia de migración, sin bloquear una rama antigua.

**Gate:** parcialmente verificado en transición: guard local, identidad de política, invalidación de caché y reportes pasan; instalación global, shells externos y runtime Sentinel quedan pendientes.

### Fase 4 — Integración multi-proyecto y CI *(pendiente de runtime global)*

- [ ] Crear fixtures de un proyecto Node, Rust, Python y un proyecto sin política.
- [ ] Probar `npm`, `npx`, `cargo`, `rustfmt`, comandos directos, `2>&1`, pipes y códigos de salida en PowerShell 5/7, CMD y Bash/Git Bash.
- [ ] Probar rutas anidadas, junctions/symlinks permitidos, repositorio movido y checkout de ramas con/sin política.
- [ ] CI usará la política del proyecto y el runtime fijado; nunca dependerá del perfil del desarrollador.
- [ ] Probar agentes con PowerShell/Bash/CMD, procesos hijos, pipes, `2>&1`, shell sin perfil y rutas absolutas; cada caso debe indicar si se bloquea, se observa o requiere enforcement del launcher.
- [ ] Publicar reportes compactos sin secretos y con máximo tres hallazgos/máximo cuatro recordatorios.

**Gate:** 100% de fixtures con decisión esperada, sin bloqueo cruzado entre proyectos y sin proceso huérfano.

### Fase 5 — Retirada segura del acoplamiento actual *(pendiente de dos releases y rollback probado)*

- [ ] Documentar rollback al runtime anterior y restaurar backups de perfiles.
- [ ] Retirar el PATH que apunta a `scripts/quality` solo después de verificar el PATH global.
- [ ] Eliminar shims duplicados del repositorio cuando dos versiones consecutivas hayan pasado la matriz.
- [ ] Mantener un comando de desinstalación que quite solo entradas administradas por Sentinel.
- [ ] Marcar el guard actual como legacy y conservar un periodo de compatibilidad para ramas antiguas.

**Gate:** rollback probado en una copia de perfil; ninguna rama activa pierde la capacidad de ejecutar su gate.

## Reglas de seguridad y resiliencia

- Nunca ejecutar comandos definidos por JSON; el JSON solo selecciona clases y un gate allowlisted.
- No ejecutar plugins, scripts ni binarios aportados por el repositorio sin versión/hash fijados en `sentinel.lock.json`; VarSense se invoca desde el runtime aprobado.
- Nunca mostrar tokens, variables de entorno, argumentos completos ni rutas sensibles en el mensaje de bloqueo.
- Si el runtime global está ausente o corrupto, `doctor` falla y los comandos de proyectos sin política pasan; no bloquear todo el sistema.
- Si el proyecto tiene `sentinel.config.json` en `enforce` y el runtime/analyzer fijado no está disponible o no coincide con el lock, el gate falla cerrado con una instrucción de reparación; solo `no-policy`/`pass-through` puede continuar.
- El guard no mata procesos ajenos ni borra targets fuera del directorio de caché/targets validado por plataforma (`C:\tmp\glory-sentinel`/`glory-target` en Windows, XDG cache en Linux/macOS).
- Shims usan `shell: false`/argumentos separados cuando invocan Node; PowerShell y CMD deben preservar códigos de salida.
- Cada analyzer tiene timeout, límite de bytes, cancelación y cleanup; un proceso huérfano queda marcado y no se reutiliza su salida.
- La configuración se trata como input no confiable: JSON sin ejecución, rutas canonicalizadas, symlink/junction dentro del workspace y globs con límites.
- Los nombres de rama nunca se usan como rutas sin codificación allowlisted; detached HEAD, refs CI y hashes tienen límites de longitud y colisión verificable.
- La poda de `.quality-reports` solo opera dentro de la raíz canónica, no sigue enlaces, respeta locks/procesos activos y registra un resultado auditable sin incluir secretos.
- Actualizaciones usan directorio temporal, hash/verificación y rename atómico; rollback conserva la versión anterior.

## Auditoría SOLID, rendimiento y escalabilidad por fase

Cada fase debe adjuntar evidencia de:

- **SRP:** resolver política, clasificar comando, ejecutar shim, persistir estado y reportar son módulos separados.
- **OCP/DIP:** nuevas herramientas se agregan en la política/configuración, no con `if/else` por proyecto en el core.
- **ISP:** el runtime expone interfaces pequeñas para filesystem, reloj, proceso y entorno; fixtures usan adaptadores fake.
- **Rendimiento:** una invocación normal añade solo una lectura JSON/cacheada y una resolución de raíz; no inicia Node adicional si no hay política.
- **Escalabilidad:** estado indexado por `projectRoot/policyHash`, locks por proyecto y pruebas con múltiples workspaces concurrentes.
- **Seguridad:** rutas, JSON, permisos, secretos, códigos de salida y rollback revisados por Sentinel/VarSense cuando aplique.
- **Observabilidad:** logs estructurados y reportes con `runtimeVersion`, `policyHash`, decisión, duración y motivo, sin datos sensibles.
- **Dependencias:** el grafo permitido es `Sentinel Core → contratos/política/scheduler/reporter`; `adapters → contratos`; `VarSense adapter → contratos`. Core no importa VS Code, LSP, VarSense ni código del proyecto.

## Documentación afectada e inventario de correcciones

La migración a Sentinel como plano único deja documentación desincronizada con el estado real (era IA eliminada, CLI `analyze`, reglas portables, `varsense all`, gate). Inventario completo; cada ítem se resuelve dentro de la fase indicada y queda verificado contra el código fijado en `quality-tools.json`.

### Repositorio glory-sentinel (repo dev `main` + copia instalada `.quality-tools/sentinel`)

- [ ] **`README.md`** — reescribir: eliminar la era IA (análisis IA contextual, comando toggle IA, config `codeSentinel.aiAnalysis.*`, alias Gemini; todo eliminado en 0.4.0) y documentar CLI `analyze`/`--files-from`/`--format`/`--output`/`--config`, exit codes 0/1/2, JSON `schemaVersion: '1'`, validación estricta de `sentinel.config.json`, `portableBoundaries` y el catálogo completo de reglas del `ruleRegistry`. Posicionar Sentinel como plano de control de calidad agnóstico (VS Code + CLI + LSP).
- [ ] **`help.txt`** — reemplazar el volcado del `--help` de Gemini CLI por el `--help` real de `sentinel analyze` (o eliminar; es residuo de la era IA).
- [ ] **`rules.md`** — reescribir con los IDs reales del `ruleRegistry` (actualmente lista IDs de la era IA que no coinciden) o eliminar.
- [ ] **`CHANGELOG.md`** — añadir entrada 0.4.x con las portable rules y `portableBoundaries`; marcar la deprecación del motor IA de la entrada 0.1.0.
- [ ] **Sincronizar `main` con el commit fijado** (`107be9b6`): portable rules (`src/analyzers/static/portableRules.ts`), `portableBoundaries` en `config.ts` y reglas `unsafe-process-shell`/`default-export` en el registry. Sin esto, `main` está detrás de lo que el gate consume y los README describen features que el repo dev no tiene.
- [x] **Parche local `[317A-3]`** declarado reproduciblemente en `scripts/quality/patches/sentinel-317a-3.patch` y `quality-tools.json` (`patchSha256=77bfdac2a3d245ecc50d9d79bc0539f52fdc0c72426aa21c53202c48c0bb9ed7`): `.boton-icono`, `botonIcono`, variantes kebab y fixture. Setup/lock verifican el archivo raíz, el diff aplicado y las rutas; sigue siendo patch local hasta upstream.

### Repositorio varsense (repo dev `main` + copia instalada `.quality-tools/varsense`)

- [ ] **`README.md`** — reescribir: nombre actual VarSense (no "CSS Variables Validator"), CLI `scan`/`orphan-classes`/`all`, binario `varsense`/`varsense-lsp`, LSP stdio, integración Zed y `tokenDetection` (hallazgos `token-duplicate`/`token-unused`).
- [ ] **`CHANGELOG.md`** — añadir `all` y `tokenDetection` a 2.2.0.
- [ ] **Sincronizar `main` con el commit fijado** (`b1aa3f06`): subcomando `all` y `tokenDetection`. El gate invoca `varsense all` (`adapters/varsense.mjs`), que solo existe en la copia instalada; contra `main` el gate fallaría.

### Proyecto wandori.us (glory-rust-template)

- [x] **`README.md` (raíz)** — documentar `npm run task:check`, el quality gate unificado, Sentinel y VarSense; aclarar que el runtime global aún no está instalado y que los wrappers directos no sustituyen el gate.
- [x] **`roadmap-sentinel.md`** — corregir la contradicción de `runVarsense`: el adaptador real invoca `varsense all`; `scan` y `orphan-classes` quedan como compatibilidad CLI.
- [x] **`Agente/documentacion/herramientas/matriz-paridad-sentinel-varsense-2026-08-01.md`** — añadir nota sobre `all`/`tokenDetection`/portable rules, copia instalada frente a repos `main`, lockfile, branch-key y límites del runtime global.
- [x] **`Agente/documentacion/indice-documentacion-2026-07-29.md`** — enlazar el estado de versiones, hashes, ramas y retención mediante el plan global y los manifiestos canónicos.

**Gate del inventario:** cada ítem cierra con evidencia (commit en el repo de la herramienta o en el proyecto) y el catálogo de reglas del README de Sentinel debe coincidir con `ruleRegistry.ts` de la copia instalada. Los repos dev sincronizados con las copias instaladas es prerequisito para que la documentación describa lo que el gate realmente consume.

## Definition of Done

- [x] El contrato local de política v2 no depende de una rama ni de archivos externos; el runtime global equivalente sigue bloqueado por ausencia del runtime upstream.
- [x] `doctor --migrate --dry-run` es reversible y no escribe archivos.
- [x] El guard de transición mantiene compatibilidad con v1 y aplica `enforce`/`observe` para v2 válida.
- [ ] El runtime global de Sentinel no depende de una rama ni de archivos del repositorio actual. *(bloqueado por runtime upstream ausente)*
- [ ] Un proyecto sin `sentinel.config.json` puede ejecutar libremente sus comandos.
- [ ] Un proyecto con `sentinel.config.json` puede exigir su propio gate, comandos y conjunto de analizadores.
- [ ] `sentinel analyze` conserva compatibilidad con el CLI/LSP/VS Code actual y `sentinel check` produce el reporte único del gate.
- [ ] VarSense se ejecuta como analyzer versionado con findings normalizados, sin cooldown, scheduler o reporte de cierre paralelo.
- [ ] La migración v1→v2 de configuración y `quality.config.json`/`quality-tools.json` tiene dry-run, backup, rollback y compatibilidad temporal.
- [ ] Sentinel permite completar su propio gate mediante lease controlado sin que sus hijos sean bloqueados por los shims.
- [ ] Cambiar de rama actualiza la política sin reiniciar VS Code ni reinstalar perfiles.
- [ ] `sentinel doctor`, CI y los shims muestran decisiones coherentes en PowerShell 5/7, CMD y Bash/Git Bash.
- [ ] Tests de contrato, matriz multi-proyecto, type-check, Sentinel/VarSense y documentación pasan.
- [ ] Existe rollback probado y no quedan rutas hardcodeadas a `C:\Users\...\glory-rust-template` en perfiles globales.

## Fuera de alcance de este plan

- Definir qué comandos y analizadores necesita cada proyecto; eso pertenece a su `sentinel.config.json`.
- Ejecutar automáticamente el gate por el agente; el guard solo impide bypass y recomienda el comando canónico.
- Cambiar reglas de Coolify, deploy o SSH; esas políticas siguen siendo globales y separadas.
- Convertir Sentinel en un editor o reemplazar la extensión/LSP de VarSense; ambos siguen siendo presentaciones/adapters del mismo contrato.
