# Falso positivo: sentinel-disable-file no suprime detección componente-sin-hook

## Contexto
Los componentes `SubTabServicios.tsx`, `SubTabBlog.tsx`, `SubTabProyectos.tsx` y `SubTabEquipo.tsx` tienen el comentario:
```
/* [084A-22] ... 
 * sentinel-disable-file componente-sin-hook: Los callbacks son wiring trivial que
 * delega en useContenido* hooks existentes. No justifica un hook intermedio. */
```

## Problema
Code Sentinel sigue reportando "Componente con N lineas de logica. Extraer a hook dedicado" a pesar del `sentinel-disable-file`.

## Corrección necesaria en Code Sentinel
El analyzer de React (`reactAnalyzer.ts` o `react/componentLogicDetector.ts`) debe:
1. Leer el archivo buscando patrones `sentinel-disable-file {regla}` en comentarios
2. Si la regla deshabilitada coincide con la detección a reportar, omitirla
3. El match debe ser por nombre de regla (`componente-sin-hook`) no por texto exacto

## Estado
Pendiente de implementar en `.agent/code-sentinel`.

## Actualizacion [259A-5] 2026-09-26
Soporte parcial añadido ([104A-4]): solo `verificarComponenteSinHook`
(`.quality-tools/sentinel/src/analyzers/react/reactComponentRules.ts:200-202`)
honra `sentinel-disable-file componente-sin-hook` via substring match. El resto de
reglas React lo ignoran por completo (`reactAnalyzer.ts` no consulta
`tieneSentinelDisableFile`; cada `verificar*` tampoco):

- MUERTOS (file-level y next-line ignorados): `html-nativo-en-vez-de-componente`
  x37, `menu-contextual-override-diseno` x23, `modal-con-titulo` x2,
  `modal-acciones-no-canonico` x1, `componente-artesanal` x4,
  `modal-semantica/estructura-no-canonica` x10. Verificado: `analyze.json`
  conserva los findings en las mismas lineas (+1 por las lineas de comentario
  insertadas) tras colocar 30 disables con la convencion documentada.
- VIVOS (por substring accidental): `componente-sin-hook-glory` x2 suprimidos
  porque el comentario contiene `sentinel-disable-file componente-sin-hook-glory`,
  que incluye como substring `sentinel-disable-file componente-sin-hook`.
- Precedentes igualmente muertos: `UsuarioPublicoIsland.tsx:4` (file-level html),
  next-lines en `Header.tsx`/`SeccionPerfil.tsx`/`ChatWidget.tsx`.

Fix propuesto (scope herramienta compartida, NO aplicar desde consumidor sin
propagacion via quality:bump): en cada `verificar*` de `reactComponentRules.ts`
anadir al inicio el mismo guard de [104A-4] con el ID exacto de su regla, mas
soporte `sentinel-disable-next-line` como en `staticAnalyzer.ts:331,417`.
Ojo: hacer el match por token exacto; el substring actual hace que el disable de
`componente-sin-hook-glory` suprima tambien `componente-sin-hook` y viceversa.
Mientras tanto, la fase 5b elimina los findings html/menu/modal de raiz
(migracion a DS), por lo que los disables muertos actuan como marcadores.

## Resolucion [259A-5] 2026-09-27 — sentinel 0.7.14 (commit `b00747c`)
Causas raiz confirmadas (3, no 1): (1) `tieneSentinelDisable` rompia el scan
al encontrar una linea de codigo (`break`), asi que un `next-line` sobre el
contenedor nunca alcanzaba la violacion interna (caso SeccionPagos: disable en
117, finding en 119); (2) match por substring en `sentinel-disable-file`
(supresion cruzada `componente-sin-hook` <-> `componente-sin-hook-glory`);
(3) la mayoria de `verificar*` React no tenia guard file-level.

Fix en herramienta compartida (NO en consumidor): `tieneSentinelDisable`
reescrito a ventana de 5 lineas sin `break` (atraviesa codigo); nuevo helper
`tieneSentinelDisableFile(texto, reglaId)` con match por token exacto por
linea (multi-id, `:`/`--` como separadores; bare `sentinel-disable-file` no
exime nada); guards migrados en `reactComponentRules.ts` (html, button,
artesanal, mutacion, key-index, sin-hook x2 grafias, menu, update-optimista,
cola, objeto-mutable), `reactModalRules.ts` (titulo/acciones/estructura) y
`formularioConfigSystem.ts`.

Evidencia: `src/test/suite/sentinelDisableContrato.test.ts` (28 tests;
incluye replicas SeccionPagos:117 y :218); suite completa 700 pass + 1 skip
(mocha colgado en este shell -> shim TDD `C:\tmp\sentinel-shim-run.cjs`);
`check-core` OK; `analyze` sobre NAKOMI con el build fix: `modal-con-titulo`
0, `modal-acciones` 0, `menu` 0, `artesanal` 0, `html` 0, `sin-hook-glory` 0,
`key-index` 0 (restan 5 `modal-estructura` legitimos en ModalSeoEdit sin
disable — baseline :86,108,131,171,185).

Propagacion via `quality:bump --tool sentinel --write`: pin+lock+setup+doctor
OK y commiteado en 7/12 (Glory-Laminal, gloryapi, PROYECTO TASKS,
coolify-manager-rs, GLORYINSPECTOR, GLORYPORT, NAKOMI `647b0603`); guard
`quality-sync` confirma sentinel alineado @b00747c en ellos. Bloqueados por
causas pre-existentes ajenas al fix (sin commit): WANDORIUS/RESTAURANTE
(sourcePath externo / version ilegible, familia B), ONG AGAPE/freebuff-bridge/
workspace-manager (desync varsense: checkout d9185c0 vs pin 40fed9a); sus
edits de pin se revirtieron salvo ONG AGAPE (pin b00747c = codigo que
realmente corre por checkout compartido; su lock 0.7.13 es dirt pre-existente).
Estado: RESUELTO para reglas React; pendiente genérico pasa a varsense-desync.
