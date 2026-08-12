# Runbook — Retirada de los wrappers del repo (028A-6 Fase 5 residual)

> **Fecha:** 2026-08-05
> **Plan canónico:** `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md` (Fase 5).
> **Criterio:** este runbook NO se ejecuta hoy; se ejecuta cuando dos releases consecutivas hayan pasado la matriz en verde (ver §3). Sirve para que esa ejecución sea mecánica, verificada y reversible.
> **Estado (2026-08-11):** Sentinel **0.7.1** (`b22c848`, tag `v0.7.1`) y VarSense **2.2.1** (`88f281f`,
> tag `v2.2.1`) son la segunda release adoptada por wandorius y glory-rs-rest. Setup, lock, doctor y
> suites pasan; el rollback real `0.7.1 → 0.7.0 → 0.7.1` quedó verificado. La retirada física de la
> capa A todavía no se ejecuta porque §3 exige además dos CI consecutivos verdes, matriz multi-shell y
> gate verde en cada consumidor; glory-rs-rest conserva el baseline `broadcast-mutex-riesgo-rs`.

## 1. Objetivo y contexto

El runtime global de Sentinel (`%LOCALAPPDATA%\GlorySentinel`, `sentinel install`) ya es la
**única fuente** de shims/guards: genera `npm/npx/cargo/node.cmd` + guards de bash/PowerShell en
`<target>/shims` y los expone en PATH (`shims;bin`) y perfiles. Los wrappers del repositorio
(`scripts/quality/*.cmd`, `global-cargo-guard.ps1`, `global-quality-guard.sh`) quedaron
**marcados como LEGACY** (banners `[028A-6 Fase 5]`) y solo se conservan para ramas antiguas.

Esta retirada elimina esa copia duplicada del repo. **No** elimina el orquestador local
(`task:check` + `heavy-run-guard.mjs` + adapters): ese es la capa B y se retira en el gate SNT-10
("`sentinel check` es la única autoridad de cierre"), cuando `task:check` delegue por completo.

## 2. Capas

| Capa | Archivos | Cuándo se retira |
| --- | --- | --- |
| **A — shims duplicados por el runtime** | `scripts/quality/npm.cmd`, `npx.cmd`, `cargo.cmd`, `node.cmd`, `global-cargo-guard.ps1`, `global-quality-guard.sh`, `install-global-guard.ps1`, `quality-command-guard.mjs` (+ `tests/quality-command-guard.test.mjs`) | **Este runbook** (dos releases verdes) |
| **B — orquestador local del gate** | `task-check.mjs`, `heavy-run-guard.mjs`, stages/adapters/reporter/cache/scope… | Gate SNT-10 (delegación a `sentinel check`) |

**Dependencias verificadas (2026-08-05):** ningún `.mjs` de `scripts/quality` importa
`quality-command-guard.mjs`; solo lo referencian los shims del repo (capa A), sus tests y
`install-global-guard.ps1`. `heavy-run-guard.mjs`/`runner.mjs`/`adapters/common.mjs` usan
`npm.cmd`/`cargo.cmd` como **nombre del ejecutable real de Windows**, no como los shims del repo:
no dependen de la capa A.

## 3. Criterio de las dos releases (operativo)

> **Criterio único de retirada (fuente canónica):** no se retira la capa A ni la capa B hasta tener
> dos CI consecutivos verdes en `main`, matriz multi-shell verde en las releases, `task:check` PASS
> con PATH completo y sin runtime de desarrollo, gates verdes en todos los consumidores y rollback
> verificable. Inventario y planes enlazan esta sección; una segunda release publicada por sí sola no basta.

Marcar como cumplido SOLO cuando se cumplan **todas**:

- [x] El runtime global v0.7.1 está instalado y `sentinel doctor` reporta `activeVerified:true`.
- [ ] **Dos ejecuciones CI consecutivas en `main`** (`quality.yml`) terminan en verde, con artifacts `quality-reports-<branchKey>-<task>-<commit>` y `quality-metrics-<branchKey>-<commit>` publicados.
- [ ] La matriz multi-shell del runtime (`shellMatrix.test.ts` + `guardMatrix.test.ts` en `tools/sentinel`) pasa en las dos releases (suite del submódulo, `npm test`).
- [ ] `task:check` PASS con el PATH completo (shims+bin) y también con un PATH sin `GlorySentinel` (evidencia CI sin perfil dev), como en la Fase 4.

## 4. Pre-verificación (en la rama donde se ejecute)

