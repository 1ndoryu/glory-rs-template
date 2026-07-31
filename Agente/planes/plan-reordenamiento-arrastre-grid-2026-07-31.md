# Plan: Reordenamiento de iconos por arrastre con grid (móvil + escritorio)

> **Tarea propuesta:** 297A-22
> **Fecha:** 2026-07-31
> **Estado:** ⏸ PENDIENTE DE REVISIÓN — no implementar hasta aprobación explícita del usuario
> **Siguiente paso:** revisar las decisiones abiertas (sección 8) y aprobar la opción de modelo (sección 5)

## 1. Objetivo y límites

**Problema:** los iconos no deben reordenarse mediante "Mover arriba / Mover abajo"
(swap de índices). El usuario quiere **arrastre + grid** en ambas superficies:
escritorio y móvil.

- El escritorio ya tiene drag + snap-grid (`position {col,row}`) desde 297A-20.
- El móvil no tiene drag: solo long press → menú → `workspace:move-up/down`
  (swap de `mobileOrder`, un paso a la vez).

**Objetivo:** que el launcher móvil reordene por arrastre sobre un grid de celdas
(como el escritorio), y eliminar/quitar del flujo el reorden por swap de índices.

**Límites (no se toca en este plan):**
- Backend, API, migraciones de BD.
- Identidad visual (tokens/chrome ya aprobados).
- Comandos de ventanas, deep links, analytics.
- Otras superficies (taskbar, menús de barra superior).

## 2. Hallazgos de investigación (estado actual, 2026-07-31)

### Modelo de datos (dos sistemas independientes)
- `position?: GridPosition {col,row}` — snap-grid desktop/tablet (≥769px).
  `types.ts:47-51`.
- `mobileOrder?: number` — índice plano por padre; usado por launcher móvil,
  grid desktop ≤768px y `getChildren`. `types.ts:60`.
- `fieldOverrides` permite override de `position | label | parentId | mobileOrder`.

### Launcher móvil (`mobile-shell.ts`)
- Ordena solo por `mobileOrder` (línea ~216) y renderiza en grid CSS fijo de 3
  columnas (`mobile-prototype.css:79-84`; 2 columnas ≤480px).
- **No usa `position`**; sin huecos posibles, flujo por orden del DOM.
- Único gesto: `bindLongPress` (500ms, umbral 10px) en `mobile-gestures.ts`
  → abre el mismo menú contextual `context: 'icon'` con presentación móvil.

### Comandos de reorden
- `workspace:move-up` (order 44) / `workspace:move-down` (order 45),
  `contexts: ['icon']`, en `workspace-reorder-commands.ts`.
- `reorderTarget(ctx, direction)`: swap de índices adyacentes → `reorderWorkspaceNodes`
  → escribe `mobileOrder` en `fieldOverrides`. **No toca `position`**.
- En el escritorio (≥769px) el icono tiene posición libre: el swap de `mobileOrder`
  no produce ningún movimiento visible → comandos confusos (diagnóstico confirmado).

### Escritorio (≥769px)
- Drag con Pointer Events en `icon-drag.ts`: si `onPlaceCell` y width ≥769 → modo
  placement por celda (`getCellAt` + `planPlacement` con colisiones) → `moveNodesPosition`
  escribe `position`. Si no → fallback `findReorderIndex`/`onReorder` (índice `mobileOrder`).
- Nodos sin `position` rellenan huecos con `grid-auto-flow: dense` según `mobileOrder`.
- Geometría pura y testeada en `utils/icon-grid.ts` (+ 37 tests de 297A-20).

### Tests
- Cubren geometría desktop (icon-grid, placement, reflow) y long press móvil.
- **No hay tests de reorder móvil ni del drag `icon-drag.ts`/`icon-reorder.ts`.**

## 3. Requisitos del cambio (derivados de la petición)

