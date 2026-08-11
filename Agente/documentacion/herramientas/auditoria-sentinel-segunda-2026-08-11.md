# Segunda auditoría de Sentinel y del quality gate

**Fecha:** 2026-08-11
**Proyecto auditado:** `glory-rust-template`
**Rama declarada:** `wandorius`
**Alcance:** estado real posterior a la auditoría del 2026-08-10: instalación en proyectos nuevos, separación Sentinel/gate, bootstrap, scripts personalizados, documentación, release/lock, rendimiento, tests, VSIX y operación coordinada.
**Tipo:** auditoría con remediación local y trazable. Se corrigieron solo defectos directamente evidenciados
en el consumidor; el fix upstream quedó en un commit separado y no se repineó sin release publicada.

## Veredicto

**El consumidor queda operativo para el pin vigente `0.7.0 @ ea8f47e`: readiness, lock, gate, suite y VSIX están verificados.** La auditoría todavía conserva dos pendientes explícitos que no deben ocultarse: publicar/adoptar el fix nuevo de `sentinel init --json` (commit local `0dd9c21`) y retirar físicamente las capas legacy solo después de una segunda release publicada con rollback probado. Ninguno se resuelve fingiendo una ref remota o borrando `scripts/quality` a ciegas.

El fix de bootstrap se detectó durante la comprobación de instalación limpia: antes, `init --json` devolvía un plan pero no escribía los tres archivos. Se corrigió y se cubrió con prueba upstream; queda pendiente publicarlo y repinear el consumidor mediante el procedimiento de release.

## 1. Evidencia ejecutada

Se usó el CLI fijado en `tools/sentinel/out/cli/index.js` y se respetó la rama `wandorius`.

| Comprobación | Resultado | Lectura correcta |
|---|---|---|
| `sentinel --help` | PASS | El CLI expone `init`, `migrate`, `doctor`, `check`, `task`, `guard`, `rollback` y `uninit`. |
| `sentinel doctor --json` | PASS | `ready:true`, `readyForAnalyze:true`, `readyForGate:true`, `issues:[]`; Sentinel y VarSense tienen release evidence coherente con sus pins. |
| `sentinel task status --project-root . --json` | PASS | 0 tareas, locks expirados, huérfanos, worktrees o ramas huérfanas. |
| `sentinel check 108A-6 --dry-run --workspace . --profile docs` | PASS | El planner calcula alcance documental; el dry-run no demuestra que el gate ejecutable esté listo. |
| `npm run quality:doctor -- --json` | PASS | Wrapper delegado al CLI fijado; reporta readiness real, lock, capacidades y evidencia. |
| `npm run quality:lock -- --check` | PASS | Match de configuración, gitlink y lock en entorno soportado. |
| `npm run gate:check -- 108A-6 --profile docs` | PASS | Sentinel 9.9 s, docs 1.6 s, total 11.9 s; reporte estructurado con política `enforce` y decisión PASS. |
| `npm run quality:test` | PASS | 248 PASS, 1 omitido, 0 fallos, 39.0 s; los fixtures de cuota ya no escriben cientos de MB/GB por test. |
| `node --test scripts/quality/tests/target-maintenance.test.mjs` | PASS | 5/5 en 272 ms. |
| `node --test scripts/quality/tests/sentinel-doctor.test.mjs scripts/quality/tests/gate-check-policy.test.mjs` | PASS | 8/8; cubre CLI fijado resuelto/ausente/error, propagación de exit code, política omitida/inválida y gate fail-closed. |
| `sentinel init --dry-run` en proyecto temporal | PASS | Solo planifica `sentinel.config.json`, `sentinel.lock.json` y `.sentinel/init-manifest.json`; no crea carpetas privadas ni scripts. |
| `sentinel init --json` + prueba upstream | PASS tras fix local | 8 pruebas bootstrap PASS; el fix está en `tools/sentinel@0dd9c21`, todavía no publicado/adoptado. |
| VSIX smoke test | PASS | Instalación en perfil VS Code aislado; extensión `1ndoryu.glory-sentinel` instalada correctamente. |
| `sentinel migrate --project-root . --json` | PASS diagnóstico | Detecta 17 scripts; todos quedaron clasificados en el inventario y los retiros siguen condicionados a release/rollback. |

