# Plan — Iconos del escritorio: grid, placeholder y rejilla de debug coherentes

> **Fecha:** 2026-08-05 · **Estado:** planificado
> **Fuentes:** `frontend/src/features/desktop/workspace-icon-grid.ts`, `frontend/src/features/desktop/utils/icon-grid.ts`,
> `frontend/src/features/desktop/utils/icon-reorder.ts`, `frontend/src/features/desktop/utils/icon-drag.ts`,
> `frontend/src/features/desktop/utils/debug-grid-overlay.ts`, `frontend/src/styles/desktop/desktop-shell.css`,
> `frontend/src/styles/variables.css` (tokens `--sistema-icono-*`).

## Problema (reportado por el usuario, 05-ago)

El grid de iconos del escritorio "está mal", hay fallas, y el **placeholder** de arrastre (dónde se
va a poner el icono) junto con las **rejillas rojas de debug** (Ctrl+Shift+G) no son coherentes con
las celdas reales donde aterrizan los iconos.

## Causas raíz identificadas (por código)

1. **`justify-content: space-between` horizontal no se replica.** El grid declara
   `grid-template-columns: repeat(auto-fill, 88px)` con `justify-content: space-between` y
   `direction: rtl`. Con auto-fill el eje horizontal queda lleno solo si el ancho es múltiplo
   exacto de `(88 + gap)`; si sobra espacio, `space-between` lo reparte entre columnas y **ningún
   cálculo lo replica** — `getGridMetrics` calcula `columns = floor((width + gap)/(cell + gap))` y
   `getCellAt`/`positionCellHighlight`/`debugGridOverlay` usan `col * (cellWidth + columnGap)`
   (gaps uniformes). El highlight y la rejilla roja quedan desfasados de la celda real. Solo
   `rowGapEffective` (058A-1) replica la distribución vertical; falta el equivalente horizontal
   (**`columnGapEffective`**).

