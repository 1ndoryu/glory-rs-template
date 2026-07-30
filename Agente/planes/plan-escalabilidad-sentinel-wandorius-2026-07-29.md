# Plan detallado: quality gate Sentinel/VarSense por tarea

> **Tarea:** 297A-6  
> **Fecha:** 2026-07-29  
> **Estado:** implementación autorizada; capacidades CLI y distribución completas, runner pendiente
> **Prioridad:** primer bloque técnico; bloquea todo desarrollo posterior  
> **Reglas:** `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`

## Estado de implementación

Completado el 29 de julio de 2026:

- [x] Sentinel `0.4.0`: `--files-from`, ayuda/versión, JSON schema `1`, conteos por severidad y config estricta.
- [x] VarSense `2.2.0`: ayuda/versión, JSON schema `1`, config estricta, Vanilla TS/JS y huérfanos con severidad bloqueante configurable.
- [x] Retiradas del núcleo Sentinel las cinco reglas B&W específicas de wandori.us.
- [x] Configuración local creada en `sentinel.config.json` y `varsense.config.json`.
- [x] Custom properties mediante `style.setProperty('--token', ...)` reconocidas como válidas.
- [x] Commits publicados: Sentinel `440cd48`; VarSense `b299040`.
- [x] Baseline VarSense: 44 archivos, 0 errores y 81 warnings (`73 cssInlineScript`, `8 valorHardcoded`).

Siguiente gate:

- [x] Resolver distribución reproducible y verificación de artefacto/versión sin rutas absolutas.
- [ ] Construir ahora el core del runner y sus adapters.

Distribución implementada:

- `quality-tools.json` fija repositorio, commit, versión, schema y ruta del CLI.
- `npm run quality:setup` clona en `.quality-tools/`, instala, compila y verifica versión; nunca se ejecuta implícitamente desde una revisión.
- Estado y marcadores se escriben atómicamente; una segunda ejecución usa cache verificada.
- Cada proceso tiene timeout de cinco minutos y cancelación del árbol en Windows.
- Auditoría pendiente de dependencias de tooling: npm reporta 11 vulnerabilidades en Sentinel y 26 en VarSense; no se aplicará `audit fix --force` sin revisar compatibilidad.

## 1. Resultado esperado

Al terminar cualquier tarea, incluso un agente con poco contexto debe ejecutar una sola instrucción:

```text
npm run task:check -- 297A-N
```

El comando debe:

1. Encontrar automáticamente qué cambió.
2. Elegir Sentinel, VarSense y validaciones del stack necesarias.
3. Ejecutarlas en un orden estable y con timeout.
4. Mostrar solo los errores más accionables.
5. Guardar el detalle completo fuera del contexto de conversación.
6. Mostrar como máximo cuatro recordatorios breves y contextuales.
7. Decir exactamente qué corregir y qué comando repetir.
8. Devolver un exit code fiable para agente, self-check y CI.

### Frontera obligatoria herramienta/proyecto

Sentinel y VarSense son productos agnósticos. Su núcleo solo contiene mecanismos y reglas reutilizables por distintos repositorios. Wandori.us solo aporta configuración, severidades, allowlists, exclusiones y fixtures de integración.

| Pertenece a Sentinel/VarSense | Pertenece a wandori.us |
|---|---|
| Parsers, índices, schemas y formato de hallazgos | `sentinel.config.json` y `varsense.config.json` |
| Reglas genéricas de seguridad/calidad | Reglas habilitadas y severidad efectiva |
| Mecanismos configurables de patrones/políticas | Rutas, excepciones y políticas visuales B&W |
| CLI, LSP y equivalencia entre superficies | Fixtures que representan la arquitectura del OS |
| Validación estricta de cualquier config | Recordatorios y perfiles de riesgo del runner |

Regla de decisión:

- Si una detección puede nombrar `wandori`, una app, una ruta o una decisión visual de este producto, no puede estar hardcodeada en la herramienta.
- Si la necesidad es reutilizable, se implementa como mecanismo genérico parametrizable y se activa aquí mediante config.
- Si no se puede expresar sin lógica específica, permanece como test/check del proyecto; no contamina Sentinel.

