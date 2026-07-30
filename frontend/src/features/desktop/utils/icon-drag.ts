/* wandori.us — Pointer Drag Manager
 * Sistema de drag unificado basado en Pointer Events.
 * Reemplaza HTML5 DnD (Finder) y el reordering por separado (desktop).
 * Usa data-drop-target + data-drop-id como registry de drop zones.
 * Usa document.elementFromPoint para encontrar targets bajo el cursor.
 * [Auditoría v2] Unificación Finder ↔ desktop.
 */

import type { NodeId } from '../../runtime/workspace/types';

/** Resultado de un drop. */
export interface DragDropResult {
  /** ID del nodo arrastrado. */
  readonly sourceId: string;
  /** ID del nodo destino (folder, desktop, etc). */
  readonly targetId: string;
  /** Contexto de origen ('desktop' | 'finder'). */
  readonly sourceContext: string;
  /** Contexto de destino. */
  readonly targetContext: string;
  /** Si el target es un reordering (mismo grid). */
  readonly reorderIndex?: number;
}

/** Callback cuando un drop ocurre (cross-context). */
export type DragDropHandler = (result: DragDropResult) => void;

/** Estado global de la sesión de drag activa. */
interface DragSession {
  readonly sourceId: string;
  readonly sourceContext: string;
  readonly ghost: HTMLElement;
  readonly gridEl: HTMLElement;
  readonly itemSelector: string;
  readonly onReorder?: (draggedId: NodeId, targetIndex: number) => void;
  highlightEl: HTMLElement | null;
  currentTarget: HTMLElement | null;
}

let activeSession: DragSession | null = null;
let globalDropHandler: DragDropHandler | null = null;

/** Registrar el handler global de drops (una sola vez, en desktop-shell). */
export function onGlobalDrop(handler: DragDropHandler): void {
  globalDropHandler = handler;
}

/** Encontrar el elemento drop target más cercano bajo las coordenadas. */
function findDropTarget(x: number, y: number, exclude: HTMLElement | null): HTMLElement | null {
  if (activeSession) activeSession.ghost.style.display = 'none';
  const el = document.elementFromPoint(x, y) as HTMLElement | null;
  if (activeSession) activeSession.ghost.style.display = '';

  if (!el) return null;

  const target = el.closest<HTMLElement>('[data-drop-target="true"]');
  if (!target || target === exclude) return null;
  return target;
}

/**
 * Habilita drag en un elemento usando Pointer Events.
 * El elemento debe tener data-node-id y data-drag-context.
 */
export function enableDrag(options: {
  el: HTMLElement;
  nodeId: string;
  context: string;
  gridEl: HTMLElement;
  /** Selector CSS para items arrastrables dentro del grid (para reordering). */
  itemSelector?: string;
  /** Callback de reordenamiento local (mismo grid). Llamado cuando el drop es en el mismo grid. */
  onReorder?: (draggedId: NodeId, targetIndex: number) => void;
}): () => void {
  const { el, nodeId, context, gridEl, itemSelector = '.desktop-icon--interactive', onReorder } = options;
  const DRAG_THRESHOLD = 6;

  let startX = 0;
  let startY = 0;

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    startX = e.clientX;
    startY = e.clientY;
    el.style.touchAction = 'none';

    document.addEventListener('pointermove', onPointerMove);
    document.addEventListener('pointerup', onPointerUp);
  }

  function onPointerMove(e: PointerEvent): void {
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;

    if (!activeSession && Math.abs(dx) + Math.abs(dy) > DRAG_THRESHOLD) {
      el.classList.add('desktop-icon--dragging');

      const ghost = el.cloneNode(true) as HTMLElement;
      ghost.className = 'desktop-icon desktop-icon--ghost';
      ghost.style.position = 'fixed';
      ghost.style.pointerEvents = 'none';
      ghost.style.zIndex = '10000';
      ghost.style.opacity = '0.7';
      document.body.appendChild(ghost);

      activeSession = {
        sourceId: nodeId,
        sourceContext: context,
        ghost,
        gridEl,
        itemSelector,
        onReorder,
        highlightEl: null,
        currentTarget: null,
      };
    }

    if (activeSession) {
      const rect = el.getBoundingClientRect();
      activeSession.ghost.style.left = `${e.clientX - rect.width / 2}px`;
      activeSession.ghost.style.top = `${e.clientY - rect.height / 2}px`;

      const target = findDropTarget(e.clientX, e.clientY, gridEl);
      updateHighlight(target, e.clientX, e.clientY);
    }
  }

  function onPointerUp(e: PointerEvent): void {
    document.removeEventListener('pointermove', onPointerMove);
    document.removeEventListener('pointerup', onPointerUp);
    el.style.touchAction = '';

    if (!activeSession) return;

    const target = findDropTarget(e.clientX, e.clientY, null);

    if (target) {
      const targetId = target.dataset.dropId ?? '';
      const targetContext = target.dataset.dropContext ?? '';
      const isSameGrid = target === gridEl;

      if (isSameGrid && onReorder) {
        /* Reordering dentro del mismo grid */
        const targetIndex = findReorderIndex(e.clientX, e.clientY);
        if (targetIndex >= 0) {
          onReorder(nodeId as NodeId, targetIndex);
        }
      } else if (globalDropHandler && targetId) {
        /* Drop cross-context: Finder→Desktop, Desktop→Finder, etc */
        globalDropHandler({
          sourceId: nodeId,
          targetId,
          sourceContext: context,
          targetContext,
        });
      }
    }

    /* Cleanup */
    el.classList.remove('desktop-icon--dragging');
    activeSession.ghost.remove();
    activeSession.highlightEl?.remove();
    activeSession.currentTarget?.classList.remove('desktop-icon--drop-hover');
    activeSession = null;
  }

  el.addEventListener('pointerdown', onPointerDown);

  return () => {
    el.removeEventListener('pointerdown', onPointerDown);
    document.removeEventListener('pointermove', onPointerMove);
    document.removeEventListener('pointerup', onPointerUp);
    el.style.touchAction = '';
  };
}

