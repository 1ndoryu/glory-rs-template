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

**Síntoma adicional (05-ago, usuario):** "los iconos interactúan extraños cuando los juntas — se
altera todo en vez de alterarse 1 solo". Al arrastrar un icono sobre otro (o arrastrar uno con una
selección múltiple residual), el movimiento no es puntual: se desplazan varios iconos, se superponen
o el grid entero se reordena de golpe.

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

6. **El drag de grupo se decide por la selección en el DROP, no por el gesto.** En
   `workspace-icon-grid.ts`, `onPlaceCell` consulta `selectionStore.get()` al soltar:
   `isGroup = source === 'desktop' && selectedIds.length > 1 && selectedIds.includes(draggedId)`.
   No captura el gesto al iniciar el pointerdown (a diferencia de `enableDrag`, que sí captura
   `groupIds` pero el escritorio no lo usa). Consecuencia: si queda una selección múltiple residual
   (banda de selección, Ctrl+clic), arrastrar UN icono mueve TODOS los seleccionados — "se altera
   todo en vez de alterarse 1 solo". Además la decisión puede cambiar entre el inicio y el drop si
   la selección cambia a mitad del gesto.

7. **El grupo no resuelve colisiones ni clampa al grid.** `buildGroupPlacementMoves` aplica el
   mismo delta a cada miembro sin resolver colisiones contra los no seleccionados (se superponen) y
   sin clampear a `metrics.columns/rows` (los miembros pueden quedar fuera de bounds y crear tracks
   implícitos — el caso documentado en 058A-1). `planPlacement` solo resuelve la colisión del
   arrastrado. Cuando el usuario encoge la ventana, `reflowPositions` reempaqueta TODOS los nodos
   posicionados en orden fila/col para arreglar overlaps/fuera-de-bounds: el "arreglo" altera todo
   el escritorio de golpe en vez de solo los iconos implicados.

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

### Fase 3 — Interacción de grupo predecible (se altera 1 solo, o el grupo completo conscientemente)

- [ ] Capturar el grupo al INICIO del gesto (pointerdown), no al soltar: en `onPlaceCell` leer la
  selección capturada al iniciar el drag (mismo patrón que `groupIds` de `enableDrag`, pasándole
  `getGroupIds` desde `workspace-icon-grid.ts`) y usarla para decidir el drag de grupo.
- [ ] Regla Windows: arrastrar un icono **seleccionado** mueve el grupo; arrastrar un icono **no
  seleccionado** mueve solo ese icono (y la selección se reemplaza). Verificar que un clic simple
  sobre un seleccionado sin arrastre conserva la selección (ya documentado en 058A-4).
- [ ] Resolver colisiones del grupo: al soltar, los miembros que caigan en celdas ocupadas por no
  seleccionados desplazan al ocupante (reusar `planPlacement` por miembro o resolver el grupo
  como bloque); los miembros fuera de bounds se clampean a la celda más cercana válida (nunca
  crear tracks implícitos).
- [ ] Asegurar que `reflowPositions` (resize) no reempaquete todo el grid salvo que haya un
  overlap/fuera-de-bounds real: con la geometría unificada (F1) y el grupo resuelto (F3), el
  reflow solo debe tocar los nodos que realmente cambian.
- [ ] Tests: unidad para `buildGroupPlacementMoves` (delta + clamp + colisión) y un test de
  `onPlaceCell` con selección residual: arrastrar un icono no seleccionado mueve solo ese.

**Gate F3:** con selección múltiple residual, arrastrar un icono no seleccionado altera solo ese;
arrastrar uno seleccionado mueve el grupo sin superposiciones ni fuera-de-bounds; reflow no
reordena todo el grid.

### Fase 4 — Rejilla de debug coherente o retirada

- [ ] Decisión: si la rejilla roja es herramienta interna de desarrollo, dejar de exponerla en
  build de producción (solo dev) y **hacer que use `cellOriginAt`** para no mentir.
- [ ] Si se mantiene, aplicar el mismo token/estilo del OS (monocromo, sin `#ff0000` hardcodeado) o
  marcarla dev-only; si no, retirarla (borrar `debug-grid-overlay.ts`, el atajo Ctrl+Shift+G en
  `workspace-icon-grid.ts` y el CSS `--depurar`/`__debug*`).
- [ ] VarSense: verificar tokens y que no queden clases huérfanas tras el cambio.

**Gate F4:** sin código de depuración visible en producción; si se mantiene, coherente y dev-only.

### Fase 5 — Verificación final

- [ ] Suite frontend completa + type-check + gate `task:check` (ID de tarea al abrir el bloque).
- [ ] Navegador real: 1440×900 y 1024×768 — arrastrar iconos, soltar en celdas libres y ocupadas
  (resolución de colisión), reflow al encoger/agrandar ventana (iconos no desaparecen ni se
  superponen), y el placeholder cae sobre la celda marcada.
- [ ] Móvil (<768): sin posicionamiento libre (el reorder por índice sigue siendo el fallback).

**Gate F5 / DoD:** grid coherente en desktop/tablet, placeholder exacto, drag de grupo predecible
(sin alterar iconos no implicados), sin rejillas rojas en producción, suite + navegador verdes.

## Pruebas obligatorias

- Unit/DOM: `icon-grid.test.ts` (nuevos tests de `columnGapEffective` y `cellOriginAt`), tests de
  `icon-reorder` (highlight), tests de `icon-group-drag` (delta + clamp + colisión) y de
  `onPlaceCell` con selección residual, suite completa del frontend.
- Navegador: desktop 1440/1024, tablet, móvil; arrastre con sobrante horizontal y RTL; grupo de 2-3
  iconos seleccionados (arrastrar seleccionado vs. no seleccionado); resize con grupo fuera de
  bounds.
- Gate: `npm run task:check -- <ID>` y `--full` cuando el bloque cierre.

## Criterio de salida

- Un único helper de geometría (`cellOriginAt`) alimenta getCellAt, highlight y debug.
- El placeholder coincide con la celda real al arrastrar (verificado en navegador).
- El drag de grupo se decide por el gesto (pointerdown), no por la selección del drop; arrastrar un
  icono no seleccionado altera solo ese; el grupo se mueve sin superposiciones ni fuera-de-bounds.
- El reflow por resize no reordena todo el grid salvo overlap/fuera-de-bounds real.
- La rejilla roja de debug no aparece en producción (o es dev-only y coherente).
- Los tests DOM fijan la geometría frente a `space-between` + RTL para que no regrese.