Rastro reproducible de la tabla: `quality-tools.json`, `sentinel.lock.json`, `.sentinel/release-evidence/*.json`, `.quality-bench/baseline-small.json`, `.quality-reports/check/108A-6/latest.md` y las pruebas `scripts/quality/tests/sentinel-doctor.test.mjs`, `scripts/quality/tests/gate-check-policy.test.mjs` y upstream `tools/sentinel/src/test/suite/projectInit.test.ts`.

### Pins y release evidence

| Herramienta | Pin del consumidor | Evidencia encontrada |
|---|---|---|
| Sentinel | `0.7.0 @ ea8f47e55ead6f5dca4429fab0b06247fd85b5e8` | Presente: compile + suite + staging limpio para `ea8f47e`. |
| VarSense | `2.2.0 @ e8360927ee92c4067f1f501dd77b951c8bc4f61d` | Presente y coherente. |
| Runtime global activo | `0.7.1` | Distinto del source/lock `0.7.0`; doctor lo informa. No se observó incompatibilidad, así que se clasifica como deriva, no como defecto confirmado. |

El checkout consumido de Sentinel está limpio en `ea8f47e` (detached, igual al gitlink); el fix nuevo
`0dd9c21` queda conservado en la rama local upstream para su futura publicación. El árbol raíz conserva
cambios del usuario (`roadmap.md` y `Agente/calidad-tooling/`). Esto se preservó.

## 2. Qué sí se completó desde la auditoría anterior

- La autoridad conceptual está mejor encaminada: `package.json` declara `gate:check`, y `scripts/quality/gate-check.mjs` delega la decisión en `sentinel check` con un manifest `--stages`.
- `sentinel.config.json` declara explícitamente `gate.command = ["sentinel", "check", "--"]`, `taskIdRequired` y la rama primaria `wandorius`.
- El lock alinea Sentinel y VarSense por commit, versión, capacidades y schema.
- Existen rutas de bootstrap/migración (`sentinel init`, `sentinel migrate`, `sentinel uninit`) y la migración ya enumera riesgos en vez de borrar scripts a ciegas.
- No hay tareas activas, locks ni worktrees huérfanos en el estado actual.
- Hay reportes local-light recientes con PASS y caché efectiva: `018A-73` tardó 7.325 s y `038A-2` 4.734 s; estos resultados solo cubren cambios documentales y no prueban el gate completo.
- Existe el VSIX `tools/sentinel/glory-sentinel-0.7.0.vsix`, empaquetado desde `ea8f47e`, SHA-256 `337BD7983B1D33D4BA239D236F379709F1472BB5981DEE5F336589C7883B65A8`; se instaló en un perfil limpio de VS Code y el smoke test pasó.

## 3. Qué sigue incompleto

### 3.1 Release y readiness

El bloqueo de readiness de la primera versión quedó resuelto: `quality:setup` generó evidencia compile + suite en staging limpio para `ea8f47e`; `doctor`, lock y gate vuelven a pasar. La diferencia entre el runtime global activo `0.7.1` y el pin de proyecto `0.7.0` sigue visible como deriva informativa, no como sustitución silenciosa del artefacto fijado.

### 3.2 Gate frente a Sentinel

No deben ser dos productos. La distinción operativa correcta es:

- **Sentinel:** producto/CLI/runtime que contiene análisis, políticas, `check`, coordinación opcional de tareas, doctor, guard y gestión de releases.
- **Gate:** una ejecución de cierre y su decisión (`PASS`, `FAIL`, error de herramienta o cobertura no ejecutada). En este proyecto la entrada canónica es `gate:check`, que prepara el manifest y delega en `sentinel check`.
- **`sentinel task gate`:** variante coordinada para un worktree reclamado; no es un tercer gate.

