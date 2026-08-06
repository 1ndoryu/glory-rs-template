# Plan — Migración de scripts a Sentinel Core y adapters por proyecto

> **Fecha:** 2026-08-06
> **Estado:** Fase 1 local de transición implementada; schema upstream, fixtures multi-proyecto y retirada física siguen pendientes
> **Ámbito:** calidad, coordinación de tareas y wrappers de desarrollo; no modifica todavía la skill global ni elimina scripts
> **Relación:** complementa `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md` y `Agente/planes/plan-sentinel-orquestacion-tareas-worktrees-2026-08-06.md`
> **Fuente canónica de esta iniciativa:** este documento

## 1. Decisión ejecutiva

El problema no es simplemente que haya muchos archivos. El problema es que el checkout mezcla cuatro
responsabilidades con vidas útiles distintas:

1. **Plano universal:** coordinación de tareas, locks/claims, worktrees, alcance, caché, cooldown,
   ejecución de etapas, contratos de proceso, redacción, reportería, instalación y shims.
2. **Adaptación por proyecto:** cómo se descubre el stack, qué perfiles existen, qué comandos de build/test
   se ejecutan, cómo se invoca PostgreSQL/Cargo, qué analizadores están habilitados y qué documentos son
   canónicos.
3. **Compatibilidad temporal:** aliases como `npm run task:check`, `self-check.ps1`, wrappers legacy y
   doble vía `observe` mientras Sentinel alcanza paridad demostrada.
4. **Operación específica o histórica:** rescates, Coolify/WordPress, migraciones manuales, diagnósticos de
   un sitio, scripts temporales y herramientas de desarrollo que no deben entrar en un runtime universal.

La decisión propuesta es:

> **Sentinel Core/CLI es el único dueño del plano universal. Cada proyecto aporta un manifiesto declarativo
> y un adapter pequeño para su stack. Los scripts específicos permanecen únicamente cuando encapsulan una
> operación de dominio real que no puede generalizarse sin inventar una abstracción. Los aliases se mantienen
> durante una ventana de migración medible y luego se retiran por evidencia, no por limpieza estética.**

No se propone copiar `scripts/quality` a otro repositorio, renombrarlo o crear un tercer producto. Tampoco
se propone que todos los proyectos tengan scripts personalizados extensos: el objetivo es que la
personalización se reduzca a configuración declarativa + pocos adapters versionados, con una experiencia
pública de uno o dos comandos.

## 2. Evidencia de la situación actual

### 2.1 Capacidades ya disponibles en Sentinel

La instalación y el checkout actuales muestran:

- Sentinel `0.5.0`, commit `20c13a216e879303fcf5be7469a2821391b2ec0d`, con `task claim/start/heartbeat/status/gate/integrate/cleanup/release`.
- Runtime global activo `0.5.0`, hash verificado y `sentinel doctor --json` operativo.
- Política del proyecto en `sentinel.config.json` v2, modo `enforce`, rama primaria `wandorius`.
- Lock reproducible de Sentinel y VarSense; los submódulos fijan los commits esperados.
- `sentinel check <task-id> --stages` y contratos de stages, pero la integración del gate local aún depende
  del orquestador de `scripts/quality` para crear la descripción de etapas y los adapters.
- El plan de orquestación confirma que GC, runbook multi-OS y parte de la adopción siguen pendientes.

Esto significa que la migración no parte de cero, pero tampoco autoriza a borrar `scripts/quality`: aún es
la implementación comprobada del gate de este consumidor.

### 2.2 Tamaño y acoplamiento observados

`scripts/quality` contiene aproximadamente 45 módulos ejecutables, varios de 200–430 líneas, más una suite
amplia de tests. Los mayores focos son:

- `task-check.mjs`: orquesta preflight, scope, caché, locks, stages, takeover, mantenimiento y reporte.
- `policy.mjs`, `scope.mjs`, `lockfile.mjs`, `task-takeover.mjs`, `heavy-run-guard.mjs`, `report-retention.mjs`
  y `target-maintenance.mjs`: contienen decisiones que son candidatas claras a Sentinel Core, aunque algunas
  incluyen defaults/rutas del proyecto que deben extraerse.