1. El launcher móvil reordena por **arrastre** (gesto táctil), no por menú swap.
2. El arrastre móvil suelta sobre **celdas de un grid** (como el escritorio).
3. "Mover arriba / abajo" deja de ser el mecanismo de reorden (se elimina o se
   reemplaza por una alternativa accesible que opere sobre celdas).
4. La organización móvil persiste en el overlay (`fieldOverrides`) y sobrevive
   reload/sync, sin contaminar el layout desktop.
5. Se mantiene el requisito de accesibilidad del plan móvil 297A-12 §9:
   "No depender solo de long press; mover arriba/abajo disponible como comando
   accesible" → el reorden por drag **necesita una alternativa no gestual**.

## 4. Tensiones y decisiones de diseño a resolver en revisión

### 4.1 ¿Grid móvil con huecos o apretado?
- **Escritorio**: posición libre (permite huecos).
- **Pantalla de inicio real** (iOS/Android): grid apretado, sin huecos; al mover
  un icono los demás se desplazan.
- **Recomendación:** móvil = grid **apretado** (3 columnas, sin huecos) reusando
  `planPlacement` (que ya desplaza al ocupante). Escritorio conserva posición libre.

### 4.2 ¿Un solo campo de posición o dos?
- `position` (desktop) depende del ancho auto-fill; el launcher móvil es de
  columnas fijas (3/2). **Una misma col/row no es válida para ambos.**
- **Recomendación:** añadir `mobilePosition {col,row}` (launcher) y dejar
  `position` (desktop/tablet). `mobileOrder` se deprecia a "orden de fallback"
  para nodos sin `mobilePosition` y para compat con datos existentes.
- Alternativa a evaluar: derivar `mobileOrder` del orden de `mobilePosition`.

### 4.3 ¿Qué pasa con `getChildren` y el orden de render?
- `getChildren` ordena por `mobileOrder` hoy. Con `mobilePosition`, el orden de
  render debe respetar primero `mobilePosition` y luego `mobileOrder` (fallback).
- En escritorio, los nodos sin `position` siguen fluyendo por ese orden derivado.

### 4.4 ¿Cómo convive el long press con el drag táctil?
- Hoy: long press 500ms → menú; mover >10px cancela.
- **Recomendación (patrón estándar):** mantener pulsado activa "modo edición";
  si luego hay movimiento >umbral → **drag**; si se suelta sin movimiento → **menú**.
  Es decir, el menú se abre al soltar sin arrastrar, no durante el long press.
  Umbral de drag y `touch-action` gestionados para no competir con el scroll.

### 4.5 Alternativa accesible (requisito 297A-12 §9)
- **Recomendación:** reemplazar `workspace:move-up/down` por comandos que operan
  sobre la celda (`Mover a celda anterior/siguiente` en el grid móvil, y en el
  escritorio mover la `position` una celda). Se elimina del menú del escritorio
  el swap sin efecto visible.
- Pregunta abierta: ¿se mantienen como comandos de teclado/lista, o se reemplazan
  por el drag + alternativa de lista propia?

## 5. Opciones de modelo (a elegir en revisión)

### Opción 1 — Paridad completa (recomendada)
Launcher móvil con `mobilePosition {col,row}` + drag por celdas + grid apretado.
`mobileOrder` deprecado a fallback. Máxima coherencia con el escritorio; más
cambio (types, merge, render, gesto, migración de datos).

### Opción 2 — Drag por índice (mínima)
Mantener `mobileOrder`, pero añadir drag táctil que reordena el índice al soltar
sobre una celda (reusar `findReorderIndex`). No hay posición libre móvil; el
"grid" es solo visual (soltar en celdas). Menos cambio, pero no da celdas
persistentes en móvil.

**Recomendación:** Opción 1, porque la petición pide explícitamente "arrastre y
con grid" en móvil, con paridad de comportamiento.

## 6. Fases propuestas (solo al aprobar el plan)