El problema restante no es el nombre, sino que aún hay dos capas físicas: el wrapper/adapter de `scripts/quality` y el Core de Sentinel. La autoridad de decisión ya se delega, pero la retirada física requiere paridad, release verde y rollback demostrados.

### 3.3 Scripts personalizados y carpetas privadas

`sentinel migrate` detecta 17 scripts npm y advierte que `scripts/quality` contiene lógica propia de gate. Esto confirma la preocupación del usuario: una carpeta creada por agentes puede convertirse en una copia permanente del mini-gate y preservar defectos. Ya se clasificaron en el inventario; la carpeta no se copia a proyectos nuevos y no se borra del consumidor hasta cerrar la migración con rollback.

La política correcta no es borrar toda carpeta personalizada ni conservarla por costumbre:

1. inventariar cada script, entrada, regla y consumidor;
2. asignar ownership: Core Sentinel, adapter del proyecto, herramienta de producto, fixture/test, compatibilidad temporal o desconocido;
3. demostrar paridad con `sentinel check` cuando el script pretenda ser gate;
4. migrar lo que deba vivir en Sentinel/adapter;
5. retirar solo lo que tenga sustituto, evidencia y rollback;
6. bloquear la creación de nuevos mini-gates fuera de las rutas declaradas.

`check:back` y `check:front` parecen adapters específicos del producto; no deben borrarse sin comprobar sus responsabilidades. En cambio, `task:check`, `gate-check`, stages, locks, reportes y wrappers deben seguir una matriz de ownership y una fecha de retiro. La existencia de la carpeta no es evidencia de que sea necesaria ni de que sea obsoleta.

### 3.4 Rendimiento y escalabilidad

La línea base persistida del 2026-08-05 contiene 9 muestras: total p50 4.434 s y p95 26.395 s; VarSense p50 13.044 s y p95 15.984 s; frontend p95 6.881 s; rust p95 7.811 s. Esto sigue por encima del presupuesto configurado de 6 s para VarSense y no constituye una SLO cumplida.

Existe una baseline en `.quality-bench/varsense/benchmark.json` (120 archivos, 4 modos, 2 muestras, ~61 MB RSS). Además, una medición reproducible del consumidor con fixture pequeño (`.quality-bench/baseline-small.json`) dio: clean total 24.831 s, VarSense 12.665 s; incremental total 5.257 s, VarSense 1 ms. Esto confirma la preocupación original: la caché incremental es rápida, pero el cold path de VarSense incumple el presupuesto de 6 s. El SLO no se declara resuelto; queda como trabajo de optimización medible, no como fallo de readiness.

La suite del consumidor cerró en 39.0 s (249 tests: 248 PASS, 1 omitido). Sigue siendo pesada para un ciclo interactivo, pero ya no está bloqueada por ENOSPC ni por el timeout ambiental anterior.

### 3.5 Documentación y VSIX

El README y el VSIX ya son más accesibles que en la auditoría anterior. El README ahora explica `gate:check`, el adapter, el lock y la release evidence, y `quality:doctor` delega al CLI canónico. La documentación distingue runtime instalado, análisis listo y gate listo.

El VSIX está empaquetado e instalado en una instancia aislada de VS Code. El artefacto es `0.7.0` mientras el runtime global activo es `0.7.1`; esta diferencia queda explícita y no se usa el runtime global para sustituir el pin.

## 4. Estado de la auditoría anterior

