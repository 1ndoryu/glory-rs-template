# Plan 028A-6 — Sentinel como plano global de calidad agnóstico

> **Fecha:** 2026-08-02
> **Estado:** propuesto; no implementar hasta revisar este diseño.
> **Motivación:** el guard actual depende de `scripts/quality` dentro de este repositorio. Al cambiar de rama o de proyecto no debe desaparecer, bloquear comandos legítimos ni ejecutar reglas de wandori.us fuera de su alcance.

## Decisión arquitectónica corregida

No habrá un tercer producto llamado `GloryQuality`. **Sentinel será el único plano de control y orquestación de calidad**; VarSense seguirá siendo un analizador especializado que Sentinel ejecuta mediante un contrato de plugin.

1. **Sentinel Core/CLI global:** runtime agnóstico instalado fuera de los repositorios. Resuelve la política, intercepta comandos, aplica cooldown/locks, calcula alcance incremental, orquesta etapas, administra cachés y genera el reporte.
2. **Analizadores:** reglas nativas de Sentinel, VarSense para tokens/clases CSS y futuros analizadores. Cada analizador conserva su especialidad, pero no decide por separado el gate, el cooldown ni el reporte final.
3. **Política local:** configuración declarativa versionada por proyecto y rama. Define qué comandos, analizadores y perfiles aplican; nunca contiene código ejecutable.

El nombre público será `sentinel` (`sentinel check`, `sentinel guard`, `sentinel doctor`, `sentinel status`). `task:check`, `quality-command-guard`, `global-cargo-guard` y los scripts actuales serán adaptadores de migración y después se retirarán. VarSense podrá seguir teniendo CLI/LSP propios para uso de editor, pero en el flujo de agentes su única autoridad de cierre será Sentinel.

### Contrato de responsabilidades

| Capacidad | Responsable único |
|---|---|
| Política por repositorio/rama | Sentinel Core |
| Intercepción de `cargo`, `npm`, `npx`, `rustfmt`, etc. | Sentinel Guard |
| Cooldown de 3 horas, locks y cuota de targets | Sentinel Scheduler |
| Scope incremental y caché de etapas | Sentinel Orchestrator |
| Reglas de variables/clases CSS | VarSense Analyzer, invocado por Sentinel |
| Reglas generales de código | Sentinel Analyzers |
| Reporte Markdown/JSON y exit code | Sentinel Reporter |
| Configuración específica del proyecto | `sentinel.config.json` |

Así se mantienen dos herramientas reales —Sentinel y VarSense—, no tres controles superpuestos.

## Estado real que condiciona el diseño

- Sentinel `0.4.x` ya tiene CLI `analyze`, `--files-from`, configuración estricta y adapters de CLI/LSP/VS Code, pero todavía no es un scheduler ni un interceptor global.
- VarSense `2.2.x` ya tiene core/CLI/LSP y el comando `all`; su ejecución no debe ganar una segunda política de cooldown ni un reporte de cierre independiente.
- El scheduler actual (`scripts/quality`) es específico de wandori.us: decide alcance, caché, cooldown, Rust/frontend, documentación y reporte. Se migrará por etapas; no se copiará entero dentro de Sentinel de una sola vez.
- La primera implementación debe conservar `sentinel analyze` para no romper editores. `sentinel check` será el orquestador; `sentinel guard` solo decide si un comando directo está permitido; `sentinel doctor/status` diagnostican instalación y política.

### Taxonomía pública de comandos

