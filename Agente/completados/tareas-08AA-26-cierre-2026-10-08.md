# 08AA-26 (cierre) — Reparar reglas Sentinel: promise, mixed-barrel, large-interface, inline-style

> Nota de archivo: `roadmap.md` y `Agente/completados/tareas-2026-10-08.md` estaban sucios
> por la sesión concurrente (08AA-28/29/30) al cerrar este bloque; para no mezclar frentes,
> el cierre vive en este fichero. La entrada parcial previa sigue en
> `Agente/completados/tareas-2026-10-08.md` (sección `08AA-26 (parcial)`).

## Qué
Resto de 08AA-26 (ver entrada parcial para ruta-post/path-join/key-index + fix lastIndex):
4 FPs front corregidos en `glory-sentinel` (rama local `fix/08AA-26-reglas-fp`, commit `748a387`,
NO pusheada a GitHub — repo upstream, requiere autorización): (1) `promise-sin-catch` exime
`lazy()` multilínea (`import().then` hasta 5 líneas después de `lazy(`) + soporte
`sentinel-disable-next-line` para catch-interno justificado; (2) `mixed-barrel-logic` exige ≥2
re-exports (un único re-export = alias de compatibilidad); (3) `large-interface-isp` exige ≥1
método (DTOs espejo de API exentas). En MN: 2 disables justificados (catch-interno
`use-clientes.ts`) + fix TP `inline-style` en `pagina-ask.tsx` vía CSS var `--progreso`
(primer uso del patrón [054A-19] en el proyecto). Veredictos documentados de no-cambio:
`sqlite-carga-N` (TP en bucle ajeno) y `componente-sin-hook` ×5 (TPs regla 8, refactor pendiente).

## Archivos
- `glory-sentinel/src/analyzers/react/reactErrorRules.ts` (lazy-multilínea + disable),
  `src/analyzers/static/portableRules.ts` (reExports ≥2, métodos ≥1),
  `src/test/suite/promiseSinCatch.test.ts` (nuevo, 5 tests),
  `src/test/suite/portableRules.test.ts` (+3 tests, Payload actualizado a forma con métodos).
- MN: `frontend/src/features/chat/clientes-duena.tsx` (2 disables),
  `frontend/src/features/ask/pagina-ask.tsx` (CSS var `--progreso`),
  `Agente/prevencion/prevencion-sentinel-fp-frontend-08AA-26-2026-10-08.md` (nuevo).
- `roadmap.md` bloque 08AA-26 actualizado a CERRADO (commit propio por hunk, sin tocar
  hunks ajenos 08AA-30/31). `sentinel.lock.json` queda en 0.7.13 a propósito: decisión
  adoptada SEGUIR-POR-FUENTE (ver Pendientes reales).

## Evidencia
- `npx tsc -p ./` 0 + eslint 0 errors (tests ignorados por config, como el resto) en glory-sentinel.
- Mocha subsets 11/11 + **full `npx mocha --reporter min`: 749 passing, 1 pending, EXIT 0**
  (`C:\tmp\mocha-full-08AA26b.log`; el EXIT:1 del run anterior fue flaky de Git Bash en
  shellMatrix, no de los cambios: aislado pasa en base y rama).
- CLI contra fixtures `C:\tmp\fix-08AA-26b`: Bien 0 findings, Mal 3/3
  (`promise-sin-catch`, `mixed-barrel-logic`, `large-interface-isp`).
- Propagación: `.quality-tools/sentinel` `fetch`+`checkout 748a387`+`npm run compile`
  (verificado por strings `esLazyMultilinea`/`reExports` en `out/`, queda en detached HEAD).
- MN: `npm --prefix frontend run type-check` (`tsc -b`) 0 tras ambos cambios.
- Re-gates MN (`POST /api/gate/analizar forzar:true`): 0E/443W/3H → 0E/440W/0H (reglas) →
  **0E/439W/0H** (tras fix inline-style). Las 7 reglas del alcance a 0
  (`ruta-post`, `path-join`, `key-index`, `promise`, `mixed-barrel`, `large-interface`,
  `inline-style`); deltas +2/+1/-1 en `sqlx-*`/`handler` son churn de la sesión concurrente
  (ficheros ajenos intactos por este bloque).

## Gotchas
- (1) `tieneSentinelDisable(lineas, i, id)` mira 5 líneas ATRÁS desde la reportada: el
  disable va justo encima del `.then`, no del `onClick`. (2) `as CSSProperties` necesita
  `import type { CSSProperties } from 'react'` (las custom props no existen en el tipo).
  (3) `w-[var(--progreso)]` compila estático en Tailwind: el % dinámico viaja por la var.
  (4) `countInterfaceFields` nunca contó métodos: el test viejo `Payload` (11 campos, 0
  métodos) pasó a Bien; el Mal ahora lleva 2 firmas de método. (5) No hacer `git add` de
  `tareas-2026-10-08.md` mientras la otra sesión lo tenga sucio: el cierre va en fichero
  propio.

## Pendientes reales (estado al cierre del turno Auto)
- HECHO: push de `fix/08AA-26-reglas-fp` (`748a387`) a `origin` (solo la rama) +
  registro del cierre en `roadmap.md` (commit por hunk propio).
- DECISIÓN ADOPTADA — lock SEGUIR-POR-FUENTE (recomendada, reversible): `sentinel update
  --dry-run` instalaría topología `versions/` ajena al consumo por checkout compartido
  (`.quality-tools/sentinel` por path, ver `proveedor.ts:46-49`) y nada del gate lee la
  versión del lock (`leerSentinelLock` solo lo parsea como manifiesto). No alinear a mano
  (cosmético) ni migrar topología sin pedido explícito. Si ella pide pin por versión,
  es migración deliberada, no parte de 08AA-26.
- FUTURO (fuera de 08AA-26, sin ID para no colisionar con la otra sesión): extraer 5 hooks
  (`useHiloMensajes`, `useSesionesWhatsapp`, `usePestanaIA`, `useTarjetaFotoMejora`,
  `useModalDescargarFotos`) — TPs `componente-sin-hook` regla 8.

- **Sentinel:** 3 reglas reparadas + 8 tests + 1 MD de prevención nuevo. **GLORY:** no aplica.
