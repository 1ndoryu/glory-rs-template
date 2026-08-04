# Roadmap Sentinel / VarSense / Quality Gate — wandori.us

> **Fecha:** 2026-08-01  
> **Alcance:** exclusivamente Glory Sentinel, VarSense y la migración del orquestador `scripts/quality` hacia Sentinel.
> **Fuera de alcance:** funcionalidades del OS, frontend, backend, comercio, móvil y roadmap principal.  
> **Objetivo:** convertir los hallazgos y scripts nacidos en este proyecto en capacidades agnósticas, rápidas, portables y mantenibles para cualquier proyecto, con Sentinel como único plano de control y VarSense como analizador especializado.

> **Estado (018A-43):** mínimo operativo cerrado y verificado; el roadmap principal queda desbloqueado. El commit no es requisito universal: el reporte recuerda cuándo conviene hacer staging/commit/push y cuándo documentar trabajo intermedio o compartido. El gate sí exige prueba y reporte reproducibles.

> **Decisión de alcance:** las fases SNT-02 a SNT-10 que siguen con casillas abiertas son backlog diferido. No se ejecutan como requisito de una tarea del producto mientras el gate mínimo pase, no haya regresión de rendimiento y no aparezca un finding bloqueante real. Las extensiones de reglas, paridad de adapters, benchmarks y publicación upstream quedan para una iteración específica de tooling.

## Prioridad para desbloquear el roadmap principal

Este es el único conjunto que debe ejecutarse antes de continuar con una tarea normal del producto:

- [x] `npm run quality:test` pasa la suite del orquestador.
- [x] `npm run task:check -- <task-id> --fresh` pasa Sentinel, VarSense, stack afectado y documentación.
- [x] El gate diferencia error de herramienta, finding bloqueante, warning e información.
- [x] El reporte conserva detalle en `.quality-reports/` y muestra el recordatorio condicional de commit.
- [x] Sentinel y VarSense están fijados en `quality-tools.json` y no requieren instalación automática para trabajar localmente.

Con este checklist cerrado, las mejoras restantes de este documento son backlog y no deben bloquear el siguiente bloque de `roadmap.md`. Solo vuelven a ser prioridad si aparece un fallo real del gate, una regresión de rendimiento o una tarea del producto que dependa de ellas.

### Backlog diferido deliberadamente

- Paridad formal CLI/LSP/VS Code con fixtures idénticas.
- Concurrencia multi-proceso y matriz CI real del runtime global; la fixture local ya cubre cambio de rama dentro del mismo proceso, aislamiento de identidad CI, refs peligrosas, locks entre namespaces y retención best-effort (`scripts/quality/tests/branch-isolation.integration.test.mjs`, `scripts/quality/tests/report-retention-stage.test.mjs`).
- Invalidación avanzada de índices, benchmarks RSS/tiempo y paralelismo optimizado.
- Reglas de seguridad y arquitectura de baja frecuencia (MFA, permisos client-only, webhooks, rollback optimista).
- Perfiles de tema, referencias circulares y precisión avanzada de VarSense.
- Publicación upstream, reinstalación `.vsix`, changelog, ADR y guía de migración.
- Consolidación de Sentinel como plano único (`SNT-10`), incluida la migración reversible de configuración y la retirada del scheduler duplicado.

## Cómo usar este roadmap

- Los IDs `SNT-*` son identificadores internos de este roadmap; al ejecutar una tarea se les asignará el task ID diario exigido por `AGENTS.md`.
- Una casilla solo se marca con evidencia: fixture, prueba CLI/LSP/VS Code equivalente, reporte y quality gate.
- El core de Sentinel/VarSense no recibe reglas, rutas, nombres de clases, idiomas ni decisiones de wandori.us.
- **Estado actual:** el proyecto todavía separa `sentinel.config.json`, `varsense.config.json`, `quality.config.json` y `quality-tools.json`; son contratos de transición.
- **Destino:** `sentinel.config.json` v2 contiene política, gate, guard, runtime y analyzers; `sentinel.lock.json` fija versiones/hashes. VarSense no crea gate, cooldown ni reporte de cierre propio.
- Mientras exista una regla en scripts locales, el adaptador debe marcarla como puente temporal y registrar su paridad con el core.
- Cada fase termina con revisión SOLID, rendimiento, falsos positivos, seguridad de paths/secretos y compatibilidad Windows/Linux/macOS.

## Estado inicial auditado

### Herramientas fijadas

| Herramienta | Versión/commit fijado | Estado observado |
| --- | --- | --- |
| Glory Sentinel | `0.4.0` / `107be9b61a7ed4676ee89b101ecff4112a039fb2` | CLI JSON versionado, config estricta y reglas portables de boundaries/arquitectura. |
| VarSense | `2.2.0` / `b1aa3f06ffbb96a55dd0156a99eae482f41311b8` | Core/CLI/LSP equivalentes; índice dinámico, `all` y reglas de tokens compartidas. |
| Quality gate | `scripts/quality/*.mjs` | Tiene preflight, lock, cache, redacción, reportes y perfiles; necesita endurecer errores, portabilidad y paralelismo. |