El agente no memoriza comandos internos, rutas de herramientas, listas de reglas ni orden de validación.

## 2. Experiencia del agente

### 2.1 Único comando público

```text
npm run task:check -- 297A-9
```

No se exponen flags normales al agente. El script decide incremental/full según contexto:

- Local con cambios: incremental seguro.
- Sin base calculable o cambios de config/migraciones/herramienta: full automático.
- CI: full automático.
- Opción avanzada `--fresh` solo para diagnóstico documentado; no aparece en recordatorios normales.

### 2.2 Salida cuando pasa

Máximo aproximado de 15–20 líneas:

```text
QUALITY 297A-9  PASS  42s
Sentinel   PASS
VarSense   PASS
Rust       PASS
Frontend   PASS

ANTES DE CERRAR
[ ] Probaste el flujo real afectado
[ ] Actualizaste roadmap/completados si corresponde
[ ] Stage explícito, commit, push y relectura del roadmap

Reporte: .quality-reports/297A-9/latest.md
```

### 2.3 Salida cuando falla

Máximo tres hallazgos principales; el resto va al reporte:

```text
QUALITY 297A-9  FAIL  18s
Sentinel   2 errores
VarSense   PASS
Rust       no ejecutado: bloqueo previo

CORRIGE AHORA
1. auth-role-server-only  src/...:42  Falta validar capacidad admin
2. catch-no-vacio        frontend/...:18  El error se silenció

DESPUÉS
npm run task:check -- 297A-9
Detalle: .quality-reports/297A-9/latest.md
```

### 2.4 Salida cuando la herramienta está rota

```text
QUALITY 297A-9  SETUP ERROR
No se encontró Sentinel CLI en la ruta configurada.
Acción: compilar/verificar la herramienta según el reporte.
No corrijas código del producto todavía.
```

Así se distingue una violación real de un fallo del quality gate.

## 3. Recordatorios compactos

El self-check actual imprime una lista larga antes de validar. Se reemplazará por recordatorios seleccionados según riesgo. Nunca mostrar más de cuatro.

### 3.1 Siempre al pasar

Solo tres recordatorios:

1. Probar el flujo real.
2. Actualizar roadmap/completados/documentación aplicable.
3. Stage explícito + commit/push + releer roadmap.

### 3.2 Por archivos modificados

| Perfil detectado | Recordatorio máximo |
|---|---|
| UI/CSS | Revisar navegador y viewports requeridos; usar tokens/componentes existentes |
| Async/router/app | Confirmar AbortSignal/teardown y no respuestas stale |
| Auth/admin | Probar usuario normal/visitante; UI oculta no es autorización |
| SQL/migración | Verificar constraint/default/backfill/rollback y cero interpolación |
| Commerce/upload | Precio/webhook/entitlement server-side; ningún asset privado público |
| Workspace | Probar release/overlay/papelera y conflicto 409 |
| Mobile | Teléfono sin DesktopWindow/barras; tablet conserva desktop |
| Docs/planes | Checklists, dependencias y enlaces canónicos coherentes |

### 3.3 Por hallazgo

Cada rule ID enlaza a una explicación de una o dos frases y a la fuente canónica. No se imprime el manual completo.

Ejemplo:

```text
REMEMBER auth-role-server-only: valida la capacidad en backend; ocultar Admin no protege el endpoint.
```

### 3.4 Anti-ruido

- No imprimir las 18 reglas de AGENTS.
- No copiar stdout completo de Cargo/TypeScript/Sentinel.
- No repetir warnings idénticos.
- Agrupar por regla+archivo y mostrar conteo.
- Mostrar máximo tres hallazgos y cuatro recordatorios; `--debug` manual abre detalle, no es flujo normal.
- Guardar logs completos en reporte, con secretos redactados.

## 4. Arquitectura del orquestador

Estructura objetivo:

