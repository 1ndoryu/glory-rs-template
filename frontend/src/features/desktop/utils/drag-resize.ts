/* wandori.us — Drag & Resize
 * Utilidad de puntero para arrastrar y redimensionar ventanas.
 * Usa pointer events (no mouse) para soporte táctil.
 * Actualiza el DOM directamente durante el arrastre y commitea al windowStore al soltar. */

import { updateWindowBounds, focusWindow, clampWindowBounds } from '../../runtime/window-manager';

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

/**
 * Activa drag y resize en una ventana.
 * Devuelve una función de cleanup para remover listeners.
 */
export function enableDragResize(opts: DragResizeOptions): () => void {
  const { windowEl, instanceId, dragHandle, resizable } = opts;

  /* Estado interno del arrastre */
  let isDragging = false;
  let isResizing = false;
  let startX = 0;
  let startY = 0;
  let startLeft = 0;
  let startTop = 0;
  let startW = 0;
  let startH = 0;
  let resizeEdge: 'right' | 'bottom' | 'corner' | null = null;

  /* === DRAG por la barra de título === */
  function onDragPointerDown(e: PointerEvent): void {
    if ((e.target as HTMLElement).closest('button')) return; /* No arrastrar por botones */
    e.preventDefault();
    isDragging = true;
    startX = e.clientX;
    startY = e.clientY;
    startLeft = windowEl.offsetLeft;
    startTop = windowEl.offsetTop;
    windowEl.setPointerCapture(e.pointerId);
    focusWindow(instanceId);
  }

  function onDragPointerMove(e: PointerEvent): void {
    if (!isDragging) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    const clamped = clampWindowBounds(startLeft + dx, startTop + dy, windowEl.offsetWidth, windowEl.offsetHeight);
    windowEl.style.left = `${clamped.x}px`;
    windowEl.style.top = `${clamped.y}px`;
  }

  function onDragPointerUp(e: PointerEvent): void {
    if (!isDragging) return;
    isDragging = false;
    windowEl.releasePointerCapture(e.pointerId);
    commitBounds();
  }

  /* === RESIZE por bordes === */
  function onResizePointerDown(e: PointerEvent, edge: 'right' | 'bottom' | 'corner'): void {
    e.preventDefault();
    e.stopPropagation();
    isResizing = true;
    resizeEdge = edge;
    startX = e.clientX;
    startY = e.clientY;
    startLeft = windowEl.offsetLeft;
    startTop = windowEl.offsetTop;
    startW = windowEl.offsetWidth;
    startH = windowEl.offsetHeight;
    windowEl.setPointerCapture(e.pointerId);
    focusWindow(instanceId);
  }

  function onResizePointerMove(e: PointerEvent): void {
    if (!isResizing || !resizeEdge) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    const minW = 200;
    const minH = 150;
    const clamped = clampWindowBounds(startLeft, startTop, startW + dx, startH + dy);

    if (resizeEdge === 'right' || resizeEdge === 'corner') {
      windowEl.style.width = `${Math.max(minW, clamped.w)}px`;
    }
    if (resizeEdge === 'bottom' || resizeEdge === 'corner') {
      windowEl.style.height = `${Math.max(minH, clamped.h)}px`;
    }
  }

  function onResizePointerUp(e: PointerEvent): void {
    if (!isResizing) return;
    isResizing = false;
    resizeEdge = null;
    windowEl.releasePointerCapture(e.pointerId);
    commitBounds();
  }

  /** Commitea los bounds actuales al windowStore. */
  function commitBounds(): void {
    updateWindowBounds(instanceId, {
      x: windowEl.offsetLeft,
      y: windowEl.offsetTop,
      w: windowEl.offsetWidth,
      h: windowEl.offsetHeight,
    });
  }

  /* === Cursor en bordes (resize hint) === */
  function onMouseMove(e: MouseEvent): void {
    if (isDragging || isResizing) return;
    if (!resizable) return;

    const rect = windowEl.getBoundingClientRect();
    const edgeSize = 6;
    const onRight = e.clientX > rect.right - edgeSize;
    const onBottom = e.clientY > rect.bottom - edgeSize;

    if (onRight && onBottom) {
      windowEl.style.cursor = 'nwse-resize';
    } else if (onRight) {
      windowEl.style.cursor = 'ew-resize';
    } else if (onBottom) {
      windowEl.style.cursor = 'ns-resize';
    } else {
      windowEl.style.cursor = '';
    }
  }

  function onMouseDown(e: MouseEvent): void {
    if (!resizable) return;
    /* No activar resize si el click es en la barra de título (drag handle) */
    if (dragHandle.contains(e.target as Node)) return;
    const rect = windowEl.getBoundingClientRect();
    const edgeSize = 6;
    const onRight = e.clientX > rect.right - edgeSize;
    const onBottom = e.clientY > rect.bottom - edgeSize;

    if (onRight || onBottom) {
      const edge = onRight && onBottom ? 'corner' : onRight ? 'right' : 'bottom';
      onResizePointerDown(e as unknown as PointerEvent, edge);
    }
  }

  function onPointerMove(e: PointerEvent): void {
    if (isDragging) onDragPointerMove(e);
    if (isResizing) onResizePointerMove(e);
  }

  function onPointerUp(e: PointerEvent): void {
    if (isDragging) onDragPointerUp(e);
    if (isResizing) onResizePointerUp(e);
  }

  function onMouseLeave(): void {
    if (!isDragging && !isResizing) windowEl.style.cursor = '';
  }

  /* === Binding === */
  dragHandle.addEventListener('pointerdown', onDragPointerDown);
  windowEl.addEventListener('pointermove', onPointerMove);
  windowEl.addEventListener('pointerup', onPointerUp);

  if (resizable) {
    windowEl.addEventListener('mousemove', onMouseMove);
    windowEl.addEventListener('mousedown', onMouseDown);
    windowEl.addEventListener('mouseleave', onMouseLeave);
  }

  /* Cleanup — remueve todos los listeners */
  return () => {
    dragHandle.removeEventListener('pointerdown', onDragPointerDown);
    windowEl.removeEventListener('pointermove', onPointerMove);
    windowEl.removeEventListener('pointerup', onPointerUp);
    if (resizable) {
      windowEl.removeEventListener('mousemove', onMouseMove);
      windowEl.removeEventListener('mousedown', onMouseDown);
      windowEl.removeEventListener('mouseleave', onMouseLeave);
    }
  };
}