### Hallazgos prioritarios del orquestador

> Este inventario histórico se conserva para trazabilidad. Los puntos sobre
> `runVarsense` y el patch de VarSense fueron resueltos por `varsense all` y el
> commit fijado 2.2.0; los hallazgos restantes se mantienen como backlog o
> están cubiertos por los módulos actuales de `scripts/quality`.

- `scripts/quality/adapters/custom.mjs` mantiene `hasErrors = false` y nunca lo actualiza; un script custom puede terminar con violaciones y el stage queda en PASS.
- Los scripts custom se ejecutan con `bash`, dependen de `grep`, `awk`, `sed`, `find` y parsean emojis/salida humana; el comportamiento no es portable ni tiene contrato estructurado.
- `runCustom` convierte en warnings los códigos de salida de reglas que deberían poder bloquear; además no conserva el severity declarado por cada regla.
- El adaptador actual de VarSense invoca `varsense all` una sola vez; `all` comparte el snapshot de documentos para scan, orphan-classes y tokenDetection. `scan` y `orphan-classes` se conservan como comandos de compatibilidad del CLI, no como etapas independientes del gate. La versión fijada 2.2.0 no soporta `--files-from`: el adapter conserva el manifiesto solicitado y reporta `varsense-cli-2.2.0-no-files-from` de forma auditable, sin fallback silencioso ni scheduler paralelo.
- La cache de stages necesita incorporar explícitamente versión/commit de la herramienta, versión del parser, configuración efectiva y plataforma; el hash de archivos por sí solo puede reutilizar un PASS obsoleto.
- `quality-tools.json` conserva únicamente el manifiesto de versiones y el patch local declarado de Sentinel `[317A-3]`; el patch downstream histórico de clases dinámicas de VarSense fue retirado al fijarse el soporte en el commit upstream 2.2.0.
- La detección incremental y los reportes son reutilizables, pero `docs.mjs`, reminders en español, IDs de roadmap y rutas `frontend/src` son políticas del proyecto.

## Frontera de reutilización

### Debe migrarse al core agnóstico

| Procedencia actual | Regla/capacidad reusable | Destino propuesto |
| --- | --- | --- |
| `check-sentinel-extended.sh` | límite de líneas por archivo/módulo | Sentinel static analyzer, umbral configurable por lenguaje y exclusiones declarativas |
| `check-sentinel-extended.sh` | `any`, `@ts-ignore`, `@ts-expect-error` | Sentinel TypeScript/JavaScript AST rule |
| `check-sentinel-extended.sh` | default exports | Sentinel style rule opt-in, nunca política global obligatoria |
| `check-sentinel-extended.sh` | `console.*` en producción | Sentinel rule con allowlist de logger, tests y tooling configurables |
| `check-sentinel-extended.sh` | subscribe sin cleanup | Sentinel lifecycle rule basada en AST y contratos de framework, no regex |
| `check-sentinel-extended.sh` | API fuera de service layer | regla genérica de límites de capas con `layers`/`allowedImports` en configuración |
| `check-sentinel-extended.sh` | imports directos de stores | regla genérica de dependencias prohibidas por capa |
| `check-sentinel-extended.sh` | interfaces grandes | regla ISP configurable por campos y tipos excluidos |
| `check-sentinel-extended.sh` | catch silencioso | regla multi-lenguaje con clasificación de catch vacío, comentario, log-only y propagación |
| `check-sentinel-extended.sh` | módulo con re-export + lógica | regla de barrel/lógica separada |
| `check-sentinel-extended.sh` | export no usado | preferir índice semántico TypeScript; fallback explícito como finding de baja confianza |
| `check-dom-abstraction.sh` | DOM directo fuera del adaptador UI | Sentinel browser-architecture rule con paths de boundary configurables |
| `check-window-refs.sh` | `window.*` fuera de plataforma/navegación | Sentinel platform-boundary rule con APIs permitidas por proyecto |
| `check-singleton-state.sh` | singleton mutable | Sentinel state-architecture rule; detectar instancia, módulo mutable y store global con excepciones justificadas |
| `varsense-class-index.patch` | clases dinámicas en factories/template strings | VarSense `ClassIndexBuilder` del core; eliminar parche downstream |
| `varsense.config.json` | inline Vanilla TS/JS, tokens y clases dinámicas | capacidades generales de VarSense; nombres de clases y exclusiones quedan en config local |

### Debe permanecer específico del proyecto