```text
scripts/quality/
  task-check.mjs           # coordinador fino
  args.mjs                 # task ID y flags internos
  preflight.mjs            # raíz, binarios, config, git
  scope.mjs                # archivos modificados -> perfiles
  runner.mjs               # spawn directo, timeout, captura
  cache.mjs                # fingerprint y etapas reutilizables
  reminders.mjs            # recordatorios por perfil/rule
  reporter.mjs             # terminal breve + MD/JSON
  redaction.mjs            # secretos/output seguro
  adapters/
    sentinel.mjs
    varsense.mjs
    rust.mjs
    frontend.mjs
    docs.mjs
  tests/
quality.config.json
.quality-reports/          # ignorado por git
```

Límites:

- `task-check.mjs` coordina; no contiene reglas ni parseadores específicos.
- Cada adapter traduce un CLI real a un resultado común.
- `quality.config.json` define paths, profiles, timeouts y severidades del proyecto.
- Sentinel/VarSense siguen siendo dueños de sus reglas.
- El orquestador no importa VS Code ni inicia LSP/editor.

## 5. Contrato interno común

```ts
interface QualityStageResult {
  stage: 'sentinel' | 'varsense' | 'rust' | 'frontend' | 'docs';
  status: 'pass' | 'fail' | 'error' | 'skipped';
  durationMs: number;
  findings: QualityFinding[];
  summary: string;
  logPath?: string;
}

interface QualityFinding {
  ruleId: string;
  severity: 'error' | 'warning' | 'info';
  file?: string;
  line?: number;
  message: string;
  help?: string;
}
```

Todos los adapters producen este contrato. El reporter no interpreta formatos propios de herramientas.

## 6. Detección automática de alcance

El agente no elige checks manualmente.

### 6.1 Fuente de archivos

1. Git diff frente a `--base` interno calculado.
2. Cambios staged + unstaged + archivos nuevos dentro del workspace.
3. En CI, diff del PR y full gate final.
4. Si Git no permite base confiable, full seguro.

Nunca usar `git add`, reset, checkout ni modificar el worktree.

### 6.2 Perfiles por paths

```text
*.rs, Cargo.*                         -> rust
frontend/**/*.ts                     -> frontend
frontend/**/*.css                    -> frontend + css
migrations/**                        -> rust + database
**/auth/**, middleware/auth*          -> security-auth
**/product*, **/order*, **/stripe*   -> security-commerce
features/desktop/**                  -> desktop
workspace/layout/overlay paths       -> workspace
Agente/**, roadmap.md, AGENTS.md     -> docs
sentinel/varsense/config/scripts      -> full quality-tooling
```

Los perfiles agregan checks y recordatorios; nunca desactivan checks obligatorios de otro perfil.

### 6.3 Escalado automático a full

- Cambió `Cargo.toml`, lockfiles, tsconfig, Vite, Orval o config de calidad.
- Cambió una migración.
- Cambió AGENTS/manual de arquitectura/prevención.
- No hay base Git fiable.
- CI o release gate.
- Cache inválida por herramienta/config.

## 7. Pipeline y política de bloqueo

### Etapa 0 — Preflight

- Validar task ID y presencia en roadmap/plan/completados.
- Resolver raíz sin depender de cwd casual.
- Confirmar configs y CLI de Sentinel/VarSense.
- Confirmar Node/npm/cargo solo si aplican.
- Confirmar directorio de reportes escribible.
- No imprimir variables de entorno.
- Spawn con executable+args y `shell:false`; evitar perfiles PowerShell.

Si falla, exit setup y no ejecutar código.

### Etapa 1 — Sentinel

- Changed files en incremental; workspace completo en full.
- Formato JSON estable.
- Errores bloquean; warnings se reportan según rollout/config.
- Crash/output inválido es tool error, no “PASS”.

### Etapa 2 — VarSense

- Solo si hay CSS/TS visual o full.
- Variables inexistentes/clases usadas sin definición bloquean.
- Huérfanos/hardcodes según severidad de rollout.
- No abrir editor ni depender de workspace de VS Code.

### Etapa 3 — Stack

