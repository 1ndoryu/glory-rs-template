# Segunda auditoría de Sentinel y del quality gate

**Fecha:** 2026-08-11
**Proyecto auditado:** `glory-rust-template`
**Rama declarada:** `wandorius`
**Alcance:** estado real posterior a la auditoría del 2026-08-10: instalación en proyectos nuevos, separación Sentinel/gate, bootstrap, scripts personalizados, documentación, release/lock, rendimiento, tests, VSIX y operación coordinada.
**Tipo:** auditoría con remediación local y trazable. Se corrigieron los defectos evidenciados, se publicaron
las releases correctivas y se repinearon ambos consumidores sin copiar `scripts/quality`.

## Veredicto

**La instalación y operación quedan corregidas para Sentinel `0.7.1` y VarSense `2.2.1` en los dos consumidores auditados.** Doctor, lock, release evidence, setup y suites pasan en ambos; wandorius tiene gate docs/frontend PASS y glory-rs-rest ejecuta el gate canónico pero conserva cinco findings de producto `broadcast-mutex-riesgo-rs`, ya documentados como baseline ajeno a la instalación. El rollback real `0.7.1 → 0.7.0 → 0.7.1` pasó y el runtime quedó restaurado. La retirada física de wrappers permanece condicionada por el runbook: faltan dos CI consecutivos verdes con matriz multi-shell y un gate plenamente verde en glory-rs-rest.

El fix de bootstrap se detectó durante la comprobación de instalación limpia: antes, `init --json` devolvía un plan pero no escribía los tres archivos. Se corrigió, se cubrió con prueba upstream y se publicó en `v0.7.1`; ambos consumidores ya apuntan a ese release.

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
| `sentinel init --json` + prueba upstream | PASS en release preparada | 8 pruebas bootstrap PASS; corregido en `tools/sentinel@b22c848` (release preparada `0.7.1`), pendiente de publicación/adopción. |
| VarSense cold instrumentado sobre workspace real | PASS de rendimiento | Causa aislada: recorridos repetidos de workspace en `classIndex`; el fix `tools/varsense@88f281f` baja cold a ~3.3 s y warm a ~2.8 s, por debajo del presupuesto de 6 s; pendiente de publicación/adopción. |
| VSIX smoke test | PASS | Instalación en perfil VS Code aislado; extensión `1ndoryu.glory-sentinel` instalada correctamente. |
| `sentinel migrate --project-root . --json` | PASS diagnóstico | Detecta 17 scripts; todos quedaron clasificados en el inventario y los retiros siguen condicionados a release/rollback. |

Rastro reproducible de la tabla: `quality-tools.json`, `sentinel.lock.json`, `.sentinel/release-evidence/*.json`, `.quality-bench/baseline-small.json`, `.quality-reports/check/108A-6/latest.md` y las pruebas `scripts/quality/tests/sentinel-doctor.test.mjs`, `scripts/quality/tests/gate-check-policy.test.mjs` y upstream `tools/sentinel/src/test/suite/projectInit.test.ts`.

### Pins y release evidence

| Herramienta | Pin del consumidor | Evidencia encontrada |
|---|---|---|
| Sentinel | `0.7.1 @ b22c8484fd2334f19a88f930c494091d02942e39` | Publicado en `refs/heads/release/0.7.1` y `refs/tags/v0.7.1`; compile + 558 pruebas + staging limpio. |
| VarSense | `2.2.1 @ 88f281f94e6febd02a386b7ed03d30d285eb82e1` | Publicado en `refs/heads/release/2.2.1` y `refs/tags/v2.2.1`; compile + 61 pruebas + staging limpio. |
| Runtime global activo | `0.7.1` | Coincide con source/lock después del rollback de ida y vuelta; `activeVerified:true`. |

Los checkouts consumidos de Sentinel y VarSense están limpios y detached en los commits publicados; los
gitlinks, `quality-tools.json`, `sentinel.lock.json`, release refs y release evidence coinciden. El
worktree real de glory-rs-rest conserva cambios previos del usuario y no se modificó; la adopción se
verificó en un worktree limpio y se publicó en `origin/glory-rs-rest`.

## 2. Qué sí se completó desde la auditoría anterior