- Rutas concretas (`frontend/src`, `Agente/`, `roadmap.md`), nombres de clases españolas y excepciones visuales de wandori.us.
- `docs.mjs`: validar task ID, roadmap y planes de `Agente/`; debe quedar como adapter de documentación configurable, no regla de Sentinel.
- `reminders.mjs`: recordatorios del OS, móvil, Coolify y documentación; el motor puede aceptar perfiles, pero el texto no va al core.
- Perfiles `desktop`, `mobile`, `workspace`, `commerce` y sus patrones de este repositorio; el esquema de perfiles sí puede ser agnóstico.
- `quality-tools.json`: manifiesto por proyecto; la estructura de lock/commit/hash es reusable, los repositorios y versiones son datos del consumidor.
- Excepciones como `frontend/src/utils/dom.ts`, clases `desktop-*`, `perfil-redes` o recetas del sistema visual; nunca convertirlas en defaults upstream.
- Checks que conocen `workspaceStore`, `CommandRegistry`, `WindowManager`, `/admin`, Rust/Axum o contratos concretos del OS; solo se generalizan si se modelan como límites configurables.

## Arquitectura objetivo

### Sentinel

Separar claramente cinco capas:

1. **Discovery:** normaliza paths, detecta lenguaje, respeta includes/excludes y protege contra symlinks/rutas fuera del workspace.
2. **Indexación:** parsea cada archivo una sola vez y comparte AST, símbolos, imports, exports, scopes y posiciones entre reglas.
3. **Rule engine:** ejecuta reglas declarativas/AST con contrato común, severidad, confianza, categoría, autofix y cancellation token.
4. **Policy:** aplica configuración del proyecto, severidad, baseline, suppressions justificadas y límites de findings.
5. **Adapters:** CLI JSON, LSP y VS Code consumen el mismo resultado normalizado; ningún analyzer core importa `vscode`.

Contrato mínimo de una regla:

```text
ruleId, language, category, severityDefault, confidence,
sourceRange, message, remediation, safeFix?, docsUrl, analyzerVersion
```

Una regla no ejecuta procesos, no escribe archivos, no imprime salida humana y no conoce nombres de este proyecto.

### VarSense

- Mantener `core` como fuente única para variable index, class index, parser, resolver y análisis documental.
- Construir un índice compartido por snapshot de workspace; `scan` y `orphan-classes` deben consumirlo en una ejecución combinada.
- Mantener CLI, LSP y extensión como adapters finos, equivalentes mediante fixtures.
- Separar parser CSS/valores, indexadores, política de tokens y presentación; ningún core importa VS Code.
- Migrar el parche de clases dinámicas upstream y versionar el contrato de extracción de clases para factories, `className`, objetos y template strings.

### Quality gate

- Mantener el flujo público `npm run task:check -- <task-id>` y un output JSON/Markdown estable.
- Reemplazar el adapter de scripts humanos por invocaciones CLI estructuradas con `outputSchemaVersion`.
- Ejecutar etapas independientes en paralelo con límite de concurrencia y cancelación; mantener orden determinista en el reporte.
- Separar `tool error`, `rule error`, `blocking finding`, `warning` e `information`; nunca degradar un error de infraestructura a warning.
- Cachear por contenido + config efectiva + commit/version de herramienta + parser + plataforma; invalidar por cambio de lock, patch o schema.
- Mantener lock, timeout, redacción, escritura atómica y cleanup como librería reusable del orquestador, no como lógica duplicada por adapter.

## Fases y checklist

### SNT-01 — Baseline, contratos y seguridad del gate

**Objetivo:** congelar el comportamiento actual y eliminar PASS falsos antes de añadir reglas.

- [x] Reproducir `runCustom` con una fixture que falle y corregir la propagación de `hasErrors`/exit code mediante `custom-rules.mjs`.
- [x] Definir contrato estructurado para el bridge custom y retirar Bash/grep del camino normal; los scripts legacy quedan como referencia histórica.
- [x] Diferenciar severity declarada, código de herramienta, timeout, crash y finding bloqueante en `common.mjs`/reporter.
- [x] Exponer y probar la decisión local de política (`no-policy`, `legacy-v1`, `observe`, `enforce`, `pass-through`, `invalid-policy`) en guard/doctor/reporte mediante `scripts/quality/policy-decision.mjs`; la matriz global de shells/runtime continúa pendiente.
- [x] Añadir pruebas de regresión para custom con error, warning, información y salida estructurada.
- [x] Verificar que logs/reportes redaccionan secretos y credenciales sin truncar el diagnóstico esencial; `redaction.test.mjs` cubre token, bearer y password.
- [x] Ejecutar `npm run quality:test` y `task:check` full; baseline actual: 23 tests de quality, gate PASS en 52s, 38 archivos y reportes JSON/Markdown.
- [x] Validar `lockWaitMs` como entero seguro; el comando público usa `0` y falla de forma determinista si el task ya está ocupado.

**Gate:** ninguna regla que falle puede producir PASS; todos los resultados tienen schema, severity y causa distinguibles.