- Rust: format check, check, clippy `-D warnings`, test según alcance/config.
- Frontend: type-check y tests configurados.
- Docs: enlaces locales, planes activos con checklist/dependencia/gate, roadmap solo pendientes.
- Evitar ejecutar la misma validación de nuevo desde self-check.

### Etapa 4 — Reporte y recordatorios

- Agregar resultados.
- Redactar secretos antes de archivo/terminal.
- Escribir JSON primero y Markdown después.
- Imprimir resumen compacto, próximos pasos y ruta.
- Exit code final estable.

### Fail-fast equilibrado

- Preflight roto: detener todo.
- Sentinel con error crítico: puede omitir checks costosos, pero ejecutar análisis barato restante si aporta contexto.
- Stack compile roto: no lanzar tests dependientes.
- Reporte siempre se genera aunque una etapa falle.

## 8. Exit codes

| Código | Significado | Acción del agente |
|---:|---|---|
| 0 | Todo pasó | seguir recordatorios de cierre |
| 1 | Código/calidad falló | corregir primeros hallazgos y repetir mismo comando |
| 2 | Config, herramienta, JSON inválido o timeout | reparar quality gate; no parchear producto |
| 130 | Cancelación del usuario | repetir solo cuando se decida continuar |

El script imprime la acción asociada; el agente no memoriza esta tabla.

## 9. Cache y reanudación

Para no gastar tiempo ni contexto al corregir:

- Fingerprint por etapa: archivos relevantes + config + versión herramienta + comando.
- Etapa PASS se reutiliza solo si su fingerprint no cambió.
- FAIL/ERROR nunca se cachea como éxito.
- Cambiar config, adapter, lockfile o herramienta invalida lo relacionado.
- CI y `--fresh` ignoran cache.
- Terminal indica `PASS (cached)` sin repetir logs.
- Si cambió código, invalidar la etapa relevante y todas las etapas posteriores dependientes.
- Estado/cache/reportes se escriben de forma atómica para no aceptar archivos parciales.
- Cache vive en `.quality-reports/cache/`, ignorada por Git.

## 10. Reportes

```text
.quality-reports/
  297A-9/
    latest.json
    latest.md
    logs/
      sentinel.log
      varsense.log
      rust.log
      frontend.log
```

JSON contiene:

- task ID, fecha, duración, modo y base.
- commit/worktree fingerprint.
- hashes de config y versiones de herramientas.
- etapas y hallazgos normalizados.
- recordatorios seleccionados.
- exit code y acción siguiente.

Markdown contiene resumen humano y detalle agrupado. Los reportes son locales/artefactos CI, no se commitean por defecto.

## 11. Redacción y seguridad del runner

- No enumerar/imprimir env completo.
- Denylist de nombres sensibles y redacción de patrones conocidos.
- Redactar valores de alta entropía en líneas con `TOKEN|KEY|SECRET|PASSWORD|AUTH`.
- Limitar tamaño de línea/log y marcar truncamiento.
- No ejecutar comandos mediante strings/shell; usar executable + args separados.
- No instalar dependencias ni compilar herramientas automáticamente durante preflight.
- No leer navegador, cookies, `.env` ni archivos fuera de lo necesario.
- No realizar staging, commit, push, deploy ni cambios de archivos fuente.
- Fixture específica demuestra que un secreto simulado nunca llega a terminal/reporte.

## 12. Integración con self-check

`scripts/self-check.ps1` actual se reemplaza como fuente de validación:

- Deja de imprimir todas las reglas/proceso en cada ejecución.
- Se convierte en wrapper fino de `npm run task:check -- {ID}` o se retira tras compatibilidad.
- No vuelve a ejecutar `check:back`, `test` y `check:front` si el orquestador ya los ejecutó.
- `roadmap` con pendientes no se trata como error; el docs adapter verifica estructura.
- La guía extensa permanece en AGENTS/manuales, no en stdout.

### Problemas concretos que la migración debe eliminar