- `adapters/*.mjs`: son la frontera correcta para el stack; no deben convertirse en core.
- `run-with-db.mjs` y `branch-db.mjs`: mezclan un helper de Rust/PostgreSQL de este consumidor con el guard
  universal de comandos pesados y banners de coordinación.
- `docs.mjs` y `custom-rules.mjs`: contienen reglas y convenciones de wandori.us; no son reglas universales.
- `bench-baseline.mjs`, `quality-profile.mjs`, `observe-compare.mjs`, `varsense-parity.mjs` y mantenimiento
  son herramientas de diagnóstico/transición, no el camino mínimo que debe aprender un agente nuevo.

El inventario completo y la clasificación se ejecutarán como primera fase; este plan no presume que un
nombre de archivo implique automáticamente que deba migrarse.

## 3. Modelo objetivo

### 3.1 Tres capas, más un archivo histórico separado

| Capa                         | Vive en                                    | Dueño                | Contiene                                                                                                          | No contiene                                                                                             |
| ---------------------------- | ------------------------------------------ | -------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **Sentinel Core**            | upstream `glory-sentinel`                  | equipo de tooling    | task lifecycle, scheduler, scope contract, stage runner, cache, lock, report schema, redaction, runtime/shims, GC | rutas `frontend/src`, PostgreSQL, nombres de tareas del producto, reglas visuales, español del proyecto |
| **Sentinel project adapter** | raíz del proyecto + pequeño paquete/config | consumidor           | manifest de comandos, profiles, stack checks, DB/Cargo adapter, docs adapter, custom checks, VarSense config      | segunda política de cooldown, segundo reporte, reglas copiada de Sentinel, shell global propia          |
| **Scripts específicos**      | `scripts/` o carpeta operativa             | producto/operaciones | deploy autorizado, migración particular, rescue, codegen, fixtures, tareas de dominio                             | coordinación universal, secretos embebidos, gate paralelo, wrappers duplicados                          |
| **Legacy/experimental**      | `scripts/legacy/` o historial              | mantenimiento        | compatibilidad durante la ventana definida                                                                        | nuevas dependencias, uso recomendado por agentes                                                        |

La palabra “adapter” no debe convertirse en una excusa para conservar un segundo orquestador. Un adapter
puede seleccionar comandos y transformar contratos; no decide si un finding bloquea, no implementa otro
cooldown, no escribe otro reporte final y no crea otro sistema de claims/worktrees.

### 3.2 Contrato de proyecto propuesto

La forma preferida es declarativa:

```json
{
    "schemaVersion": 1,
    "project": {
        "primaryBranch": "wandorius",
        "stack": ["rust", "node", "postgres"]
    },
    "gate": {
        "profiles": {
            "rust": {"adapter": "rust-postgres", "checks": ["fmt", "check", "clippy", "test"]},
            "frontend": {"adapter": "vite", "checks": ["type-check", "test:changed", "build"]},
            "docs": {"adapter": "wandorius-docs"}
        },
        "fullChecks": {"ci": ["rust.clippy", "rust.test", "frontend.test:full", "frontend.build"]}
    },
    "analyzers": {
        "sentinel": {"enabled": true},
        "varsense": {"enabled": true, "config": "varsense.config.json"}
    }
}
```

El formato exacto debe ser definido por Sentinel Core, no inventado en este plan. Mientras ese contrato no
exista, `sentinel.config.json` v2 y el adapter local continúan siendo la fuente real. La configuración debe
permitir:

- comandos como arrays de argumentos, nunca shell concatenado;
- timeout, perfil y condición (`local-light`, `full`, `ci`) declarativos;
- variables de entorno allowlisted y nombres de archivos de reportes;
- estados `pass`, `findings`, `tool-error`, `timeout`, `cancelled`, `invalid-output`;
- límites de salida, memoria y concurrencia;
- una versión/protocolo de adapter y hash del manifest;
- un modo de compatibilidad explícito, nunca autodetectado silenciosamente.