### SNT-02 — Sustituir shell scripts por analyzers portables

**Objetivo:** retirar dependencia de Bash/grep/awk/sed y mover la semántica al core de Sentinel.

- [x] Diseñar fixtures negativos/positivos para las reglas portables nuevas (`portableRules.test.ts`, `portableConfig.test.ts`); las equivalencias legacy completas quedan como siguiente fixture de migración.
- [x] Implementar en el core reglas portables para console, catches, interfaces, barrels, proceso shell, boundaries DOM/window, servicios y estado singleton; las heurísticas usan severidad configurable.
- [x] Implementar límites de capas/boundaries mediante configuración portable; el core no conoce rutas de wandori.us.
- [x] Añadir severidad, paths de boundary y exclusiones declarativas en `sentinel.config.json`.
- [ ] Ejecutar la misma fixture en CLI, LSP y VS Code; comparar JSON normalizado, líneas, columnas, severity y regla.
- [ ] Mantener los scripts como bridge de comparación durante una sola fase; borrarlos solo cuando la paridad esté demostrada.

**Gate:** Sentinel cubre todas las reglas portables, funciona sin Bash en Windows/Linux/macOS y la salida coincide con fixtures de equivalencia.

### SNT-03 — VarSense upstream y análisis de una sola pasada

**Objetivo:** eliminar el parche local y reducir el trabajo duplicado del escaneo CSS/TS.

- [x] Llevar la extracción de clases dinámicas al commit upstream fijado mediante fixtures reproducibles; retirar el patch downstream del manifest.
- [x] Implementar `varsense all`, que comparte provider/snapshot de documentos para `VariableIndex` y `ClassIndex` en una ejecución.
- [ ] Invalidar índices por archivo y dependencias, no por workspace completo; cancelar trabajo obsoleto cuando cambia el documento.
- [ ] Añadir fixtures para clases estáticas, template strings, objetos `className`, factories, multilinea y falsos positivos.
- [x] Garantizar paridad CLI/LSP/VS Code con suite upstream (45 pruebas, smoke LSP y check-core).
- [x] Eliminar el parche de `quality-tools.json`; VarSense queda fijado en `b1aa3f06...`.

**Gate:** una ejecución comparte índices, conserva los hallazgos actuales y mejora tiempo/memoria frente al baseline.

### SNT-04 — Motor SOLID de reglas y adaptadores

**Objetivo:** hacer que añadir una regla no obligue a modificar analyzers, CLI, LSP y extensión por separado.

- [x] Mantener registry tipado de reglas y ampliar findings con confidence, remediation y analyzerVersion.
- [ ] Separar `RuleContext`, `Finding`, `Fix`, `Policy` y `Report`; aplicar DIP entre engine y parser/indexer.
- [ ] Eliminar condicionales globales por proyecto/framework; usar profiles/capabilities declarativos.
- [ ] Definir límites de tamaño para archivos, analizadores, adapters y servicios; dividir módulos antes de superar el límite.
- [x] Añadir cancellation y concurrencia acotada de stages (configurable 1–4, default 1), con duración y conteos en reportes.
- [ ] Prohibir imports editor-specific en `core`, con check automático en CI para Sentinel y VarSense.

**Gate:** una regla de prueba se registra una sola vez y aparece de forma equivalente en CLI/LSP/VS Code sin tocar adapters existentes.

### SNT-05 — Cache, incrementalidad y rendimiento

**Objetivo:** reducir tiempo de feedback sin sacrificar determinismo ni seguridad.

- [x] Definir fingerprint completo: contenido, config efectiva, tool commit, parser/runtime, OS, Node y dependencias locales importadas.
- [x] Compartir snapshot de documentos de VarSense entre análisis relacionados e invalidar dependencias locales en el fingerprint.
- [x] Ejecutar stages con runner acotado y backpressure; el default serial protege equipos de agentes compartidos.
- [x] Cancelar procesos hijos y workers en timeout/interrupción; pruebas de timeout/cancelación pasan.
- [x] Añadir presupuesto de timeout y reportar cache hit/miss; el presupuesto RSS comparativo queda pendiente del benchmark upstream.
- [x] Escribir cache/reportes de forma atómica y resistente a escrituras concurrentes; `atomic-file.test.mjs` confirma que nunca queda JSON parcial.

#### SNT-05A — Rendimiento local completado en 018A-4