- [ ] Hacer obligatorio y validar el Task ID; hoy puede faltar o no corresponder al roadmap.
- [ ] Ejecutar procesos sin cargar perfiles de PowerShell (`-NoProfile`) para evitar efectos laterales y exposición accidental de secretos.
- [ ] Sustituir las 28 casillas estáticas no verificadas por hasta cuatro recordatorios calculados por perfil.
- [ ] Eliminar recordatorios contradictorios sobre SSH; producción solo usa `coolify-manager-rs`.
- [ ] Evitar ejecutar siempre backend, tests y frontend cuando el alcance no los requiere.
- [ ] Añadir `cargo fmt --check` al perfil Rust.
- [ ] No abortar al primer fallo barato: recoger resultados independientes útiles y omitir solo etapas dependientes.
- [ ] Conservar la diferencia entre fallo de calidad, error de infraestructura y cancelación.
- [ ] Añadir timeout, redacción, reporte y reanudación segura.
- [ ] Integrar Sentinel y VarSense en el mismo pipeline, sin una segunda pasada duplicada.
- [ ] Corregir el parser del roadmap para que títulos y texto descriptivo no cuenten como tareas.
- [ ] Pasar a procesos hijos solo un entorno mínimo permitido; nunca heredar secretos innecesarios.
- [ ] Tratar terminación por señal o ausencia de exit code como error, nunca como éxito mediante `code ?? 0`.
- [ ] Actualizar la descripción del paquete raíz para que refleje Vanilla TypeScript, no React.

Compatibilidad temporal:

- [ ] Una fase corta permite ambos comandos, pero ambos llaman al mismo core.
- [ ] Comparar resultados sobre fixture real.
- [ ] Actualizar AGENTS/package/CI.
- [ ] Retirar flujo duplicado en el mismo bloque.

## 13. Integración con Sentinel y VarSense

### Sentinel

- Usar CLI/core oficial y config `sentinel.config.json`.
- Reglas reutilizables viven en repo Sentinel, nunca en el runner.
- Cada regla: fixture positiva/negativa + equivalencia CLI/LSP/editor.
- Mensaje incluye rule ID, por qué, corrección y enlace corto.

### VarSense

- Usar CLI/core oficial y config canónica.
- Consumir archivos CSS/TS relevantes o workspace full.
- Distinguir variable inexistente, hardcode, clase faltante y huérfana.
- Dynamic classes requieren allowlist estrecha/documentada.

### Cuando falta una regla

El runner no intenta compensarla. Registra prevención, implementa la regla en herramienta agnóstica, prueba equivalencia y luego activa config del proyecto.

## 14. Capacidades CLI y distribución requeridas antes del runner

El orquestador solo será sencillo si las herramientas tienen contratos pequeños y fiables. Estas capacidades se implementan y prueban primero; el runner no hará parsing de texto humano ni asumirá detalles internos.

### 14.1 Contrato común obligatorio

- [x] `--help` imprime ayuda y termina con `0`; nunca lanza excepción ni se confunde con error de infraestructura.
- [x] `--version` devuelve versión semántica; el schema viaja en cada salida JSON.
- [x] `--format json` produce JSON válido aun cuando existan hallazgos.
- [x] Toda salida JSON incluye `schemaVersion`, versión de herramienta, alcance analizado, duración y conteo por severidad.
- [x] La config se valida estrictamente; claves desconocidas, tipos inválidos y reglas inexistentes fallan con mensaje concreto.
- [x] El manifest fija commit/versión y el setup valida que el artefacto compilado reporte esa versión.
- [x] Los CLI distinguen hallazgos (`1`) de configuración/infraestructura (`2`); cancelación `130` pertenece al runner pendiente.
- [ ] El adapter cuenta severidades desde JSON y aplica la política canónica; no confía solo en el exit code del CLI.

### 14.2 Glory Sentinel