### 3.3 Experiencia simplificada para agentes

El agente nuevo no debería aprender 25 comandos de mantenimiento. En este consumidor el camino canónico
incluye primero el lock local y después el claim de Sentinel; no se deben omitir capas ni inventar una
secuencia abreviada:

```text
1. npm run task:take -- --task <ID> --by <agente>
2. sentinel task claim <ID> --project-root . --agent <agente>
3. sentinel task start <ID> --project-root . --agent <agente> --primary-branch wandorius
4. editar únicamente en el worktree registrado
5. sentinel task gate <ID> --project-root <worktree> --agent <agente>
6. commit explícito en el worktree
7. sentinel task integrate <ID> --project-root . --agent <agente> --target wandorius
8. sentinel task cleanup <ID> --project-root . --agent <agente>
9. sentinel task release <ID> --project-root . --agent <agente>
10. npm run task:release -- --task <ID>
```

Si el proyecto ofrece un wrapper futuro, debe ejecutar o validar esta secuencia completa; no debe ocultar
el ownership ni crear un tercer coordinador. Para una corrección pequeña sin paralelismo, el gate de cierre
sigue siendo `npm run task:check -- <ID>` con `GLORY_AGENT_ID` igual al agente que tomó la tarea.

**Runbook corto de agente:**

- **Inicio normal:** seguir los diez pasos anteriores; nunca trabajar en la rama primaria si `start` creó
  un worktree.
- **Fallo de gate:** leer `.quality-reports/` y corregir la causa exacta; no repetir comandos pesados a
  ciegas ni usar `--allow-heavy` sin autorización explícita y motivo auditable.
- **Tarea tomada por otro:** detenerse; ejecutar `sentinel task status --project-root . --json` y coordinar,
  no usar `--force` salvo takeover expirado con evidencia.
- **Fin normal:** verificar que el worktree, la rama, el claim Sentinel y el lock local fueron limpiados.

Los comandos de diagnóstico/benchmark/cleanup quedan para maintainers y CI, no para la guía principal del
agente.

En consumidores que aún requieren compatibilidad:

```text
npm run task:check -- <ID>
```

debe delegar al mismo core y mostrar una recomendación breve si se ejecuta desde la raíz incorrecta,
si falta la toma o si el stack adapter no está instalado. Los comandos de diagnóstico/benchmark/cleanup
quedan para maintainers y CI, no para la guía principal del agente.

No se deben añadir aliases por cada combinación (`quality:rust`, `quality:frontend`, etc.) salvo que haya
un uso humano repetido y documentado. La complejidad debe estar detrás del gate.

## 4. Clasificación inicial de los scripts

La clasificación es provisional hasta la auditoría de dependencias y pruebas de fase 1.

### 4.1 Candidatos a migrar al Core de Sentinel

Migrar la **capacidad**, no necesariamente el archivo ni el nombre:

- `scope.mjs`: descubrimiento de alcance, manifest, hashes y expansión de dependencias.
- `cache.mjs`: caché de stages por identidad de proyecto/rama/política/runtime.
- `lock.mjs`: locks de gate; combinar con el scheduler universal.
- `heavy-run-guard.mjs`: cooldown/lease/override auditado, sin defaults `C:\\tmp` del proyecto.
- `runner.mjs`, `stage-runner.mjs`, `structured-tool.mjs`: ejecución segura, timeout, cancelación y contrato JSON.
- `reporter.mjs`, `redaction.mjs`: schema y salida compartida; los templates de producto deben ser plugins.
- `branch-identity.mjs`, `report-retention.mjs`: namespace, retención y poda segura.
- partes universales de `preflight.mjs`, `lockfile.mjs`, `policy-decision.mjs` y `quality-command-guard.mjs`.
- coordinación actualmente en `task-takeover.mjs`, que debe converger con el coordinador de Sentinel; no deben
  quedar dos claims obligatorios con semántica distinta.