- [x] Mantener Vitest serial por defecto (`maxWorkers=1`, sin paralelismo de archivos); la suite completa queda explícita en `test:full`.
- [x] Añadir `test:changed` como selección segura: un grafo local de imports ejecuta solo los tests que dependen del código cambiado; configuración, borrados y renombres fuerzan suite completa; untracked fuente entra en el grafo y un cambio sin test dependiente no consume workers innecesarios.
- [x] Corregir la carrera Windows de `writeAtomic`: dos escritores que compiten por el mismo reporte reintentan `EPERM/EBUSY/EEXIST` sin propagar un falso fallo ni borrar rutas ajenas.
- [x] Añadir captura máxima de 64 KiB por stdout/stderr del runner para evitar crecimiento de memoria; el reporte conserva un marcador visible para solicitar el log original cuando el adapter lo soporte.
- [x] Versionar fingerprint de caché con Node, plataforma, arquitectura, configuración, manifiesto de herramientas y archivos del alcance; añadir prueba de hit/miss por cambio de contenido.
- [x] Incluir borrados/renombres en detección de alcance y forzar invalidación full ante cambios ambiguos.
- [x] Añadir pruebas de lock fail-fast, scope/globs, caché y captura ruidosa; confirmar `npm run quality:test` (17/17) y `task:check -- 297A-19 --fresh` PASS.

**Gate SNT-05A:** cerrado. La ejecución local ya no dispara automáticamente workers múltiples ni una suite completa por cada archivo fuente; el grafo de imports selecciona dependencias y `test:full` conserva la revisión total explícita. El benchmark comparativo de Sentinel/VarSense y el índice compartido quedan pendientes de SNT-03/SNT-05.

#### SNT-05B — Quality gate local ligero (028A-2)

El reporte `297A-49` tardó 533833 ms: Rust consumió 483037 ms (90,5 %) y expiró durante `cargo test` sobre un target frío. Sentinel (9881 ms), VarSense (13201 ms) y frontend (15874 ms) no fueron el cuello de botella. La política cambia el alcance por defecto sin eliminar la suite completa:

- [x] Evitar que cambios de `package.json` o `package-lock.json` fuercen un full Rust; solo los archivos de infraestructura que cambian el contrato del gate mantienen esa invalidación.
- [x] En Rust local ejecutar únicamente `cargo fmt --check` y `cargo check`; reservar `cargo clippy` y `cargo test` para `npm run task:check -- <ID> --full` o `--ci`.
- [x] Mantener `npm --prefix frontend run test` como selector seguro (`test:changed`) y conservar `test:full` como orden explícita.
- [x] Separar fingerprints de caché para `local-light`, `full` y `ci`, y mostrar en el reporte qué cobertura se ejecutó.
- [x] Emitir un recordatorio contextual con el comando `--full` cuando una tarea tocó Rust en modo ligero; no presentar ese resultado como cobertura completa.
- [x] Aplicar cooldown configurable de 180 minutos a `--full`, `cargo test`, `cargo clippy` y `cargo bench`, con lock de una sola ejecución y override explícito auditado.
- [x] Interceptar Cargo desde `run-with-db` y el shim global `cargo.cmd`; un full bloqueado degrada a `local-light` y conserva el motivo en JSON/Markdown.
- [x] Limpiar `C:\tmp\glory-target` con cuota de 15 GB/retención de 7 días, validación de raíz y preservación de targets con proceso activo; la limpieza inicial liberó aproximadamente 29 GB.
- [x] Particionar `.quality-reports` por `projectRoot/branch-key/task-id`, incluyendo reportes Markdown/JSON, logs, caché y locks; implementar `branch-key-v1` con `canonicalRef` UTF-8, NFC, SHA-256 hexadecimal y encoding allowlisted para ramas normales, detached HEAD y refs CI sin permitir traversal (`scripts/quality/branch-identity.mjs`).
- [x] Añadir retención específica de reportes (defaults: 7 días, 512 MiB por workspace y 128 MiB por rama), contando reportes/logs/tool reports/caché/locks; marcar `overQuota` sin borrar la rama activa, podar históricos/tareas/caché elegibles, respetar locks/temporales/escrituras recientes, eliminar locks huérfanos solo tras TTL + PID inactivo, y mantener poda/errores fuera del exit code (`report-retention.mjs`).
- [x] Exponer `quality:reports:cleanup:dry` y `quality:reports:cleanup`; el modo destructivo requiere `--cleanup --yes`.
- [x] Implementar lectura solo lectura del layout histórico `.quality-reports/<task-id>/`: el namespace canónico gana; el legacy solo se acepta con metadata exacta de rama, sin metadata queda ambiguo; se rechazan traversal, symlinks y JSON canónico corrupto sin fallback (`scripts/quality/report-reader.mjs` + tests). El writer nunca actualiza un alias global; retirar la compatibilidad tras dos versiones sigue pendiente.
- [ ] Activar el interceptor dentro de los perfiles PowerShell solo después de backup y autorización explícita; el instalador por defecto no reescribe perfiles persistentes.
- [ ] Medir en CI/nocturno los tiempos cold/warm de Rust y fijar un presupuesto operativo sin bloquear el feedback local; revisar el target estable compartido antes de cambiar `CARGO_TARGET_DIR`.