| Comando | Responsabilidad | ¿Ejecuta analizadores? |
|---|---|---|
| `sentinel analyze` | Análisis de archivos/workspace y salida normalizada | Sí, uno o varios analyzers |
| `sentinel check <task-id>` | Gate completo/incremental, etapas, caché, reporte y exit code | Sí, mediante el orquestador |
| `sentinel guard <command>` | Interceptar una validación directa y recomendar el gate | No |
| `sentinel doctor/status` | Diagnosticar política, versión, shims, locks y cachés | No |
| `sentinel install/update/rollback` | Gestionar el runtime global versionado | No |

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
    "sentinel": { "profile": "project-default" },
    "varsense": { "enabled": true, "config": "varsense.config.json" }
  },
  "allow": ["dev", "preview", "codegen", "quality:*"]
}
```

### Versionado y migración de configuración

El proyecto ya usa `sentinel.config.json` v1 para reglas, includes, excludes y boundaries del analizador. No se puede reutilizar ese nombre introduciendo `gate` y `guard` sin contrato de migración.

- [ ] Definir `sentinel.config.json` v2 como un envelope con secciones `policy`, `gate`, `guard`, `runtime`, `analyzers.sentinel` y `analyzers.varsense`.
- [ ] Mapear automáticamente la configuración v1 actual a `analyzers.sentinel` sin cambiar severidades ni patrones; unknown keys deben fallar en `sentinel doctor`, no ignorarse.
- [ ] Migrar `quality.config.json` (timeouts, perfiles, cooldown, presupuestos), `varsense.config.json` y `quality-tools.json` mediante un comando `sentinel doctor --migrate --dry-run` antes de escribir.
- [ ] Crear `sentinel.lock.json` para fijar runtime, protocolo, versión/commit/hash de Sentinel y VarSense; la política no puede descargar ni ejecutar versiones arbitrarias.
- [ ] Usar un formato mínimo estable para el lock: `schemaVersion`, runtime `{version, commit, sha256}`, analyzers `{version, protocolVersion, sha256}` y fecha de generación; nunca guardar secretos.
- [ ] Definir precedencia: defaults del runtime < configuración del proyecto < perfil explícito de CI; variables de entorno solo pueden seleccionar un perfil allowlisted, nunca cambiar severidades o saltarse enforcement.
- [ ] Mantener lectura de los formatos anteriores durante dos versiones de runtime, con warning visible y fecha de retirada.

**Gate:** una migración dry-run no modifica archivos; la migración aplicada es reversible, conserva un hash de la configuración efectiva y dos versiones consecutivas producen el mismo conjunto de findings.

### Resolución de política

- [ ] Buscar desde el directorio actual hacia arriba hasta la raíz del workspace.
- [ ] Usar únicamente `sentinel.config.json` como fuente canónica; no inferir reglas leyendo `AGENTS.md` ni scripts arbitrarios.
- [ ] Canonicalizar la ruta antes de leerla y rechazar rutas fuera del workspace.
- [ ] Calcular `policyHash` y asociarlo al estado; un cambio de rama o política nunca reutiliza una decisión cacheada de otra política.
- [ ] Si no existe política: `pass-through` silencioso para permitir trabajar en cualquier proyecto.
- [ ] Si existe una política inválida: no bloquear comandos desconocidos; mostrar una advertencia concisa y hacer fallar `sentinel doctor`/CI para que el proyecto corrija su configuración.
- [ ] Si `mode` es `observe`: registrar el hallazgo y mostrar la recomendación, pero no impedir la ejecución.
- [ ] Si `mode` es `enforce`: bloquear únicamente las clases declaradas y devolver código no cero.
- [ ] VarSense no crea cooldown, lock ni reporte paralelo: Sentinel le entrega el manifiesto de archivos y recoge sus hallazgos con el contrato de analizador.

## Arquitectura por fases

### Fase 0 — ADR, contratos y compatibilidad

- [ ] Crear ADR con Sentinel Core, el contrato de analizadores (incluido VarSense), la política local y la matriz `enforce/observe/pass-through`.
- [ ] Definir JSON Schema versionado de `sentinel.config.json`, errores allowlisted y límites de tamaño/profundidad.
- [ ] Definir contrato de salida estable: `decision`, `projectRoot`, `policyPath`, `policyHash`, `reason`, `recommendedCommand`, `exitCode`.
- [ ] Definir contrato de plugin, taxonomía `analyze/check/guard/doctor` y matriz de compatibilidad Sentinel↔VarSense.
- [ ] Definir compatibilidad Windows PowerShell 5/7, PowerShell Core, CMD, Bash/Git Bash (interactivo y `BASH_ENV`) y CI sin depender de variables específicas de VS Code.
- [ ] Definir política de actualización, rollback y migración desde el guard actual.

**Gate:** ADR aprobado, schema con fixtures válidos/ inválidos y contrato de salida revisado.

### Fase 1 — Sentinel Core global instalable y estable

- [ ] Extraer el clasificador, scheduler, scope, caché y reporter a Sentinel Core, sin imports de wandori.us ni de VarSense.
- [ ] Crear CLI global `sentinel check|guard|doctor|status|install|update|rollback`.
- [ ] Instalar versiones en `%LOCALAPPDATA%\GlorySentinel\versions` y cambiar `current` de forma atómica.
- [ ] Ejecutar analyzers mediante adapters aislados con timeout, cancelación, límite de salida y estados `tool-error/timeout/invalid-output`.
- [ ] Generar shims con resolución del ejecutable real sin recursión; preservar argumentos, códigos de salida y redirecciones.
- [ ] Dot-sourcear únicamente la ruta global estable en ambos perfiles; crear backup antes de cualquier modificación.
- [ ] Mantener los wrappers del repositorio solo como adaptadores para desarrollo, no como dependencia del perfil global.

**Gate:** una rama que elimina `scripts/quality` no rompe el perfil ni el CLI global; `doctor` identifica la versión activa y el ejecutable real.

### Fase 2 — Resolución por workspace y rama

- [ ] Implementar descubrimiento de raíz y política en cada comando, sin estado de proceso que sobreviva al cambio de rama.
- [ ] Diferenciar `no-policy`, `observe`, `enforce` y `invalid-policy` en el resultado y el reporte.
- [ ] Invalidar decisiones/cooldowns por `projectRoot + policyHash + runtimeVersion`.
- [ ] Mantener cooldown/locks solo para comandos declarados como pesados por la política; no compartirlos entre proyectos.
- [ ] Emitir leases efímeros firmados para que los procesos hijos iniciados por `sentinel check` puedan usar herramientas pesadas sin que el propio shim los bloquee; el lease debe estar ligado a PID, proyecto, comando, expiración y task ID.
- [ ] Definir la frontera de enforcement: shims cubren shells normales; el launcher del agente/CI debe invocar `sentinel guard` antes de ejecutar procesos. Rutas absolutas y shells `--noprofile --norc` se registran como bypass no interceptable por un script de proyecto, no se presentan como cobertura completa.
- [ ] Añadir `sentinel doctor` con diagnóstico de raíz, política, hash, modo, shims, PATH y comando recomendado.

**Gate:** matriz con dos proyectos y dos ramas: el proyecto configurado bloquea lo declarado; el proyecto sin política pasa; cambiar de rama actualiza la decisión sin reiniciar el editor.

### Fase 3 — Adaptador de wandori.us y VarSense

- [ ] Añadir `sentinel.config.json` al proyecto con `sentinel check -- <TareaId>` como gate; conservar un alias temporal para `npm run task:check`.
- [ ] Migrar `quality-command-guard.mjs`, `global-cargo-guard.ps1`, `npm.cmd`, `npx.cmd` y `cargo.cmd` al runtime global de Sentinel sin duplicar reglas.
- [ ] Mantener `quality.config.json` solo para la transición de tiempos, alcance y cachés; la política de comandos y analizadores vive en Sentinel.
- [ ] Integrar VarSense como adaptador de analizador (`files-from`, hallazgos tipados, caché e invalidación), sin un gate ni scheduler propio.
- [ ] Actualizar `quality:install-guard` para instalar/copiar Sentinel y retirar rutas hardcodeadas del repositorio.
- [ ] Ejecutar VarSense desde Sentinel y demostrar paridad de hallazgos con su CLI/LSP, sin permitir que VarSense cierre la tarea por separado.
- [ ] Ejecutar primero en modo `observe` contra el gate actual y comparar reportes normalizados; activar `enforce` solo después de resolver diferencias, errores de herramienta y falsos positivos.
- [ ] Mantener compatibilidad temporal con el guard actual y emitir advertencia de migración, sin bloquear una rama antigua.

**Gate:** wandori.us bloquea `npx vitest`, `npm run test:*`, type-check/lint/build y Cargo directo; `task:check`, `quality:*`, desarrollo y proyectos externos siguen funcionando.

### Fase 4 — Integración multi-proyecto y CI

- [ ] Crear fixtures de un proyecto Node, Rust, Python y un proyecto sin política.
- [ ] Probar `npm`, `npx`, `cargo`, `rustfmt`, comandos directos, `2>&1`, pipes y códigos de salida en PowerShell 5/7, CMD y Bash/Git Bash.
- [ ] Probar rutas anidadas, junctions/symlinks permitidos, repositorio movido y checkout de ramas con/sin política.
- [ ] CI usará la política del proyecto y el runtime fijado; nunca dependerá del perfil del desarrollador.
- [ ] Probar agentes con PowerShell/Bash/CMD, procesos hijos, pipes, `2>&1`, shell sin perfil y rutas absolutas; cada caso debe indicar si se bloquea, se observa o requiere enforcement del launcher.
- [ ] Publicar reportes compactos sin secretos y con máximo tres hallazgos/máximo cuatro recordatorios.

**Gate:** 100% de fixtures con decisión esperada, sin bloqueo cruzado entre proyectos y sin proceso huérfano.

### Fase 5 — Retirada segura del acoplamiento actual

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

## Definition of Done

- [ ] El runtime global de Sentinel no depende de una rama ni de archivos del repositorio actual.
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