- capacidad universal de `target-maintenance.mjs` solo si se generaliza el concepto de artefacto pesado y sus
  providers; la ruta, cuota y detección WMI no deben quedar codificadas como regla universal.

**Criterio de migración:** solo mover una capacidad cuando exista un API agnóstico, fixture positiva/negativa,
contrato JSON, rollback y evidencia de que dos proyectos distintos la pueden usar sin conocer wandori.us.

### 4.2 Deben quedarse como adapters del proyecto

- `adapters/sentinel.mjs`: invocación de Sentinel con el `sentinel.config.json` del consumidor.
- `adapters/varsense.mjs` y `varsense-contract.mjs`: VarSense como plugin; nunca gate independiente.
- `adapters/rust.mjs`: `cargo fmt/check/clippy/test` y su política local de BD.
- `adapters/frontend.mjs`: TypeScript/Vitest/Vite y presupuestos de este frontend.
- `adapters/docs.mjs`: roadmap, planes, manuales y español de este repositorio.
- `adapters/custom.mjs` y `custom-rules.mjs`: reglas de arquitectura del producto hasta que una regla sea
  demostrablemente agnóstica y se publique en Sentinel upstream.
- `run-frontend-tests.mjs`/`frontend-test-selection.mjs`: selección Vitest específica de estructura; puede
  reducirse a un adapter declarativo después de comprobar que el contrato incremental cubre sus casos.
- `branch-db.mjs` y el núcleo de `run-with-db.mjs`: helper específico Rust/PostgreSQL, derivación segura de
  DB/target y lifecycle del proceso cargo.
- `codegen-local.mjs`: operación de OpenAPI/Orval del stack.
- `check-roadmap.mjs`: integración con editor y problem matcher de este proyecto.

### 4.3 Deben quedarse, pero fuera del camino normal de calidad

- `bench-baseline.mjs`, `bench-fixtures.mjs`, `quality-profile.mjs`, `export-ci-metrics.mjs` y
  `observe-compare.mjs`: diagnóstico/CI/transición; no hacerlos obligatorios para agentes.
- `index-maintenance.mjs`, `target-maintenance.mjs`, `report-cleanup.mjs`: mantenimiento explícito o etapa
  best-effort del core, no comandos cotidianos.
- `setup.mjs`, `lock-generator.mjs`, `sentinel-doctor.mjs`, `install-global-runtime.mjs`: bootstrap y
  operaciones de maintainer; preferir el CLI global de Sentinel una vez que la integración sea completa.
- `self-check.ps1`: alias temporal y luego retirada; no duplicar validaciones.

### 4.4 Deben retirarse o archivarse tras verificación

- `global-cargo-guard.ps1`, `global-quality-guard.sh`, `install-global-guard.ps1`, `cargo.cmd`, `node.cmd`,
  `npm.cmd`, `npx.cmd`, `quality-command-guard.mjs`: wrappers duplicados cuando el runtime global y los shims
  publicados cubran las mismas decisiones en dos releases consecutivos.
- scripts temporales (`tmp-*`) y diagnósticos de un incidente ya cerrado: no migrarlos; archivarlos fuera del
  checkout o eliminarlos con revisión explícita de referencias.
- scripts de operaciones de otros productos/sitios (`fix-guillermo-*`, rescates de Kamples, etc.): no deben
  ser parte del template universal; mover a un repositorio/runbook operativo específico, con secretos
  revocados y sin endpoints hardcodeados.
- `add-smtp-envs.ps1`, `push-prod-envs*.ps1`, `redeploy-sites.ps1`, `diagnose-coolify.sh` y equivalentes:
  no migrarlos a Sentinel; producción debe usar Coolify Manager y estas copias deben auditarse/retirarse.

**Importante:** no borrar ni mover automáticamente estos archivos en esta fase. Primero comprobar referencias,
secreto, historial, uso reciente y propiedad. Un script que contenga un token o endpoint sensible debe
tratarse como incidente de seguridad y salir del flujo normal de migración; no copiar su contenido a un
nuevo adapter.

## 5. Fases ejecutables