2. **`direction: rtl` duplica fórmulas y es fuente de desfase.** El grid es RTL (col 0 = derecha,
   crece a la izquierda) y hay **tres implementaciones** de la geometría RTL: `getCellAt`
   (`right - x`), `positionCellHighlight` (`right - (col+1)*cell - col*gap`) y `debugGridOverlay`
   (`rect.width - (col+1)*cell - col*gap`). Ya se corrigió una vez (297A-20: "antes se restaba un
   gap de más") pero siguen siendo fórmulas paralelas propensas a divergir. La incoherencia entre
   el highlight (positionCellHighlight) y la rejilla (debugGridOverlay) indica que divergen otra vez.

3. **`cellWidth` se mide desde el primer icono, no del track.** `getGridMetrics` toma
   `cellWidth = first.getBoundingClientRect().width`, que es `--sistema-icono-celda` (88px) solo
   porque `.desktop-icon--interactive { width: var(--sistema-icono-celda) }`. Si el item cambia de
   ancho (label largo, overflow, tema), la geometría del snap-grid deja de coincidir con los tracks
   reales del CSS grid. Debería medirse el **track declarado** (`gridTemplateColumns`) o derivarse
   de `(width - (columns-1)*gap) / columns`, no del item.

4. **Rejilla de debug es "DEPURACION TEMPORAL" que quedó pegada.** `debug-grid-overlay.ts` y
   `.desktop-icon-grid--depurar`/`__debug`/`__debug-celda` (borde rojo + col,row) están marcados
   "PENDIENTE: eliminar" (297A-20) pero siguen en producción con atajo Ctrl+Shift+G. Además su
   geometría usa el mismo `col * (cellWidth + gap)` sin `columnGapEffective`, así que **la propia
   herramienta de diagnóstico miente** sobre dónde caen las celdas.

5. **Sin tests de geometría real (DOM).** `icon-grid.test.ts` solo prueba lógica pura con métricas
   mockeadas; `getGridMetrics`/`getCellAt`/`positionCellHighlight` no tienen un test con un grid
   real montado (jsdom) que fije la geometría frente a `space-between` y RTL. La falla se cuela
   porque nadie verifica "el highlight cae exactamente sobre la celda real".

## Objetivo

Un único cálculo de geometría de celdas (con `columnGapEffective` y RTL) consumido por
`getCellAt`, el highlight de arrastre y la rejilla de debug, verificado con tests DOM sobre un grid
real, y sin código de depuración visible para el usuario final.

## Fases

### Fase 1 — Unificar la geometría del grid (columnaGapEffective)

- [ ] Añadir `columnGapEffective` a `GridMetrics` replicando la distribución horizontal de
  `justify-content: space-between/around/evenly` (mismo patrón que `rowGapEffective`).
- [ ] Medir `cellWidth` desde el track real: prioridad a `gridTemplateColumns` parseada
  (auto-fill → derivar track de `(width - (columns-1)*gap)/columns`), fallback al primer item.
- [ ] Extraer un helper único `cellOriginAt(col, row, metrics)` (LTR y RTL) y usarlo en
  `getCellAt`, `positionCellHighlight` y `debugGridOverlay.render` (eliminar las tres fórmulas
  paralelas).
- [ ] Tests DOM (jsdom): montar un grid real con `repeat(auto-fill, 88px)` + `space-between` + RTL,
  con y sin sobrante, y verificar que `getCellAt(celda real)` → `col,row` exacto y que
  `cellOriginAt` devuelve el origen del track (comparado con el rect del icono posicionado).

**Gate F1:** `cellOriginAt` es la única fuente de geometría; tests DOM verdes con sobrante
horizontal y RTL.

### Fase 2 — Coherencia del placeholder de arrastre

- [ ] Verificar en navegador que el highlight (`desktop-icon-drop-target`) cae exactamente sobre la
  celda destino al arrastrar (con y sin sobrante horizontal, desktop ≥769 y tablet).
- [ ] Ajustar el `transition: left/top` para que el placeholder no "baile" entre celdas con
  sobrante distribuido (comparar el origen calculado con el rect real del icono al soltar).
- [ ] Tests DOM: `updateHighlight` en modo placement produce un highlight cuyo rect coincide con la
  celda destino real (offset dentro del grid).

**Gate F2:** el placeholder coincide con la celda destino en desktop y tablet; tests verdes.

### Fase 3 — Rejilla de debug coherente o retirada

- [ ] Decisión: si la rejilla roja es herramienta interna de desarrollo, dejar de exponerla en
  build de producción (solo dev) y **hacer que use `cellOriginAt`** para no mentir.
- [ ] Si se mantiene, aplicar el mismo token/estilo del OS (monocromo, sin `#ff0000` hardcodeado) o
  marcarla dev-only; si no, retirarla (borrar `debug-grid-overlay.ts`, el atajo Ctrl+Shift+G en
  `workspace-icon-grid.ts` y el CSS `--depurar`/`__debug*`).
- [ ] VarSense: verificar tokens y que no queden clases huérfanas tras el cambio.

**Gate F3:** sin código de depuración visible en producción; si se mantiene, coherente y dev-only.

### Fase 4 — Verificación final

- [ ] Suite frontend completa + type-check + gate `task:check` (ID de tarea al abrir el bloque).
- [ ] Navegador real: 1440×900 y 1024×768 — arrastrar iconos, soltar en celdas libres y ocupadas
  (resolución de colisión), reflow al encoger/agrandar ventana (iconos no desaparecen ni se
  superponen), y el placeholder cae sobre la celda marcada.
- [ ] Móvil (<768): sin posicionamiento libre (el reorder por índice sigue siendo el fallback).

**Gate F4 / DoD:** grid coherente en desktop/tablet, placeholder exacto, sin rejillas rojas en
producción, suite + navegador verdes.

## Pruebas obligatorias

- Unit/DOM: `icon-grid.test.ts` (nuevos tests de `columnGapEffective` y `cellOriginAt`), tests de
  `icon-reorder` (highlight), suite completa del frontend.
- Navegador: desktop 1440/1024, tablet, móvil; arrastre con sobrante horizontal y RTL.
- Gate: `npm run task:check -- <ID>` y `--full` cuando el bloque cierre.

## Criterio de salida

- Un único helper de geometría (`cellOriginAt`) alimenta getCellAt, highlight y debug.
- El placeholder coincide con la celda real al arrastrar (verificado en navegador).
- La rejilla roja de debug no aparece en producción (o es dev-only y coherente).
- Los tests DOM fijan la geometría frente a `space-between` + RTL para que no regrese.