**Gate SNT-05B:** el modo local no recompila clippy/tests por cada tarea, ningún full se repite durante el cooldown y el informe deja una ruta reproducible para la suite completa. La suite completa sigue siendo obligatoria al cerrar una fase, cambiar infraestructura Rust, preparar publicación o ejecutar CI; no se ejecuta automáticamente en cada archivo.

#### SNT-05C — Reportes por rama y retención acotada

**Estado local:** implementado y cubierto por fixtures. La lista siguiente conserva
solo los límites que dependen del runtime global o de pruebas multi-proceso reales;
no describe como pendientes los contratos ya activos en `scripts/quality`.

**Objetivo:** evitar que `.quality-reports` crezca indefinidamente y que resultados/cache/locks de una rama contaminen otra.

- [x] Resolver `branch-key` desde la rama Git, una ref CI allowlisted o `detached-<full-sha>`; guardar ref/SHA original en metadata y codificar la clave con límites estrictos.
- [x] Escribir en `.quality-reports/branches/<branch-key>/<task-id>/` los `latest.md/json`, logs y reportes de adapters.
- [x] Mover caché y locks a `.quality-reports/branches/<branch-key>/cache/` y `locks/`; incluir `branch-key`/commit/ref en identidad y fingerprints.
- [x] Leer el layout antiguo solo durante la transición, en modo lectura y con metadata exacta de rama; no se escriben aliases ni symlinks inseguros. La retirada tras dos versiones queda pendiente.
- [x] Configurar TTL y cuota máxima; implementar poda determinista con `--dry-run`, protección de la rama activa, locks activos, procesos/escrituras recientes y límites de workspace.
- [x] Añadir fixtures locales para dos ramas concurrentes en namespaces, cambio de rama dentro del proceso, detached HEAD, identidades CI, nombres peligrosos/largos, symlinks y fallo de poda; la concurrencia multi-proceso/CI real queda ligada al runtime global.

**Gate SNT-05C:** dos ramas producen namespaces y `latest` independientes; un PASS/cache/lock no cruza ramas; la poda libera únicamente candidatos elegibles, respeta TTL/cuota y no borra una ejecución activa ni rutas fuera del workspace. El reporte conserva bytes/candidatos y resultado de poda sin secretos.

**Gate:** benchmark reproducible demuestra mejora; dos ejecuciones iguales producen el mismo JSON ordenado y no reutilizan PASS obsoleto.

### SNT-06 — Reglas de seguridad, contratos y arquitectura

**Objetivo:** cubrir riesgos recurrentes de proyectos web sin convertir políticas de wandori.us en defaults.

- [ ] Añadir reglas configurables para secretos, credenciales en URL/logs, open redirects, input no validado y permisos client-only.
- [ ] Detectar errores enmascarados, `ok: true` tras catch, updates optimistas sin rollback y operaciones críticas sin resultado.
- [ ] Detectar SQL/interpolación peligrosa, procesos con shell, I/O sin manejo, `unwrap`/panic de producción y webhooks no idempotentes donde el lenguaje lo permita.
- [ ] Generalizar arquitectura de capas: UI → contrato → servicio → repository/adaptador, con imports y llamadas prohibidas configurables.
- [ ] Añadir reglas de lifecycle: listeners/subscriptions sin teardown, async stale, AbortSignal ausente y cleanup incompleto.
- [ ] Clasificar findings por confianza para que heurísticas complejas no bloqueen sin evidencia suficiente.

**Gate:** cada regla nueva tiene fixture positivo/negativo, documentación, severity rationale, falso positivo conocido y paridad en los tres adapters.

### SNT-07 — VarSense visual y diseño portable

**Objetivo:** convertir las necesidades visuales de este proyecto en capacidades de tokens reutilizables.

- [ ] Mantener detección de variables no definidas, fallbacks hardcoded, inline styles y propiedades prohibidas como reglas configurables.
- [x] Añadir detección de tokens duplicados y no usados con severidad independiente en `varsense all`; las referencias circulares quedan pendientes por requerir resolver ciclos.
- [ ] Añadir perfiles de tema claro/oscuro y cobertura de roles semánticos sin imponer paleta, idioma o nombres de variables.
- [ ] Detectar clases huérfanas cross-file con índice compartido y excluir únicamente patrones declarados por el consumidor.
- [ ] Separar `autofix` seguro de sugerencia; nunca reescribir CSS masivamente sin preview, diff y rollback.
- [ ] Medir precisión sobre CSS, SCSS, LESS, Vanilla TS/JS y plantillas soportadas; mantener límites de parsing claros.

**Gate:** un proyecto sin diseño 1-bit puede usar VarSense con otra convención de tokens sin cambiar el core.

### SNT-08 — Orquestador portable y CI

**Objetivo:** convertir `scripts/quality` en una librería/adaptador reutilizable, no en una colección de scripts de wandori.us.