### Fase 0 — Baseline, ownership y seguridad (ID propuesto: `SNT-12`)

**Dependencia:** terminar la integración operativa de `028A-18` y no tocar `tools/sentinel` en paralelo.
Este plan no cambia la prioridad del bloque habilitado hasta que el consumidor autorice formalmente SNT-12.
**Evidencia:** `Agente/documentacion/herramientas/inventario-scripts-adapters-sentinel-2026-08-06.md`.
**ID de gate del consumidor:** usar un ID con formato de tarea del proyecto (por ejemplo `028A-20`) cuando
se ejecute `npm run task:check`; `SNT-12` es el alias de planificación y coordinación, no un ID válido del
parser legacy del gate.

**Objetivo:** congelar evidencia antes de tocar la arquitectura y bloquear la propagación de secretos.

- [x] Registrar `git status`, rama, versión/commit/hash de Sentinel y estado de ambos submódulos.
- [x] Inventariar todos los scripts del root y `scripts/quality` con owner, referencias, frecuencia, secretos,
      dependencias, capacidades y destino propuesto.
- [x] Graficar referencias desde `package.json`, CI, README, AGENTS, planes y scripts.
- [x] Crear una tabla versionada con **cada archivo**: ruta, categoría, dueño, consumidores, último uso
      conocido, destino (`core`, `adapter`, `específico`, `legacy`, `retirar`), riesgo, criterio de salida,
      release objetivo y rollback. La clasificación narrativa de este plan no sustituye esa matriz.
- [x] Separar cambios preexistentes de esta iniciativa; no stagear ni editar WIP ajeno.
- [x] Ejecutar baseline no destructivo: `sentinel doctor --json`, `sentinel task status --json`,
      `npm run quality:test` y el gate mínimo solo cuando el ownership de la tarea esté tomado.
- [x] Auditar inmediatamente scripts con tokens, cookies, IPs, endpoints autenticados o credenciales.
      La exposición de un secreto es un bloqueo de seguridad: no se copia a la matriz ni a un adapter,
      se revoca por el procedimiento autorizado y se retira el archivo del template en una tarea separada.

**Salida:** inventario versionado, baseline reproducible, lista de archivos con propiedad clara y cero
secretos nuevos propagados. Hasta esta salida no se mueve ni elimina ningún script.

**Resultado 2026-08-06:** baseline y matriz creados; `npm run quality:test` pasó 211/211 en la rama base;
`sentinel doctor --json` reportó runtime 0.5.0 verificado; no se encontraron secretos literales en la
búsqueda focalizada; `emit-openapi.ps1` quedó señalado porque referencia `clean-cargo-target.ps1`, ausente
en el checkout actual. La ausencia de owners reales y de frecuencia histórica requiere revisión del equipo;
no se inventan esos datos.

### Fase 1 — Contrato mínimo de adapter (ID: `SNT-13`)

**Objetivo:** hacer que un proyecto nuevo necesite configuración, no una copia de `scripts/quality`.

**Resultado local 2026-08-06:** `quality-adapter.json` y `adapter-manifest.mjs` implementan el contrato de transición; `stages.mjs`/`stage-process.mjs` usan argv estructurado, validan paths/task IDs y generan salida versionada. El runner aplica allowlist mínima no sensible más variables declaradas por el manifest; `observe-compare` usa `--run-id`, conserva reportes canónicos y exige metadata fresca. Evidencia: suite completa del adapter **217/217 PASS**, incluyendo 12 pruebas nuevas/dirigidas, `node --check`, gate `028A-18` PASS local-light (full diferido por cooldown) y `git diff --check` PASS. No se afirma publicación upstream ni paridad multi-proyecto.

- [x] Cerrar dónde vive el manifest de transición local (`quality-adapter.json`); el contrato final upstream queda pendiente.
- [ ] Diseñar y publicar en Sentinel el schema de `project adapter manifest`.
- [ ] Implementar un adapter de referencia Node/Vite y otro Rust/PostgreSQL en fixtures pequeñas.
- [x] Definir versión, hash, capabilities, timeout, env allowlist, output schema y exit-code mapping en el manifest local.
- [x] Definir que los comandos usan argv estructurado, sin shell concatenado, y que el adapter no puede
      crear scheduler, cooldown, claim, worktree o reporter final.
