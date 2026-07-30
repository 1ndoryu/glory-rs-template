/* wandori.us — Icon Drag
 * Utilidad de puntero para arrastrar iconos del desktop y reordenarlos.
 * Usa pointer events para soporte táctil. Persiste posiciones en workspace overlay.
 * [Plan 297A-11 §9.4] */

import type { NodeId } from '../../runtime/workspace/types';

export interface IconDragOptions {
  /** Elemento del icono arrastrable. */
  iconEl: HTMLElement;
  /** ID del nodo del workspace. */
  nodeId: NodeId;
  /** Contenedor del grid (para calcular celdas). */
  gridEl: HTMLElement;
  /** Callback para reordenar los IDs y persistir. */
  onReorder: (draggedId: NodeId, targetIndex: number) => void;
}

/**
 * Habilita drag para reordenar un icono en el grid.
 * Devuelve una función de cleanup.
 */
export function enableIconDrag(options: IconDragOptions): () => void {
  const { iconEl, nodeId, gridEl, onReorder } = options;
  const DRAG_THRESHOLD = 6;

  let dragging = false;
  let startX = 0;
  let startY = 0;
  let ghostEl: HTMLElement | null = null;
  let highlightEl: HTMLElement | null = null;

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    startX = e.clientX;
    startY = e.clientY;
    dragging = false;
    iconEl.style.touchAction = 'none';

    document.addEventListener('pointermove', onPointerMove);
    document.addEventListener('pointerup', onPointerUp);
  }

  function onPointerMove(e: PointerEvent): void {
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;

    if (!dragging && Math.abs(dx) + Math.abs(dy) > DRAG_THRESHOLD) {
      dragging = true;
      iconEl.classList.add('desktop-icon--dragging');
      iconEl.setPointerCapture(e.pointerId);

      /* Crear ghost (imagen semi-transparente del icono) */
      ghostEl = iconEl.cloneNode(true) as HTMLElement;
      ghostEl.className = 'desktop-icon desktop-icon--ghost';
      ghostEl.style.position = 'fixed';
      ghostEl.style.pointerEvents = 'none';
      ghostEl.style.zIndex = '10000';
      ghostEl.style.opacity = '0.7';
      document.body.appendChild(ghostEl);

      /* Highlight del target */
      highlightEl = document.createElement('div');
      highlightEl.className = 'desktop-icon-drop-target';
      gridEl.appendChild(highlightEl);
    }

    if (dragging && ghostEl) {
      const iconRect = iconEl.getBoundingClientRect();
      ghostEl.style.left = `${e.clientX - iconRect.width / 2}px`;
      ghostEl.style.top = `${e.clientY - iconRect.height / 2}px`;

      /* Encontrar el icono más cercano al cursor */
      const targetIndex = findTargetIndex(e.clientX, e.clientY);
      if (highlightEl && targetIndex >= 0) {
        positionHighlight(targetIndex);
      }
    }
  }

  function onPointerUp(e: PointerEvent): void {
    document.removeEventListener('pointermove', onPointerMove);
    document.removeEventListener('pointerup', onPointerUp);

    if (dragging) {
      const targetIndex = findTargetIndex(e.clientX, e.clientY);
      if (targetIndex >= 0) {
        onReorder(nodeId, targetIndex);
      }
    }

    /* Cleanup */
    iconEl.classList.remove('desktop-icon--dragging');
    iconEl.style.touchAction = '';
    ghostEl?.remove();
    ghostEl = null;
    highlightEl?.remove();
    highlightEl = null;
    dragging = false;
  }

  function findTargetIndex(x: number, y: number): number {
    const icons = gridEl.querySelectorAll<HTMLElement>('.desktop-icon--interactive');
    let closestIndex = -1;
    let closestDist = Infinity;

    icons.forEach((icon, i) => {
      if (icon === iconEl) return; /* Skip self */
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

  function positionHighlight(targetIndex: number): void {
    if (!highlightEl) return;
    const icons = gridEl.querySelectorAll<HTMLElement>('.desktop-icon--interactive');
    const target = icons[targetIndex];
    if (!target) return;

    const rect = target.getBoundingClientRect();
    const gridRect = gridEl.getBoundingClientRect();
    highlightEl.style.left = `${rect.left - gridRect.left}px`;
    highlightEl.style.top = `${rect.top - gridRect.top}px`;
    highlightEl.style.width = `${rect.width}px`;
    highlightEl.style.height = `${rect.height}px`;
  }

  iconEl.addEventListener('pointerdown', onPointerDown);

  return () => {
    iconEl.removeEventListener('pointerdown', onPointerDown);
    document.removeEventListener('pointermove', onPointerMove);
    document.removeEventListener('pointerup', onPointerUp);
    iconEl.style.touchAction = '';
    ghostEl?.remove();
    highlightEl?.remove();
  };
}