- [x] Extraer y probar `runner`, `redaction`, `atomic-file`, `lock`, `cache`, `preflight`, `reporter`, `scope` y stage runner como módulos agnósticos.
- [x] Hacer adapters declarativos por herramienta: `structured-tool.mjs` centraliza executable, args, schema, timeout y error policy.
- [ ] Ejecutar stages independientes en paralelo y conservar el orden canónico solo al consolidar el reporte.
- [x] Mantener `docs` y reminders como adapters del proyecto; el runner no añade reglas de producto al core.
- [x] Definir modo local incremental, modo `--full` y modo CI reproducible; el check no instala ni muta dependencias.
- [ ] Publicar reporte Markdown/JSON, exit codes documentados y artifacts sin secretos; conservar detalle en `.quality-reports/`.
- [ ] Validar ejecución en Windows PowerShell, Git Bash, Linux CI y macOS sin asumir comandos POSIX.
- [x] Proveer selector frontend explícito (`npm --prefix frontend run test:changed`) y conservar `test`/`test:full` como suite completa; no usar `passWithNoTests` para ocultar fallos.

**Gate:** un segundo repositorio puede adoptar el orquestador cambiando solo manifest, profiles, paths y policies.

### SNT-09 — Migración, release y mantenimiento

**Objetivo:** retirar deuda local sin romper consumidores existentes.

- [x] Crear matriz de paridad con ruleId, severidad, fixture, adapters, commits y pendientes.
- [x] Fijar commits, schemas y patch local compatible en `quality-tools.json`; [317A-3] queda hashado y la publicación/remoción upstream se mantiene separada de este workspace.
- [ ] Reinstalar `.vsix` solo después de compilar, probar y autorizar; nunca reiniciar VS Code automáticamente.
- [ ] Eliminar scripts shell y el parche VarSense cuando las equivalencias pasen en CI y el reporte no cambie sin justificación.
- [ ] Versionar migraciones de config, aliases de ruleId y suppressions; no invalidar silenciosamente pipelines existentes.
- [ ] Registrar changelog, ADR, guía de migración y benchmark de cada release.

**Gate:** rollback a la versión anterior funciona, ningún consumidor pierde diagnósticos críticos y el bridge local queda eliminado o con fecha explícita de retiro.

### SNT-10 — Sentinel como plano único de control y migración del gate

**Objetivo:** eliminar la separación conceptual entre un "quality gate" independiente y Sentinel. Sentinel debe ser el único dueño de política, guard, cooldown, scope, caché, ejecución de etapas y reporte; VarSense conserva su core/CLI/LSP, pero entra como analyzer versionado.

**Plan canónico:** `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md`.

- [ ] Definir el contrato `analyze/check/guard/doctor/status` sin romper el CLI `sentinel analyze` actual.
- [ ] Definir el contrato de analyzer: manifest de alcance, configuración efectiva, cancelación, timeout, salida normalizada, métricas y estados de error.
- [x] Diseñar/validar localmente el envelope `sentinel.config.json` v2 y mapear la configuración Sentinel v1 actual mediante migración dry-run; `no-policy/observe/enforce` del guard local están cubiertos, mientras backup/rollback aplicado y runtime global siguen pendientes.
- [x] Crear `sentinel.lock.json` con versión/commit/hash de Sentinel, VarSense y protocolo; preflight verifica `git archive`, checkout limpio y realpath dentro del workspace; el runtime local declara `identitySha256` + `artifactSha256: null` explícito hasta existir runtime global. Locks históricos sin el campo se rechazan y se regeneran con `quality:lock --write`.
- [ ] Exigir `artifactSha256` real para runtime global instalado y completar instalación/rollback sin ejecutar plugins o binarios arbitrarios del repositorio.
- [ ] Extraer scheduler, cooldown, locks, scope, caché y reporter desde `scripts/quality` al runtime de Sentinel; conservar `task:check` como alias temporal.
- [ ] Integrar VarSense por adapter CLI JSON/JSONL con `files-from`; sus comandos editoriales CLI/LSP no pueden cerrar el gate ni crear un reporte paralelo.
- [ ] Emitir leases efímeros para que `sentinel check` ejecute herramientas pesadas sin quedar bloqueado por sus propios shims; auditar PID, proyecto, comando, expiración y task ID.
- [ ] Probar enforcement de shells normales y del launcher del agente; documentar explícitamente rutas absolutas y `--noprofile --norc` como límites no interceptables por scripts del repositorio.
- [ ] Ejecutar doble vía en `observe`, comparar findings ordenados y activar `enforce` solo tras paridad y cinco tareas reales dentro del presupuesto.
- [ ] Retirar gradualmente `quality-command-guard`, `global-cargo-guard`, wrappers y scripts duplicados cuando dos releases consecutivos permitan rollback.

**Gate SNT-10:** `sentinel check` es la única autoridad de cierre; VarSense aparece como etapa/analyzer dentro del reporte combinado; ningún proyecto sin política queda bloqueado; la migración de configuración es reversible y la matriz multi-shell/multi-proyecto pasa.

