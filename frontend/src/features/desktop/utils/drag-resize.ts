/* wandori.us — Drag & Resize
 * Utilidad de puntero para arrastrar y redimensionar ventanas.
 * Patrón estándar: pointerdown inicia, document-level pointermove/pointerup terminan.
 * Así funciona Windows/macOS: soltar fuera de la ventana sigue funcionando. */

import { updateWindowBounds, focusWindow, clampWindowBounds, toggleMaximizeWindow } from '../../runtime/window-manager';

export interface DragResizeOptions {
  /** Elemento raíz de la ventana (section.desktop-window). */
  windowEl: HTMLElement;
  /** ID de la ventana en windowStore. */
  instanceId: string;
  /** Elemento que activa el drag (header). */
  dragHandle: HTMLElement;
  /** Si la ventana permite resize. */
  resizable: boolean;
}

type ResizeEdge = 'right' | 'bottom' | 'corner' | 'left' | 'bottom-left';

interface ActiveDrag {
  startX: number;
  startY: number;
  startLeft: number;
  startTop: number;
}

interface ActiveResize {
  edge: ResizeEdge;
  startX: number;
  startY: number;
  startLeft: number;
  startTop: number;
  startW: number;
  startH: number;
}

/**
 * Activa drag y resize en una ventana.
 * Devuelve una función de cleanup para remover listeners.
 */