```bash
# 1. Runtime activo y verificado
sentinel status --json          # activeVersion 0.4.x, activeVerified true
sentinel doctor

# 2. Gate local sano antes de tocar nada
npm run quality:test
npm run task:check -- <task-id>   # PASS

# 3. Matriz del runtime verde (submódulo)
cd tools/sentinel && npm test && npm run check:core && cd ../..

# 4. Sin referencias vivas a la capa A fuera de sí misma
grep -rln "quality-command-guard\.mjs" scripts/ tools/ .github/ --include="*.mjs" --include="*.ts" --include="*.sh" --include="*.ps1" | grep -v "scripts/quality/global-\|scripts/quality/npm\.cmd\|scripts/quality/npx\.cmd\|scripts/quality/cargo\.cmd\|scripts/quality/node\.cmd\|tests/"
# Esperado: vacío (o solo el propio test a retirar)
```

## 5. Retirada (capa A)

```bash
git rm scripts/quality/npm.cmd \
       scripts/quality/npx.cmd \
       scripts/quality/cargo.cmd \
       scripts/quality/node.cmd \
       scripts/quality/global-cargo-guard.ps1 \
       scripts/quality/global-quality-guard.sh \
       scripts/quality/install-global-guard.ps1 \
       scripts/quality/quality-command-guard.mjs \
       scripts/quality/tests/quality-command-guard.test.mjs
```

**Conservar (capa B, NO retirar):** `heavy-run-guard.mjs`, `task-check.mjs`, `policy-*.mjs`,
`runner.mjs`, adapters, `install-global-runtime.mjs` (sigue siendo el instalador/desinstalador del
runtime y retira la entrada legacy del PATH por nombre, no por archivo).

Ajustes posteriores obligatorios:
- [ ] `scripts/self-check.ps1` y cualquier documento que cite `quality-command-guard` como comando: apuntar a `sentinel guard`.
- [ ] `AGENTS.md` §11 (herramientas obligatorias): quitar la mención a los shims del repo si existe; el guard vive en el runtime.
- [ ] Plan 028A-6 Fase 5 y `roadmap-sentinel.md` SNT-10: marcar "Eliminar shims duplicados" con el commit de retirada y esta verificación.

## 6. Verificación post-retirada

```bash
npm run quality:test                    # 210/210 esperado (sin el test retirado: n-1)
npm run task:check -- <task-id>         # PASS (el gate local NO usa la capa A)
cmd //c "npm.cmd --version"             # debe resolver el npm real de Windows (no el shim del repo)
powershell -NoProfile -c "npm --version"   # sin interceptor del repo; el runtime sigue en el PATH/perfiles
bash -lc "npm --version"                # el guard del runtime (dot-source global) sigue cargando
cd tools/sentinel && npm test && npm run check:core && cd ../..
```

El enforcement sigue activo vía el runtime: en un repo con `sentinel.config.json` `enforce`,
`npm run test` desde una shell normal debe seguir bloqueando con 78 (los shims globales del runtime
interceptan). Si NO bloquea, el PATH/perfiles no tienen los shims del runtime: abortar y revisar
`sentinel doctor` antes de continuar.

## 7. Rollback

```bash
# 1. Restaurar los archivos desde git (sin tocar el runtime instalado)
git checkout HEAD~1 -- scripts/quality/       # o: git restore --source=<commit-anterior> scripts/quality

# 2. Reinstalar el runtime por si acaso (idempotente; nunca rompe la rama)
node scripts/quality/install-global-runtime.mjs --dry-run   # revisar
node scripts/quality/install-global-runtime.mjs             # aplicar

# 3. Verificar
sentinel doctor && npm run quality:test && npm run task:check -- <task-id>
```

El rollback del runtime en sí (versión anterior + backups de perfil) está documentado y probado
en el plan Fase 5 (demo 14/14, restauración byte a byte): `sentinel rollback` + copia del backup
de `<target>/shims/profile-backups` sobre el perfil.

## 8. Criterio de salida (todo junto)

- [ ] `git rm` aplicado solo a la capa A; `git status` sin archivos ajenos.
- [ ] Gate local PASS, suite del orquestador verde, matriz del submódulo verde.
- [ ] Shell nueva: los shims del runtime bloquean 78 en un repo enforce y `sentinel` resuelve.
- [ ] Ninguna rama activa pierde la capacidad de ejecutar su gate (el runtime es global; la capa A era copia).
- [ ] Commit `028A-6: Fase 5 - wrappers del repo retirados (criterio de dos releases cumplido)` + tarea liberada.