- [x] Añadir `--files-from <archivo>` para analizar una lista explícita de archivos cambiados.
- [x] Mantener modo workspace completo para CI, migraciones, configuración y cambios sin base segura.
- [x] Incluir `scripts/**` mediante la config local para que Sentinel supervise el futuro runner.
- [ ] Probar equivalencia de hallazgos entre CLI, LSP, VS Code y Zed con las mismas fixtures.
- [ ] Documentar límites de archivo, workspace, exclusiones y códigos de salida como contrato estable.
- [x] Retirar del núcleo las cinco reglas B&W hardcodeadas para wandori.us; su política se traslada a configuración local.
- [ ] Añadir una prueba de arquitectura que impida introducir nombres/rutas de proyectos consumidores en reglas generales.

### 14.3 VarSense

- [x] Soportar consumidores y estilos inline Vanilla `.ts/.js`, además de `.tsx/.jsx`; este proyecto no depende de React.
- [x] Corregir `orphan-classes` para devolver fallo cuando su severidad configurada sea `error`.
- [x] Definir el contrato inicial simple: si cambia CSS o TS visual, analizar el workspace visual completo.
- [ ] Añadir análisis por lista de archivos solo cuando pueda conservar la exactitud entre definición CSS y consumidores TS.
- [ ] Cubrir variables inexistentes, clases faltantes, huérfanas, hardcodes y allowlists dinámicas con fixtures positivas/negativas.
- [x] Crear `varsense.config.json` local con tokens, propiedades y excepciones visuales de wandori.us.

### 14.4 Distribución reproducible local y CI

- [x] No depender en CI de rutas absolutas a repositorios hermanos.
- [x] Fijar repositorio y commit exacto de cada CLI en `quality-tools.json`.
- [x] Registrar versión/schema esperados y verificarlos durante `quality:setup`; el preflight reutilizará el mismo manifest.
- [ ] Permitir rutas locales de desarrollo solo mediante config explícita no commiteada, nunca como valor canónico.
- [x] Separar instalación en `quality:setup`; el futuro runner no descarga, compila ni reinstala herramientas.
- [x] Probar clones limpios desde GitHub dentro de `.quality-tools/`, sin consumir repositorios hermanos.

**Gate de capacidades:** ambos CLI cumplen fixtures de ayuda, versión, schema, configuración, severidad, alcance y artefacto fresco; una instalación limpia los reproduce. Solo entonces comienza el core del runner.

## 15. Rollout de reglas

### Ola 0 — Infraestructura

- Preflight, adapters, reporter, cache, redaction y tests del runner.
- Sin bloquear producto hasta demostrar estabilidad.

### Ola 1 — Seguridad/fallos silenciosos

- Capacidades admin server-side.
- Publicación/visibilidad obligatoria.
- Token no Web Storage.
- DTO/asset privado no público.
- Checkout/webhook/download server-authoritative.
- HTML sanitizado, catch/IO no silencioso y Fetch con resultado.

### Ola 2 — Lifecycle/desktop/móvil

- MountedView async con signal/teardown.
- App solo AppRegistry.
- Chrome/WindowManager/commands centralizados.
- Sin z-index por app/listas paralelas.
- Presentación móvil no monta ventanas/barras desktop ni duplica app/store.

### Ola 3 — API/datos/analytics

- Orval Fetch tags-split/generated protegido.
- SQL en repository y tipado.
- Defaults privados/estados ortogonales.
- Workspace versionado.
- Analytics allowlist/límites/idempotencia/privacidad.

### Ola 4 — Identidad/VarSense

- Tokens y clases coherentes.
- Sin colores/sombras/radios/bordes prohibidos.
- Lucide oficial 1px.
- Sin CSS inline/recetas visuales duplicadas/hover decorativo.

Por ola:

- [ ] Inventariar baseline.
- [ ] Corregir casos reales.
- [ ] Añadir fixtures.
- [ ] Activar regla en warning durante prueba corta.
- [ ] Verificar falsos positivos.
- [ ] Convertir alta confianza a error.
- [ ] Añadir regresión CI.

No se permite congelar deuda indefinidamente como baseline. 297A-6 cierra con cero errores activos.

## 16. Plan de implementación paso a paso

### Fase A — Contrato UX aprobado

