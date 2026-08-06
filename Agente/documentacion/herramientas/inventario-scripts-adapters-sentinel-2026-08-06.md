# Inventario — scripts, adapters y Sentinel

> **Fecha de corte:** 2026-08-06
> **Tarea:** SNT-12 / SNT-14 / SNT-15
> **Workspace inventariado:** consumidor `wandorius`, con transición ejecutada en worktree Sentinel `task/157fb8a2b2a4e1dc-SNT-15`
> **Estado:** Fase 0 cerrada; Fase 1 y Fase 2 local de transición cerradas; reducción del adapter bloqueada
> **Fuente canónica:** `Agente/planes/plan-migracion-scripts-adapters-sentinel-2026-08-06.md`

## 1. Alcance y evidencia

El inventario se construyó desde archivos versionados, no desde el contenido accidental de una máquina. La clasificación conserva scripts específicos y wrappers legacy hasta contar con evidencia de retirada.

- 122 archivos bajo `scripts/`.
- 8 scripts de raíz/compatibilidad.
- 70 scripts, wrappers, parches y utilidades bajo `scripts/quality/`.
- 44 tests bajo `scripts/quality/tests/` en la línea base.
- 9 adapters bajo `scripts/quality/adapters/`.
- Sentinel fijado en `0.5.0`, commit `20c13a216e879303fcf5be7469a2821391b2ec0d`.
- VarSense fijado en `2.2.0`, commit `e8360927ee92c4067f1f501dd77b951c8bc4f61d`.
- Rama primaria: `wandorius`.

El inventario no copia secretos ni declara owners históricos inexistentes. La búsqueda focalizada no encontró tokens literales/credenciales; esto no sustituye un scanner dedicado.

## 2. Destino provisional

- **Core candidato:** scope/cache/runner/lease/report/coordinación, solo tras API agnóstica y fixtures.
- **Adapter:** Rust/PostgreSQL, frontend/Vite, docs/custom y VarSense.
- **Diagnóstico/mantenimiento:** observe, benchmarks, profile, cleanup y target/index maintenance.
- **Legacy/retirada condicionada:** shims globales, wrappers de compatibilidad y scripts históricos/operativos, sin borrado automático.

## 3. Evidencia de Fase 1 local (SNT-13)

`quality-adapter.json` y `adapter-manifest.mjs` definen transporte argv, stages, profiles, timeouts, allowlist, schema y exit codes. `stages.mjs` genera declaraciones para `sentinel check --stages`; `stage-process.mjs` produce JSON v1; `observe-compare.mjs` mantiene run-id, namespace aislado y metadata fresca.

Evidencia integrada previa: **217/217 PASS**, `node --check`, `git diff --check`, gate local-light. No equivale a schema upstream ni paridad multi-proyecto.

## 4. Evidencia de Fase 2 local (SNT-14/SNT-15)

- El manifest pasó a ser la fuente de nombres/perfiles de stage; unknown profiles/stages, claves de stage, y deriva manifest/implementación fallan cerrado.
- `quality-adapter.json` y el entrypoint se validan como archivos regulares contenidos físicamente; symlink/junction y realpath fuera del workspace se rechazan, también en la ruta síncrona del gate.
- El runner local conserva una allowlist base no sensible; el validador rechaza variables sensibles (`DATABASE_URL`, tokens, keys, passwords y equivalentes), también cuando llegan desde la base heredada.
- La allowlist efectiva deduplica nombres sin distinguir mayúsculas/minúsculas, evitando ambigüedad `PATH`/`Path` en Windows.
- El transporte sigue separado por argv y conserva `shell:false` para procesos normales; los paths/reportes/task IDs permanecen contenidos.
- La ruta normal `task-check` mantiene compatibilidad, pero `stageDefinitions` consume el manifest validado automáticamente; no se declara todavía que Sentinel sea el único orquestador.

Evidencia reproducible en el worktree SNT-15:

```text
node --check scripts/quality/adapter-manifest.mjs
node --check scripts/quality/stage-definitions.mjs
node --test scripts/quality/tests/adapter-manifest.test.mjs \
  scripts/quality/tests/stage-definitions.test.mjs \
  scripts/quality/tests/profile-contract.test.mjs
19 tests: 19 PASS, 0 FAIL
git diff --check: PASS
```

Las fixtures cubren manifest enlazado fuera del workspace, schema estricto, allowlist base/efectiva, selección real desde disco, manifest inválido y stage sin factory. El observe end-to-end y el gate completo no se declaran PASS: el worktree no tiene `.env`/`DATABASE_URL`, `frontend/node_modules` ni CLI compilado de VarSense. El gitlink histórico de `glory-rs` no está disponible en el remoto actual; no se sustituye ni se fuerza un commit distinto. No se copian secretos ni se falsea el resultado.

## 5. Criterio y límites

- [x] Archivos versionados clasificados por grupo/destino.
- [x] Consumers directos de package/CI/frontend identificados.
- [x] Capacidades universales separadas de adapters y diagnóstico.
- [x] Owners provisionales declarados sin inventar propietarios reales.
- [x] Contrato local estricto con validación física, seguridad de entorno y paridad de stages.
- [ ] Owners reales y frecuencia histórica confirmados por el equipo.
- [ ] Schema upstream y contrato final acordados/publicados.
- [ ] Fixtures de dos stacks independientes y paridad CLI/LSP/editor.

**Conclusión:** el inventario y SNT-15 no autorizan retirar `scripts/quality`, modificar la skill global ni publicar cambios upstream sin una tarea explícita y un gate propio.