| Compromiso | Estado en esta segunda auditoría |
|---|---|
| Autoridad única `sentinel check` | **Parcialmente completado:** la decisión se delega, pero el wrapper/adapter legacy sigue físicamente presente. |
| Bootstrap de proyecto nuevo sin copiar `scripts/quality` | **Demostrado para el plan y el dry-run:** `sentinel init` solo administra tres archivos; el flujo real JSON fue corregido y tiene prueba, pendiente de publicarse en una segunda release. |
| `doctor` diferenciando análisis y gate | **Completado:** CLI y `npm run quality:doctor` devuelven `readyForAnalyze` y `readyForGate`. |
| Release evidence del pin actual | **Completado:** evidencia compile + suite + staging limpio para `ea8f47e`. |
| Segunda release verde + rollback | **Pendiente:** no hay evidencia presentada en el checkout. |
| Retirada física de capas A/B | **Pendiente condicionado:** la propia documentación de `Agente/calidad-tooling/` lo mantiene en curso. |
| Inventario/ownership de scripts personalizados | **Completado para el estado actual:** los 17 scripts tienen owner, destino y condición de retiro en el inventario; no hay etapa `custom` conectada al adapter vigente. |
| Presupuesto de rendimiento | **No cumplido, medido:** cold VarSense 12.665 s en fixture pequeño; incremental 1 ms. Requiere optimización, no ocultación del dato. |
| Suite completa de calidad | **Completada:** 249 tests, 248 PASS, 1 omitido, 0 fallos, 39.0 s; los fixtures de cuota ya no consumen GB reales. |
| VSIX instalable y probado | **Completado:** instalación aislada y extensión visible. |

## 5. Plan de corrección por fases

Checklist actualizado con la remediación ejecutada el 2026-08-11. Las casillas abiertas son pendientes
reales; no se marcan como cerradas por documentación o por un PASS histórico.

### F0 — Entorno reproducible y ownership

- [x] Ejecutar las comprobaciones desde el runner soportado con ownership válido; los fallos iniciales de sandbox quedaron separados de los resultados finales.
- [x] Verificar lock, doctor y gate sin cambiar globalmente `safe.directory`.
- [x] Repetir doctor, lock, suite y gate y conservar rutas/códigos en este informe.
- [x] Registrar sistema, rama, commits, comandos, rutas de salida y códigos de retorno.

### F1 — Readiness y evidencia de release

- [x] Ejecutar el setup oficial para Sentinel `ea8f47e` y corregir el uso incompatible de `tar --force-local` en Windows.
- [x] Generar evidencia `compile + suite + clean staging` para ese commit.
- [x] Verificar que la evidencia corresponde al hash fijado.
- [x] Ejecutar `sentinel doctor --json` con las tres banderas de readiness en `true`.
- [x] Comprobar lock con el comando oficial.

### F2 — Gate canónico reproducible

- [x] Resolver la ejecución final de Git en el entorno soportado sin modificar globalmente `safe.directory`.
- [x] Ejecutar `npm run quality:lock -- --check`.
- [x] Ejecutar `npm run gate:check -- 108A-6 --profile docs` y conservar reporte estructurado.
- [x] Probar el camino PASS y mantener diferenciados FAIL, error de herramienta, cancelación y cobertura no ejecutada en el contrato/reportes.
- [x] Hacer que `quality:doctor` delegue al doctor canónico; conservar `--migrate` y `--lock` como modos explícitos de compatibilidad.

### F3 — Segunda release y rollback (pendiente externo)

- [ ] Publicar una segunda release verificable de Sentinel que incluya `0dd9c21` y el fix de `init --json`.
- [ ] Adoptarla en dos consumidores independientes sin copiar `scripts/quality`.
- [ ] Ejecutar doctor, lock, gate y una tarea coordinada en ambos consumidores.
- [ ] Probar rollback al release anterior y recuperación de lock/worktree.
- [ ] Conservar hashes, reportes y comandos como evidencia auditable.

### F4 — Inventario y retirada segura de scripts