export function enableDragResize(opts: DragResizeOptions): () => void {
  const { windowEl, instanceId, dragHandle, resizable } = opts;

  let drag: ActiveDrag | null = null;
  let resize: ActiveResize | null = null;
  const MIN_W = 200;
  const MIN_H = 150;
  const EDGE_SIZE = 4;

  /* ─── Drag (title bar) ─── */

  /* Doble-clic en titlebar = maximizar/restaurar */
  function onTitleBarDblClick(e: MouseEvent): void {
    if ((e.target as HTMLElement).closest('button')) return;
    toggleMaximizeWindow(instanceId);
  }

  function onDragPointerDown(e: PointerEvent): void {
    if ((e.target as HTMLElement).closest('button')) return;
    e.preventDefault();
    drag = {
      startX: e.clientX,
      startY: e.clientY,
      startLeft: windowEl.offsetLeft,
      startTop: windowEl.offsetTop,
    };
    focusWindow(instanceId);
  }

  /* ─── Resize (edges) ─── */

  function onResizePointerDown(e: PointerEvent, edge: ResizeEdge): void {
    e.preventDefault();
    e.stopPropagation();
    resize = {
      edge,
      startX: e.clientX,
      startY: e.clientY,
      startLeft: windowEl.offsetLeft,
      startTop: windowEl.offsetTop,
      startW: windowEl.offsetWidth,
      startH: windowEl.offsetHeight,
    };
    focusWindow(instanceId);
  }

  /* ─── Document-level move/up (shared by drag & resize) ─── */

  function onDocumentPointerMove(e: PointerEvent): void {
    if (drag) {
      const dx = e.clientX - drag.startX;
      const dy = e.clientY - drag.startY;
      const clamped = clampWindowBounds(
        drag.startLeft + dx,
        drag.startTop + dy,
        windowEl.offsetWidth,
        windowEl.offsetHeight,
      );
      windowEl.style.left = `${clamped.x}px`;
      windowEl.style.top = `${clamped.y}px`;
      return;
    }

    if (resize) {
      const dx = e.clientX - resize.startX;
      const dy = e.clientY - resize.startY;
      const edge = resize.edge;

      let newX = resize.startLeft;
      let newY = resize.startTop;
      let newW = resize.startW;
      let newH = resize.startH;

      /* Horizontal */
      if (edge.includes('right')) {
        newW = Math.max(MIN_W, resize.startW + dx);
      }
      if (edge.includes('left')) {
        newW = Math.max(MIN_W, resize.startW - dx);
        newX = resize.startLeft + (resize.startW - newW);
      }

      /* Vertical */
      if (edge === 'bottom' || edge === 'corner' || edge === 'bottom-left') {
        newH = Math.max(MIN_H, resize.startH + dy);
      }

      /* Clamp to workspace bounds */
      const clamped = clampWindowBounds(newX, newY, newW, newH);
      windowEl.style.left = `${clamped.x}px`;
      windowEl.style.top = `${clamped.y}px`;
      windowEl.style.width = `${clamped.w}px`;
      windowEl.style.height = `${clamped.h}px`;
    }
  }

  function onDocumentPointerUp(): void {
    if (drag) {
      drag = null;
      commitBounds();
    }
    if (resize) {
      resize = null;
      commitBounds();
    }
  }

  /* ─── Cursor hint on edges ─── */

  function detectEdge(e: MouseEvent): ResizeEdge | null {
    if (!resizable) return null;
    const rect = windowEl.getBoundingClientRect();
    /* Top edge excluded: titlebar lives there (Windows/macOS convention) */
    const onRight = e.clientX > rect.right - EDGE_SIZE;
    const onBottom = e.clientY > rect.bottom - EDGE_SIZE;
    const onLeft = e.clientX < rect.left + EDGE_SIZE;

    if (onBottom && onLeft) return 'bottom-left';
    if (onBottom && onRight) return 'corner';
    if (onRight) return 'right';
    if (onBottom) return 'bottom';
    if (onLeft) return 'left';
    return null;
  }

  function cursorForEdge(edge: ResizeEdge | null): string {
    switch (edge) {
      case 'right': return 'ew-resize';
      case 'left': return 'ew-resize';
      case 'bottom': return 'ns-resize';
      case 'corner': return 'nwse-resize';
      case 'bottom-left': return 'nesw-resize';
      default: return '';
    }
  }

  function onMouseMove(e: MouseEvent): void {
    if (drag || resize) return;
    const edge = detectEdge(e);
    windowEl.style.cursor = cursorForEdge(edge);
  }

  function onMouseDown(e: MouseEvent): void {
    if (!resizable) return;
    if (dragHandle.contains(e.target as Node)) return;
    const edge = detectEdge(e);
    if (edge) {
      /* Cast MouseEvent to the shape PointerEvent needs for our handler */
      onResizePointerDown(e as unknown as PointerEvent, edge);
    }
  }

  function onMouseLeave(): void {
    if (!drag && !resize) windowEl.style.cursor = '';
  }

  /* ─── Commit bounds to store ─── */

  function commitBounds(): void {
    updateWindowBounds(instanceId, {
      x: windowEl.offsetLeft,
      y: windowEl.offsetTop,
      w: windowEl.offsetWidth,
      h: windowEl.offsetHeight,
    });
  }

  /* ─── Binding ─── */

  dragHandle.addEventListener('dblclick', onTitleBarDblClick);
  dragHandle.addEventListener('pointerdown', onDragPointerDown);
  /* Document-level: catches pointer even outside window */
  document.addEventListener('pointermove', onDocumentPointerMove);
  document.addEventListener('pointerup', onDocumentPointerUp);

  if (resizable) {
    windowEl.addEventListener('mousemove', onMouseMove);
    windowEl.addEventListener('mousedown', onMouseDown);
    windowEl.addEventListener('mouseleave', onMouseLeave);
  }

  /* Cleanup */
  return () => {
    dragHandle.removeEventListener('dblclick', onTitleBarDblClick);
    dragHandle.removeEventListener('pointerdown', onDragPointerDown);
    document.removeEventListener('pointermove', onDocumentPointerMove);
    document.removeEventListener('pointerup', onDocumentPointerUp);
    if (resizable) {
      windowEl.removeEventListener('mousemove', onMouseMove);
      windowEl.removeEventListener('mousedown', onMouseDown);
      windowEl.removeEventListener('mouseleave', onMouseLeave);
    }
  };
}