- [ ] **Fase 0 — Revisión y aprobación.** Resolver secciones 4 y 5 con el usuario;
      fijar opción de modelo, huecos sí/no, destino de move-up/down.
- [ ] **Fase 1 — Modelo de datos.** Añadir `mobilePosition` a tipos/fieldOverrides;
      deprecar `mobileOrder` a fallback; actualizar `merge.ts`, `overlay-mutations.ts`,
      `getChildren` y `default-release.ts` (migrar/derivar datos existentes).
- [ ] **Fase 2 — Launcher snap-grid.** Convertir `.movilLauncher__grid` a grid con
      celdas; reutilizar geometría de `icon-grid.ts` parametrizada por columnas fijas;
      render con `mobilePosition` (fallback `mobileOrder`).
- [ ] **Fase 3 — Drag táctil en launcher.** Extender `mobile-gestures.ts`:
      long press → modo edición; movimiento >umbral → drag; soltar en celda →
      `planPlacement` (apretado o libre según decisión) → persistir `mobilePosition`.
- [ ] **Fase 4 — Escritorio.** Quitar del menú contextual el swap sin efecto visible;
      el drag desktop conserva `position`. Evaluar si el fallback `onReorder` del
      grid ≤768px sigue siendo necesario.
- [ ] **Fase 5 — Alternativa accesible.** Reemplazar `workspace:move-up/down` por
      comandos sobre celdas (o lista alternativa), cumpliendo 297A-12 §9.
- [ ] **Fase 6 — Migración y limpieza.** Datos sin `mobilePosition` → derivar de
      `mobileOrder`; eliminar código muerto del swap si procede.
- [ ] **Fase 7 — Tests y validación visual.** Unitarios (geometría móvil, merge,
      gesto) + E2E visual 320/360/390, drag táctil, huecos, accesibilidad, teclado,
      reload/sync y móvil↔tablet.

## 7. Gate y criterio de salida

**Gate por fase:** cada fase pasa `npm run task:check -- 297A-22` (o el ID
asignado) + tests de la fase antes de avanzar.

**Criterio final (Definition of Done):**
- [ ] El launcher móvil reordena por arrastre sobre celdas y persiste en overlay
      (sobrevive reload/sync sin contaminar el layout desktop).
- [ ] "Mover arriba/abajo" no es el mecanismo de reorden (eliminado o reemplazado
      por alternativa accesible que opera sobre celdas).
- [ ] Desktop conserva posición libre (297A-20) sin regresión.
- [ ] Requisito 297A-12 §9 cumplido: existe alternativa no gestual.
- [ ] Validado en 320/360/390/768+ con drag táctil, foco, teclado y zoom 200%.
- [ ] Manual de arquitectura/identidad/roadmap/Sentinel actualizados.

## 8. Preguntas abiertas para la revisión (respuestas del usuario)

1. ¿Móvil con grid apretado (sin huecos, como pantalla de inicio) o con huecos
   (como el escritorio)? → Recomendado: apretado.
2. ¿Opción 1 (paridad con `mobilePosition`) u Opción 2 (drag por índice)?
   → Recomendado: Opción 1.
3. ¿"Mover arriba/abajo": se eliminan del menú o se convierten en comandos
   accesibles sobre celdas? → Recomendado: convertir en accesibles sobre celdas.
4. ¿El menú contextual del launcher se abre al soltar sin arrastrar (patrón
   estándar) o se mantiene el long press actual + drag tras él?
5. ¿`mobileOrder` se elimina del todo (tras migrar datos) o queda como orden de
   fallback permanente?

## 9. Enlaces

- Roadmap: `roadmap.md` (297A-22 pendiente de revisión).
- Plan móvil base: `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`
  (requisito accesibilidad §9, invariantes §2).
- Iconos libres escritorio: `Agente/planes/completados/plan-iconos-libres-desktop-2026-07-31.md`.
- Manual arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`.