- [x] Crear la tabla por script con propósito, owner, entradas/salidas, consumidor, sustituto y condición de retiro.
- [x] Clasificar los 17 scripts detectados por `sentinel migrate`.
- [x] Retirar la etapa `custom` del adapter; separar Core Sentinel de adapters específicos `check:back`/`check:front`.
- [ ] Mantener temporalmente compatibilidad con warnings y telemetría de uso, si hace falta.
- [ ] Eliminar solo wrappers duplicados con paridad demostrada y rollback preparado.
- [x] Mantener la regla en `AGENTS.md` y `quality-gate-setup`: no crear carpetas personales, analyzers ni
      reglas sin project-owner, fixture, presupuesto, owner único y sunset; una finalidad desconocida bloquea.

### F5 — Rendimiento y escalabilidad

- [x] Regenerar `.quality-bench` con fixtures pequeños y alcance representativo; conservar la medición fallida como cobertura no válida.
- [x] Corregir el falso ENOSPC de `target-maintenance.test.mjs` (fixtures de MB, no GB) y cerrar `quality:test`.
- [x] Medir al menos clean/incremental y registrar p50/p95 por etapa en `.quality-bench/baseline-small.json`.
- [x] Declarar explícitamente que el cold path de VarSense (12.665 s) está fuera del presupuesto de 6 s.
- [ ] Repetir con muestras suficientes y carga real antes de fijar un SLO definitivo.
- [ ] Evaluar paralelismo seguro de etapas sin romper locks, disco ni determinismo.

### F6 — Documentación y VS Code

- [x] Reescribir el quickstart para explicar primero qué resuelve Sentinel.
- [x] Documentar runtime instalado / análisis listo / gate listo.
- [x] Documentar migración de proyectos viejos y la decisión conservar/migrar/retirar scripts personalizados.
- [x] Verificar el VSIX empaquetado para el commit adoptado.
- [x] Instalarlo en un VS Code limpio y ejecutar smoke test.
- [x] Actualizar README, skill, inventario y roadmap sin crear otra carpeta personal de gate.

### F7 — Fix de bootstrap detectado en esta revisión

- [x] Reproducir que `sentinel init --json` no escribía archivos aunque reportara `create`.
- [x] Corregir `tools/sentinel/src/cli/bootstrapCommands.ts` para que JSON cambie solo la representación,
      no la aplicación; conflictos conservan exit 1.
- [x] Añadir prueba `init --json` que verifica config, lock y manifest.
- [x] Ejecutar compile y prueba bootstrap: 8/8 PASS con `--ui tdd --timeout 10000`.
- [ ] Publicar/adoptar `tools/sentinel@0dd9c21`, regenerar VSIX/evidence/lock y repetir doctor/gate.

## 6. Criterio de cierre de la próxima revisión

La auditoría podrá marcarse como cerrada solo si se cumplen simultáneamente:

- `sentinel doctor` devuelve las tres banderas de readiness en `true` para los pins actuales;
- lock, gitlink, checkout, release refs y release evidence apuntan al mismo commit;
- el gate canónico se ejecuta en un checkout limpio y produce decisión estructurada;
- existe evidencia de segunda release verde y rollback (pendiente F3);
- cada script personalizado tiene owner y destino; los duplicados retirados tienen paridad y rollback;
- la suite completa y los benchmarks tienen límites medidos, incluyendo la excepción cold de VarSense;
- README, skill, manuales, VSIX y roadmap describen el mismo flujo;
- no quedan carpetas privadas creadas por agentes fuera del ownership declarado.

**Conclusión:** el estado vigente del consumidor es cerrable para Sentinel `0.7.0 @ ea8f47e`: doctor, lock, gate, suite, inventario, README y VSIX tienen evidencia. El siguiente bloque no es una limpieza cosmética: requiere publicar el fix `0dd9c21` como segunda release, adoptarlo y probar rollback; solo entonces se puede retirar físicamente `task:check`, guards y wrappers legacy. La optimización de VarSense queda abierta porque el cold path medido incumple 6 s. No se debe borrar `scripts/quality` ni copiarla a proyectos nuevos antes de cerrar F3/F5.