- [x] Crear fixtures locales PASS/findings y regresiones de timeout, cancellation, paths inseguros, malformed transport y missing tool.
- [x] Mantener `sentinel check --stages` como transporte de transición.
- [ ] Documentar una guía de cinco minutos para añadir un tercer proyecto sin copiar scripts.

**Pendiente upstream:** schema publicado/fijado, fixtures multi-proyecto Node/Vite + Rust/PostgreSQL, paridad CLI/LSP/editor, rollback y dos releases consecutivos.

**Gate:** dos fixtures de stacks distintos usan el mismo core; ningún adapter implementa scheduler/reporter
propio; CLI/LSP/editor siguen funcionando; la matriz de contratos y códigos de salida está versionada.

### Fase 2 — Migración de capacidades universales (ID: `SNT-14`)

**Orden obligatorio:** contratos/runner → scope/cache → scheduler/lease → report/retention → task coordinator.

- [ ] Mover cada capacidad desde su implementación local al core upstream mediante cambios pequeños y fixtures.
- [ ] Para cada capacidad, mantener un shim de compatibilidad que delegue, no una segunda implementación.
- [ ] Comparar resultados old/new en modo `observe` sobre cinco tareas reales identificadas por ID, dos ramas
      y un segundo fixture/proyecto; guardar decisiones, findings normalizados y estados de herramienta.
- [ ] Medir duración, cache hits/misses, RSS, archivos analizados/reutilizados y decisiones de error.
- [ ] Definir PASS como: mismo exit code semántico, mismos `ruleId:file:line:severity:message` después de
      normalización, ningún `tool-error` convertido en PASS, y cero divergencias sin explicación aprobada.
- [ ] Retirar duplicación solo después de dos releases consecutivos sin divergencia.
- [ ] Probar rollback: instalar el runtime anterior desde su hash, repetir una tarea canaria y verificar que
      alias, reporte y exit code vuelven al valor previo sin tocar perfiles ajenos.

**Gate:** paridad exacta de decisión y hallazgos, o divergencia explicada y aprobada; rollback a la versión
previa del runtime probado con evidencia reproducible.

### Fase 3 — Reducir el adapter de wandori.us (ID: `SNT-15`)

- [ ] Convertir las etapas de Rust, frontend, docs, custom y VarSense a manifest + adapters delgados.
- [ ] Reemplazar `task-check.mjs` por delegación a `sentinel check`, conservando el alias npm.
- [ ] Hacer que `run-with-db` solo resuelva DB/target y ejecute el comando permitido; el lease/guard universal
      debe venir de Sentinel.
- [ ] Retirar imports locales de cooldown, report, cache y takeover del adapter del consumidor.
- [ ] Mantener únicamente los comandos públicos mínimos en `package.json`: `task:check`, `quality:doctor`,
      `quality:test` (contratos del adapter) y operaciones explícitas de diagnóstico.
- [ ] Actualizar README/AGENTS para que un agente normal aprenda primero `sentinel task` + `sentinel task gate`.

**Gate:** un clon limpio con runtime global fijado ejecuta el mismo gate sin depender de `scripts/quality` para
scheduler/report/guard; el alias produce el mismo report JSON/Markdown y exit code.

### Fase 4 — Retirada controlada y simplificación (ID: `SNT-16`)

- [ ] Marcar wrappers duplicados como deprecated con versión inicial/final y mensaje de migración.
- [ ] Probar dos releases consecutivos en local, CI, PowerShell 5/7, CMD y Bash/Git Bash.
- [ ] Confirmar que rutas absolutas y shells sin perfil siguen documentadas como límites del launcher, no como
      cobertura falsa.