- [ ] Auditar comandos/versiones/formato/exit codes actuales de ambos CLI.
- [ ] Auditar self-check/package/CI/config/excepciones.
- [ ] Crear fixtures mínimas limpia/Sentinel/VarSense.
- [ ] Registrar baseline Rust actual, incluyendo errores Clippy conocidos, sin corregir aún.
- [ ] Aprobar el comando único y las salidas PASS/FAIL/SETUP ERROR de máximo 15–20 líneas.
- [ ] Entregar el contrato a un agente sin contexto y comprobar que sabe qué corregir y qué comando ejecutar después.

**Gate A:** usuario aprueba la experiencia antes de implementar.

### Fase B — Capacidades CLI

- [ ] Completar todos los requisitos de la sección 14.1.
- [ ] Implementar y probar `--files-from` y autosupervisión en Sentinel.
- [ ] Implementar soporte Vanilla TS y exit codes correctos en VarSense.
- [ ] Probar fixtures equivalentes CLI/LSP/editor.

**Gate B:** Sentinel y VarSense ofrecen contratos estables; no se necesita parsing heurístico.

### Fase C — Distribución y configuración

- [ ] Elegir distribución reproducible y fijar versiones compatibles.
- [ ] Crear configs canónicas validadas por schema.
- [ ] Verificar artefactos frescos por versión/hash.
- [ ] Probar instalación limpia local/CI sin rutas absolutas hermanas.

**Gate C:** el preflight obtiene las mismas herramientas y reglas en cualquier entorno soportado.

### Fase D — Core del runner

- [ ] Crear args/preflight/scope/runner/result types.
- [ ] Spawn directo multiplataforma con timeout/cancelación.
- [ ] Implementar redaction y límites de output.
- [ ] Tests unitarios de args, scope, timeout y redaction.

**Gate D:** runner ejecuta herramienta fake limpia/fallida/colgada sin perder control.

### Fase E — Adapters

- [ ] Sentinel JSON adapter.
- [ ] VarSense adapter.
- [ ] Rust adapter.
- [ ] Frontend adapter.
- [ ] Docs adapter.
- [ ] Normalizar status/findings/help.
- [ ] Contar severidades desde JSON y aplicar política de bloqueo configurada.

**Gate E:** cada adapter distingue pass/fail/error y produce contrato común versionado.

### Fase F — Reporter, reminders y cache

- [ ] Terminal PASS/FAIL/SETUP ERROR dentro de límites.
- [ ] Markdown/JSON y logs por etapa.
- [ ] Selección de máximo cuatro recordatorios por perfil/rule.
- [ ] Fingerprints e invalidación segura.
- [ ] Escritura atómica de estado/reporte para que una cancelación no deje un PASS falso.
- [ ] Próximo comando exacto en cada salida.

**Gate F:** un agente puede actuar viendo solo stdout; detalle queda disponible sin inundar contexto.

### Fase G — Integración local/self-check

- [ ] Script npm `task:check`.
- [ ] `.gitignore` reportes/cache.
- [ ] Wrapper/migración de self-check sin duplicación.
- [ ] Ejecutar sobre tarea docs, TS/CSS y Rust reales.
- [ ] Corregir fallos del runner antes de reglas nuevas.

**Gate G:** un comando funciona desde raíz y subdirectorios en Windows y entorno CI equivalente.

### Fase H — Rollout Sentinel/VarSense y CI

- [ ] Completar Ola 1 y dejar cero errores.
- [ ] Integrar Ola 2 antes del runtime 297A-9.
- [ ] Planificar Olas 3/4 antes de sus dominios y activarlas progresivamente.
- [ ] No reinstalar/reiniciar VS Code automáticamente.
- [ ] `task:check` incremental local y full CI comparten core.
- [ ] CI publica reportes al fallar.
- [ ] Pruebas de secretos, timeout, cache y output masivo.
- [ ] Actualizar AGENTS, documentación de herramientas y roadmap.
- [ ] Retirar validación duplicada.