- La autoridad conceptual está mejor encaminada: `package.json` declara `gate:check`, y `scripts/quality/gate-check.mjs` delega la decisión en `sentinel check` con un manifest `--stages`.
- `sentinel.config.json` declara explícitamente `gate.command = ["sentinel", "check", "--"]`, `taskIdRequired` y la rama primaria `wandorius`.
- El lock alinea Sentinel y VarSense por commit, versión, capacidades y schema.
- Existen rutas de bootstrap/migración (`sentinel init`, `sentinel migrate`, `sentinel uninit`) y la migración ya enumera riesgos en vez de borrar scripts a ciegas.
- No hay tareas activas, locks ni worktrees huérfanos en el estado actual.
- Hay reportes local-light recientes con PASS y caché efectiva: `018A-73` tardó 7.325 s y `038A-2` 4.734 s; estos resultados solo cubren cambios documentales y no prueban el gate completo.
- Existe el VSIX histórico `tools/sentinel/glory-sentinel-0.7.0.vsix`, empaquetado desde `ea8f47e`, SHA-256 `337BD7983B1D33D4BA239D236F379709F1472BB5981DEE5F336589C7883B65A8`; se instaló en un perfil limpio de VS Code y el smoke test pasó. El VSIX 0.7.1 también fue generado y su hash queda en la evidencia de release; instalarlo es una distribución del editor separada del gate del consumidor.

## 3. Qué sigue incompleto

### 3.1 Release y readiness

El bloqueo de readiness quedó resuelto para los dos releases publicados: `quality:setup` generó evidencia
compile + suite en staging limpio, `quality:lock --check` pasó y `doctor` devolvió `ready:true`,
`readyForAnalyze:true`, `readyForGate:true` e `issues:[]` en ambos consumidores.