## Nuevas reglas propuestas por prioridad

### Bloqueantes (P0)

- `quality-tool-error-propagation`: timeout/crash/schema inválido nunca puede producir PASS.
- `hardcoded-secret-context`: ampliar detección a URLs, logs, JSON de configuración y archivos temporales sin exponer el secreto en el reporte.
- `unsafe-process-shell`: detectar `shell: true`, comandos concatenados y argumentos no separados.
- `private-route-client-only`: detectar rutas protegidas cuya autorización solo existe en UI/guard cliente; requiere configuración de boundary.
- `error-enmascarado`: detectar éxito sintético después de catch o fallback vacío en operaciones críticas.

### Alta prioridad (P1)

- `layer-boundary-import`: imports/calls fuera de capas permitidas.
- `async-stale-without-abort`: fetch/listener async sin AbortSignal o cleanup verificable.
- `subscription-without-dispose`: subscribe/observer/event listener sin retorno de cleanup.
- `optimistic-update-without-rollback`: estado mutado antes de confirmación sin reversión.
- `api-call-outside-service`: configurable para fetch/client SDK fuera de adapters/services.
- `dom-access-outside-platform`: `document`/`window` directo fuera del boundary declarado.
- `singleton-mutable-state`: instancias globales mutables sin contrato de ciclo de vida.
- `large-interface-isp`: interfaces/types por encima del umbral con propuesta de composición.
- `mixed-barrel-logic`: barrel que exporta y contiene lógica ejecutable.

### Calidad visual y mantenibilidad (P2)

- `unused-export-confidence`: export no usado con índice semántico y clasificación de barrel/public API.
- `token-duplicate`: tokens equivalentes con nombres distintos.
- `token-unused`: variables declaradas sin uso, excluyendo contratos públicos declarados.
- `theme-role-missing`: tema configurado sin roles semánticos equivalentes.
- `class-index-dynamic-confidence`: clase dinámica no indexable, separando warning de falso positivo.
- `file-size-budget`: presupuesto por lenguaje/módulo y no un único umbral rígido.

## Pruebas y evidencia obligatoria

- [ ] Fixtures de equivalencia para cada regla: fuente, expected JSON, severity, línea/columna, mensaje estable y falso positivo.
- [ ] Mismos fixtures ejecutados por CLI, LSP y VS Code; diferencias solo en transporte/presentación.
- [x] Tests de config estricta: claves desconocidas, rutas fuera del workspace, modos inválidos y políticas symlink/junction en loader y guard; globs peligrosos, severity/ruleId del analyzer y paridad upstream quedan pendientes del contrato Sentinel Core.
- [ ] Tests de seguridad: secretos redacted, symlink/path traversal, shell injection, timeout, cancelación y procesos huérfanos.
- [ ] Ejecutar `npm run __sentinel_guard_probe__`: el guard debe devolver `BLOQUEADO` sin invocar npm; si aparece "Missing script", la shell/launcher está sin interceptor y no se puede cerrar la cobertura global.
- [ ] Tests de cache: hit válido, cambio de contenido, config, commit, parser, schema y plataforma.
- [ ] Benchmarks small/medium/full con límite de memoria, tiempo, concurrencia y cantidad de findings.
- [ ] Tests de reporte: máximo de hallazgos/reminders, detalle completo en artifact, salida determinista y exit code.
- [ ] Pruebas de migración: versión anterior, versión nueva, rollback y supresión documentada.

## Definition of Done global

- [ ] Toda regla portátil dejó de depender de Bash, regex frágil o rutas de wandori.us.
- [ ] Sentinel y VarSense mantienen core editor-agnóstico y paridad CLI/LSP/VS Code.
- [ ] El quality gate propaga errores reales, no duplica análisis y tiene cache/versionado correcto.
- [ ] Las configuraciones del proyecto contienen solo política, paths y excepciones locales.
- [ ] Cada regla tiene fixture, documentación, severity, confianza, remediation y criterio de retiro.
- [ ] Windows/Linux/macOS y CI producen reportes equivalentes.
- [ ] Sentinel/VarSense pasan sus propios checks SOLID, rendimiento, seguridad y calidad.
- [ ] `roadmap-sentinel.md` se actualiza como fuente única de esta iniciativa; el roadmap principal no se modifica desde este bloque.

## Comandos de cierre por bloque

```text
npm run quality:test
npm run task:check -- <task-id-real>
# Durante SNT-10, el equivalente canónico será:
sentinel check <task-id-real>
```

Para cambios en los repositorios upstream, además:

```text
npm test
npm run compile
npm run check:core
```

El empaquetado `.vsix`, instalación y cualquier cambio de extensión requieren validación completa y autorización aplicable. No se reinicia VS Code automáticamente.