**Gate H:** mismo commit produce decisión equivalente local/CI y las reglas foundation bloquean regresiones reales.

### Fase I — Prueba de uso con agente de bajo contexto

- [ ] Dar únicamente Task ID, comando y stdout a un agente que no conozca el proyecto.
- [ ] Confirmar que identifica estado, máximo tres problemas, archivo/regla y próximo comando sin abrir manuales.
- [ ] Confirmar que un PASS le recuerda cierre documental/Git sin mostrar el protocolo completo.
- [ ] Corregir cualquier ambigüedad de lenguaje o acción antes de declarar estable el gate.

**Gate I:** la calidad se puede cerrar siguiendo la salida mecánicamente, sin memoria implícita del proyecto.

## 17. Matriz de pruebas del runner

### Argumentos/preflight

- [ ] Task ID válido, inválido y ausente.
- [ ] Ejecutado desde subdirectorio.
- [ ] CLI/config faltante o versión incompatible.
- [ ] `--help`, `--version`, schema JSON y clave de config desconocida.
- [ ] Artefacto compilado obsoleto y distribución limpia sin repos hermanos.
- [ ] Paths con espacios/Unicode/Windows.
- [ ] Worktree sin base o sin cambios.

### Ejecución

- [ ] Proceso exit 0/1/2 y señal.
- [ ] Timeout y cancelación de árbol de procesos.
- [ ] Output vacío, inválido, enorme y stderr-only.
- [ ] Etapa dependiente se omite con razón.
- [ ] Reporte se escribe aunque falle una etapa.

### Scope/cache

- [ ] Staged, unstaged y untracked.
- [ ] Cambio CSS activa frontend+VarSense.
- [ ] Cambio migración/config activa full.
- [ ] PASS cacheado se reutiliza sin input cambiado.
- [ ] Cambio fuente/config/tool invalida cache.
- [ ] FAIL nunca se reutiliza como PASS.

### UX/reminders

- [ ] PASS dentro del límite de líneas.
- [ ] FAIL muestra máximo tres hallazgos y próximo comando.
- [ ] Máximo cuatro recordatorios relevantes.
- [ ] Auth/commerce/UI/mobile/docs reciben recordatorio correcto.
- [ ] No aparece checklist genérico completo.

### Seguridad

- [ ] Secretos simulados redactados en terminal, MD, JSON y logs.
- [ ] No shell interpolation.
- [ ] No env completo.
- [ ] No mutación Git/código/deploy.
- [ ] Perfil PowerShell con efectos secundarios no se ejecuta.
- [ ] Señal o exit code ausente nunca se interpreta como PASS.
- [ ] El runner se analiza a sí mismo y no queda excluido por `scripts/**`.

### Usabilidad con poco contexto

- [ ] Un agente nuevo resuelve un FAIL usando solo stdout y el reporte enlazado.
- [ ] Un agente nuevo completa el cierre de un PASS sin leer AGENTS completo.
- [ ] Ninguna salida normal exige escoger manualmente Sentinel, VarSense, Cargo, frontend o docs.

## 18. Definition of Done 297A-6

- [ ] Un único comando público y documentado.
- [ ] Agente no elige herramientas/checks manualmente.
- [ ] Terminal breve, accionable y sin secretos.
- [ ] Reportes completos MD/JSON fuera del contexto.
- [ ] Máximo tres hallazgos y cuatro recordatorios contextuales; nunca una lista genérica.
- [ ] Reejecución rápida con cache segura.
- [ ] Self-check no duplica validaciones.
- [ ] Local y CI comparten el mismo core.
- [ ] CLI/versiones/schemas/configs/distribución son reproducibles y verificables.
- [ ] Sentinel supervisa el runner y VarSense entiende consumidores Vanilla TS.
- [ ] Ola 1 activa con cero errores.
- [ ] Fixtures prueban runner, Sentinel y VarSense.
- [ ] Prueba con agente de bajo contexto completada.
- [ ] No suppressions amplias ni reglas paralelas.
- [ ] Usuario revisó la experiencia y autorizó continuar con 297A-7.