/**
 * Marcar un elemento como drop target.
 * Debe tener data-drop-id y opcionalmente data-drop-context.
 */
export function makeDropTarget(options: {
  el: HTMLElement;
  dropId: string;
  context: string;
}): void {
  options.el.setAttribute('data-drop-target', 'true');
  options.el.setAttribute('data-drop-id', options.dropId);
  options.el.setAttribute('data-drop-context', options.context);
}

/** Actualizar highlight visual del drop target. */
function updateHighlight(target: HTMLElement | null, x: number, y: number): void {
  if (!activeSession) return;

  if (activeSession.currentTarget && activeSession.currentTarget !== target) {
    activeSession.currentTarget.classList.remove('desktop-icon--drop-hover');
  }

  activeSession.currentTarget = target;

  if (target) {
    target.classList.add('desktop-icon--drop-hover');

    /* Highlight de posición para reordering (solo mismo grid) */
    const isSameGrid = target === activeSession.gridEl;
    if (isSameGrid) {
      const targetIndex = findReorderIndex(x, y);
      if (targetIndex >= 0) {
        positionHighlight(targetIndex);
        return;
      }
    }
  }

  activeSession.highlightEl?.remove();
  activeSession.highlightEl = null;
}

/** Encontrar índice de reordering en el grid del target. */
function findReorderIndex(x: number, y: number): number {
  if (!activeSession) return -1;

  const icons = activeSession.gridEl.querySelectorAll<HTMLElement>(activeSession.itemSelector);
  let closestIndex = -1;
  let closestDist = Infinity;

  icons.forEach((icon, i) => {
    const rect = icon.getBoundingClientRect();
    const cx = rect.left + rect.width / 2;
    const cy = rect.top + rect.height / 2;
    const dist = Math.hypot(x - cx, y - cy);
    if (dist < closestDist) {
      closestDist = dist;
      closestIndex = i;
    }
  });

  return closestIndex;
}

/** Posicionar highlight de reordering. */
function positionHighlight(targetIndex: number): void {
  if (!activeSession) return;
  const icons = activeSession.gridEl.querySelectorAll<HTMLElement>(activeSession.itemSelector);
  const target = icons[targetIndex];
  if (!target) return;

  if (!activeSession.highlightEl) {
    const hl = document.createElement('div');
    hl.className = 'desktop-icon-drop-target';
    activeSession.gridEl.appendChild(hl);
    activeSession.highlightEl = hl;
  }

  const rect = target.getBoundingClientRect();
  const gridRect = activeSession.gridEl.getBoundingClientRect();
  const hl = activeSession.highlightEl;
  hl.style.left = `${rect.left - gridRect.left}px`;
  hl.style.top = `${rect.top - gridRect.top}px`;
  hl.style.width = `${rect.width}px`;
  hl.style.height = `${rect.height}px`;
}