- [ ] Ejecutar `sentinel task gc --dry-run`/cleanup y demostrar cero worktrees/locks propios pendientes.
- [ ] Retirar físicamente solo archivos sin referencias y con rollback documentado; no tocar scripts operativos
      fuera de esta iniciativa.
- [ ] Archivar este plan cuando la retirada esté completa y enlazar la lección resultante.

**Gate:** el proyecto sigue cerrando tareas con el camino corto; el rollback instala el runtime anterior y
reactiva el alias sin restauraciones manuales peligrosas.

## 6. Política de permanencia para scripts

Un script se conserva si cumple al menos una de estas condiciones:

- encapsula una operación de dominio o proveedor autorizada que no existe en otros proyectos;
- es un adapter de una herramienta externa con contrato estable y tests;
- ofrece una experiencia humana/IDE que Sentinel no debe absorber;
- es necesario para bootstrap reproducible y está versionado, acotado y sin secretos;
- tiene un segundo consumidor real y una abstracción común reduciría más complejidad de la que añade.

Un script se migra si:

- la capacidad aparece en dos proyectos o dos adapters con la misma semántica;
- decide coordinación, política, caché, reporte, leases o interceptación;
- puede expresarse como contrato declarativo sin conocer rutas/dominio;
- su duplicación ya produce divergencias observables.

Un script se archiva/retira si:

- solo resuelve un incidente histórico o sitio ajeno;
- duplica un shim/runtime global ya publicado;
- contiene secretos, endpoints o supuestos de una máquina;
- no tiene referencias ni owner y no puede ejecutarse reproduciblemente.

**Regla anti-abstracción:** un segundo caso real es requisito para extraer un adapter compartido, pero la
coordinación universal es una excepción: claims, locks, worktrees, gate y reporte deben ser únicos desde el
primer proyecto para evitar carreras y dos autoridades.

## 7. Seguridad y no-sorpresas

- No copiar tokens, cookies, IPs privadas, credenciales o endpoints autenticados a la nueva arquitectura.
- La auditoría de secretos es un bloqueo de Fase 0: antes de inventariar contenido sensible se debe aislar
  el archivo, revocar/rotar la credencial por el canal autorizado y registrar solo la evidencia no sensible.
- Auditar y revocar cualquier secreto expuesto en scripts antes de moverlos o documentarlos.
- No usar shell concatenado en manifests; argumentos separados y allowlists.
- No ejecutar scripts de deploy/rescate/producción durante esta migración.
- No borrar `scripts/quality` hasta demostrar paridad, rollback y dos releases.
- Mantener el submódulo `tools/sentinel` serializado; no mezclar esta iniciativa con cambios upstream ajenos.
- Toda modificación al core universal ocurre upstream, con fixtures, commit publicado, gitlink/lock del
  consumidor y gate del consumidor.

## 8. Métricas de éxito

- **Complejidad para el agente:** camino recomendado reducido a `sentinel task ...` + `sentinel task gate`;
  no más de cinco comandos de calidad visibles en la guía principal.
- **Tamaño del adapter:** objetivo inicial ≤10 archivos de integración y ≤600 líneas propias, sin contar
  reglas de producto ni helpers de DB.
- **Dedupe:** cero implementaciones locales de scheduler/cooldown/report/task coordinator al cerrar SNT-15.
- **Paridad:** 100% de decisiones y findings normalizados en cinco tareas reales, dos ramas y dos stacks.
- **Reproducibilidad:** clon limpio + submódulos + runtime hash verificado reproduce el gate sin rutas
  absolutas ni variables `GLORY_*_SOURCE_PATH`.
- **Portabilidad:** matriz soportada declarada; límites de shells/rutas absolutas documentados y testeados.
- **Seguridad:** cero secretos nuevos en git, logs o reports; manifests sin comandos interpolados.
- **Operación:** cleanup/GC deja cero worktrees/locks propios y no toca recursos ajenos.

## 9. Riesgos y decisiones abiertas