La evidencia local no equivale a CI verde. La consulta de Actions del upstream muestra `main` en fallo en
los tres runs consecutivos más recientes: [#36](https://github.com/1ndoryu/glory-sentinel/actions/runs/31372934670),
[#37](https://github.com/1ndoryu/glory-sentinel/actions/runs/31379295222) y
[#38](https://github.com/1ndoryu/glory-sentinel/actions/runs/31380898957). Por eso el criterio de retirada
no se marca como cumplido, aunque los releases publicados pasen compile/suite en staging local.

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

La línea base persistida del 2026-08-05 contiene 9 muestras: total p50 4.434 s y p95 26.395 s; VarSense p50 13.044 s y p95 15.984 s; frontend p95 6.881 s; rust p95 7.811 s. Esa medición motivó el fix upstream. La instrumentación sobre el workspace real aisló `classIndexMs` como cuello: el provider recorría el árbol repetidamente para cada patrón. Tras consolidar los patrones en un recorrido y cachear el snapshot por exclusiones, el release publicado `2.2.1 @ 88f281f` midió cold ~3.3 s y warm ~2.8 s (tres ejecuciones, sin cambio de hallazgos), por debajo del presupuesto de 6 s.

Existe una baseline en `.quality-bench/varsense/benchmark.json` (120 archivos, 4 modos, 2 muestras, ~61 MB RSS). Con el pin publicado, el gate frontend de wandorius midió VarSense en 5.913 s, bajo el presupuesto configurado de 6 s; la optimización upstream también midió cold ~3.3 s y warm ~2.8 s. La mejora conserva la cobertura y elimina los recorridos repetidos del workspace.

La suite del consumidor cerró en 39.0 s (249 tests: 248 PASS, 1 omitido). Sigue siendo pesada para un ciclo interactivo, pero ya no está bloqueada por ENOSPC ni por el timeout ambiental anterior. La suite upstream de VarSense pasó 61 pruebas tras la optimización; Sentinel pasó 8 pruebas focalizadas de bootstrap más compile/check-core/smoke.

### 3.5 Documentación y VSIX

El README y el VSIX ya son más accesibles que en la auditoría anterior. El README ahora explica `gate:check`, el adapter, el lock y la release evidence, y `quality:doctor` delega al CLI canónico. La documentación distingue runtime instalado, análisis listo y gate listo.

El VSIX 0.7.0 está instalado en una instancia aislada y el 0.7.1 fue generado desde el commit publicado. La extensión de VS Code y el runtime del gate son artefactos distintos; ninguno sustituye silenciosamente el pin del otro.

## 4. Estado de la auditoría anterior

| Compromiso | Estado en esta segunda auditoría |
|---|---|
| Autoridad única `sentinel check` | **Parcialmente completado:** la decisión se delega, pero el wrapper/adapter legacy sigue físicamente presente. |
| Bootstrap de proyecto nuevo sin copiar `scripts/quality` | **Completado:** `sentinel init` solo administra tres archivos; `init --json` escribe correctamente y tiene prueba en 0.7.1. |
| `doctor` diferenciando análisis y gate | **Completado:** CLI y `npm run quality:doctor` devuelven `readyForAnalyze` y `readyForGate`. |
| Release evidence del pin actual | **Completado:** evidencia compile + suite + staging limpio para Sentinel 0.7.1 y VarSense 2.2.1. |
| Segunda release verde + rollback | **Completado con límite:** ambos releases están publicados/adoptados, doctor/lock/suites pasan y rollback real fue probado; faltan dos CI consecutivos y el gate sin baseline rojo en glory-rs-rest. |
| Retirada física de capas A/B | **Pendiente condicionado:** la propia documentación de `Agente/calidad-tooling/` lo mantiene en curso. |
| Inventario/ownership de scripts personalizados | **Completado para el estado actual:** los 17 scripts tienen owner, destino y condición de retiro en el inventario; no hay etapa `custom` conectada al adapter vigente. |
| Presupuesto de rendimiento | **Completado para el pin publicado en wandorius:** VarSense 5.913 s en gate frontend y cold/warm instrumentado bajo 6 s; se conserva el histórico fuera de presupuesto como comparación. |
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

- [x] Ejecutar el setup oficial para Sentinel `b22c848` y corregir el uso incompatible de `tar --force-local` en Windows.
- [x] Generar evidencia `compile + suite + clean staging` para los commits publicados.
- [x] Verificar que la evidencia corresponde al hash fijado.
- [x] Ejecutar `sentinel doctor --json` con las tres banderas de readiness en `true`.
- [x] Comprobar lock con el comando oficial.

### F2 — Gate canónico reproducible

- [x] Resolver la ejecución final de Git en el entorno soportado sin modificar globalmente `safe.directory`.
- [x] Ejecutar `npm run quality:lock -- --check`.
- [x] Ejecutar `npm run gate:check -- 108A-6 --profile docs` y conservar reporte estructurado.
- [x] Probar el camino PASS y mantener diferenciados FAIL, error de herramienta, cancelación y cobertura no ejecutada en el contrato/reportes.
- [x] Hacer que `quality:doctor` delegue al doctor canónico; conservar `--migrate` y `--lock` como modos explícitos de compatibilidad.

### F3 — Segunda release y rollback

- [x] Publicar Sentinel `v0.7.1` (`b22c848`) y VarSense `v2.2.1` (`88f281f`) con refs verificables.
- [x] Adoptar ambos releases en wandorius y glory-rs-rest sin copiar `scripts/quality`.
- [x] Ejecutar setup, doctor, lock y gate canónico en ambos; el gate de glory-rs-rest conserva cinco findings de producto `broadcast-mutex-riesgo-rs`.
- [x] Probar rollback real del runtime `0.7.1 → 0.7.0 → 0.7.1`; `activeVerified:true` y doctor listo al restaurar.
- [x] Conservar hashes, reportes, commits y comandos en este informe y en `evidencia-release-sentinel-071-varsense-221-2026-08-11.md`.
- [ ] Completar dos ejecuciones CI consecutivas verdes y la matriz multi-shell exigidas por el runbook antes de retirar la capa A.

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
- [x] Declarar explícitamente que el cold path histórico de VarSense (12.665 s) estaba fuera del presupuesto de 6 s.
- [x] Repetir con muestras suficientes y carga real para validar la corrección: 3 ejecuciones instrumentadas, cold ~3.3 s y warm ~2.8 s.
- [x] Publicar/adoptar `88f281f`, repetir gate y conservar el presupuesto del pin consumido.
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
- [x] Publicar/adoptar `tools/sentinel@b22c848` (`0.7.1`), regenerar evidence/lock y repetir doctor/gate. El VSIX histórico 0.7.0 sigue siendo válido como artefacto anterior; su regeneración queda separada de la instalación del gate.

## 6. Criterio de retirada pendiente / próxima revisión

La adopción y la corrección de instalación ya están cerradas. La retirada física de capas A/B solo podrá
marcarse como cerrada cuando se cumpla el criterio único del runbook §3:

- `sentinel doctor` devuelve las tres banderas de readiness en `true` para los pins actuales;
- lock, gitlink, checkout, release refs y release evidence apuntan al mismo commit;
- el gate canónico se ejecuta en un checkout limpio y produce decisión estructurada;
- existe evidencia de segunda release y rollback (F3 completada); queda pendiente la CI/matriz/gates verdes
  adicionales exigidos por el runbook;
- cada script personalizado tiene owner y destino; los duplicados retirados tienen paridad y rollback;
- la suite completa y los benchmarks tienen límites medidos, incluyendo la excepción cold de VarSense;
- README, skill, manuales, VSIX y roadmap describen el mismo flujo;
- no quedan carpetas privadas creadas por agentes fuera del ownership declarado.

**Conclusión:** la instalación, bootstrap, readiness, lock, rendimiento y adopción multi-consumidor quedan corregidos en Sentinel `0.7.1`/VarSense `2.2.1`; el rollback real también está verificado. Sentinel y gate son una sola autoridad de decisión (`gate:check` prepara; `sentinel check` decide), mientras `task:check` queda como alias de compatibilidad. No se deben borrar ni copiar carpetas personalizadas: la capa A/B solo se retira después de dos CI consecutivos, matriz multi-shell y gates verdes. El baseline `broadcast-mutex-riesgo-rs` de glory-rs-rest queda como deuda de producto separada y visible, no como falso PASS.