| Riesgo                                             | Decisión/mitigación                                                     |
| -------------------------------------------------- | ----------------------------------------------------------------------- |
| Core absorbe demasiada lógica de un proyecto       | contrato declarativo + fixtures de dos proyectos antes de generalizar   |
| Adapter se convierte en segundo orquestador        | prohibir scheduler/reporter/claim local; review y regla Sentinel        |
| Retirada prematura rompe agentes                   | dos releases, alias temporal, rollback de runtime y matriz multi-shell  |
| Muchos comandos confunden a agentes                | una guía corta; mantenimiento/bench fuera del camino normal             |
| Scripts operativos contienen secretos              | no migrar; tratar como incidente y usar Coolify Manager                 |
| Capacidad de Cargo/DB no es universal              | mantener adapter Rust/Postgres, generalizar solo el contrato de proceso |
| Cambiar el runtime y el consumidor simultáneamente | publicar/fijar upstream primero, luego adapter y lock del consumidor    |
| GC borra recursos activos                          | dry-run, PID/TTL, path interno, nunca borrar ante evidencia ambigua     |

Decisiones que deben cerrarse antes de SNT-13:

1. ¿El manifest vive dentro de `sentinel.config.json`? Recomendación: sí, sección `adapter`, para conservar
   un único descubrimiento de política; abrir archivo separado solo con una razón de versionado demostrable.
2. ¿El adapter se distribuye como paquete versionado, submódulo o scripts locales pequeños? Recomendación:
   contrato en Sentinel + adapter local delgado mientras no exista un segundo consumidor real.
3. ¿Qué parte de la salida Markdown es core y qué parte es template del proyecto? Recomendación: JSON/exit
   code/estados en core; Markdown puede tener template declarativo de proyecto.
4. ¿El command runner acepta solo arrays de argv y env allowlist? Decisión recomendada: sí; no se acepta
   shell concatenado.
5. ¿El segundo proyecto de conformidad será Node/Python o Rust sin PostgreSQL? Elegir uno que no comparta
   rutas ni dominio de wandori.us y fijar el criterio antes de implementar.

## 10. Definition of Done global

No se declara esta migración terminada hasta que:

- [ ] el inventario de scripts está completo y cada archivo tiene destino/owner;
- [ ] Sentinel Core publica el contrato de adapter y sus fixtures;
- [ ] dos proyectos de stacks distintos usan el mismo core sin copiar `scripts/quality`;
- [ ] VarSense se ejecuta como analyzer y no crea gate/reporte/cooldown paralelo;
- [ ] wandori.us conserva sus adapters de Rust/frontend/docs/custom sin rutas universales hardcodeadas;
- [ ] `task-check.mjs` y wrappers duplicados delegan o quedan retirados con evidencia;
- [ ] cinco gates reales muestran paridad de decisión/hallazgos y los errores de herramienta son fail-closed;
- [ ] dos releases consecutivos pasan la matriz multi-shell/CI;
- [ ] rollback y cleanup/GC están probados;
- [ ] documentación/roadmap/planes/completados reflejan el estado real;
- [ ] la skill global se actualiza **solo al final**, con una regla agnóstica respaldada por evidencia,
      versión nueva, fixture/regresión y confirmación en una sesión posterior;
- [ ] no quedan secretos, procesos, locks, worktrees ni ramas propias pendientes.

## 11. Cierre documental y actualización de la skill global

La skill global no se modifica durante Fases 0–3. Al finalizar Fase 4 se debe hacer una revisión explícita:

- Si la experiencia demuestra una regla generalizable (por ejemplo, “un adapter no puede poseer scheduler,
  reporter o claim propio”), se crea primero una prevención/fixture y se actualiza la skill con versión/fecha
  nueva y rollback.
- Si la evidencia solo describe este repositorio, se actualizan `AGENTS.md`, este plan y la prevención local,
  pero **no** la skill global.
- La actualización debe conservar una copia, mostrar el diff, ejecutar la suite del proyecto y la suite de
  Sentinel, publicar/fijar el cambio si afecta upstream y confirmarse en una sesión nueva.
- Hasta entonces, la skill activa sigue siendo la autoridad vigente y este plan queda como propuesta auditable.
