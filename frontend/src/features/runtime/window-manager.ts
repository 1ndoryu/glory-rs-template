/* wandori.us — Window Manager
 * Máquina de estados reactiva para ventanas del escritorio.
 * Usa el store pub/sub existente. Cada ventana tiene bounds, estado y AbortController.
 * El taskbar y el shell se suscriben a windowStore para reaccionar a cambios. */

import { createStore, type Store } from '../../store';
import type { MountedView } from '../../core/lifecycle';
import type { AppDefinition } from './app-registry';

export type WindowState = 'open' | 'minimized' | 'maximized';

export interface WindowBounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface WindowEntry {
  /** ID único de esta instancia de ventana. */
  readonly instanceId: string;
  /** ID de la app que vive dentro. */
  readonly appId: string;
  /** Título mostrado en barra y taskbar. */
  title: string;
  /** Estado visual de la ventana. */
  state: WindowState;
  /** Geometría en px (relativa al workspace). */
  bounds: WindowBounds;
  /** z-index para orden de apilamiento. */
  zIndex: number;
  /** Si esta ventana tiene foco activo. */
  focused: boolean;
  /** Contenido que la app devolvió. */
  readonly content: HTMLElement;
  /** AbortController de esta instancia — se aborta al cerrar. */
  readonly controller: AbortController;
  /** Definición de la app (referencia). */
  readonly app: AppDefinition;
}

/* === Store reactivo === */
export const windowStore: Store<WindowEntry[]> = createStore([]);

let nextZIndex = 10;
let nextWindowId = 1;

function generateWindowId(): string {
  return `win-${nextWindowId++}`;
}

/** Obtener todas las ventanas. */
export function getWindows(): WindowEntry[] {
  return windowStore.get();
}

/** Obtener la ventana con foco. */
export function getFocusedWindow(): WindowEntry | undefined {
  return windowStore.get().find(w => w.focused);
}

/** Abrir una nueva ventana para una app. */
export function openWindow(
  app: AppDefinition,
  view: MountedView,
  controller: AbortController,
  initialBounds?: Partial<WindowBounds>,
): string {
  const instanceId = generateWindowId();
  const existing = windowStore.get();
  const defaults: WindowBounds = {
    x: 40 + (existing.length % 8) * 30,
    y: 40 + (existing.length % 8) * 30,
    w: 640,
    h: 480,
  };
  const bounds: WindowBounds = { ...defaults, ...initialBounds };

  /* Desenfocar todas las existentes */
  const updated = existing.map(w => ({ ...w, focused: false }));

  const entry: WindowEntry = {
    instanceId,
    appId: app.id,
    title: app.title,
    state: 'open',
    bounds,
    zIndex: nextZIndex++,
    focused: true,
    content: view.element,
    controller,
    app,
  };

  windowStore.set([...updated, entry]);
  return instanceId;
}

/** Cerrar una ventana (destruye contenido y aborta signal). */
export function closeWindow(instanceId: string): void {
  const windows = windowStore.get();
  const target = windows.find(w => w.instanceId === instanceId);
  if (!target) return;

  /* Abortar el signal de la app */
  target.controller.abort();
  /* Ejecutar cleanup de la app si existe */
  target.content.dispatchEvent(new CustomEvent('view:destroy'));

  const remaining = windows.filter(w => w.instanceId !== instanceId);

  /* Si era la ventana enfocada, enfocar la siguiente por z-index */
  if (target.focused && remaining.length > 0) {
    const topWindow = remaining.reduce((a, b) => (a.zIndex > b.zIndex ? a : b));
    topWindow.focused = true;
  }

  windowStore.set(remaining);
}

/** Enfocar una ventana (traer al frente). */
export function focusWindow(instanceId: string): void {
  const windows = windowStore.get();
  const updated = windows.map(w => {
    if (w.instanceId === instanceId) {
      return { ...w, focused: true, zIndex: nextZIndex++ };
    }
    return { ...w, focused: false };
  });
  windowStore.set(updated);
}

/** Minimizar una ventana. */
export function minimizeWindow(instanceId: string): void {
  const windows = windowStore.get();
  const target = windows.find(w => w.instanceId === instanceId);
  if (!target || target.state === 'minimized') return;

  const updated = windows.map(w => {
    if (w.instanceId === instanceId) {
      return { ...w, state: 'minimized' as const, focused: false };
    }
    return w;
  });

  /* Enfocar la siguiente ventana visible */
  const visible = updated.filter(w => w.state === 'open');
  if (visible.length > 0) {
    const top = visible.reduce((a, b) => (a.zIndex > b.zIndex ? a : b));
    top.focused = true;
  }

  windowStore.set(updated);
}

/** Restaurar una ventana minimizada. */
export function restoreWindow(instanceId: string): void {
  const windows = windowStore.get();
  const updated = windows.map(w => {
    if (w.instanceId === instanceId) {
      return { ...w, state: 'open' as const, focused: true, zIndex: nextZIndex++ };
    }
    return { ...w, focused: false };
  });
  windowStore.set(updated);
}

/** Actualizar bounds de una ventana (drag/resize). */
export function updateWindowBounds(instanceId: string, bounds: Partial<WindowBounds>): void {
  const windows = windowStore.get();
  const updated = windows.map(w => {
    if (w.instanceId === instanceId) {
      return { ...w, bounds: { ...w.bounds, ...bounds } };
    }
    return w;
  });
  windowStore.set(updated);
}

/** Verificar si una app singleton ya está abierta. */
export function findOpenWindow(appId: string): WindowEntry | undefined {
  return windowStore.get().find(w => w.appId === appId);
}

/** Cerrar todas las ventanas (para cleanup). */
export function closeAllWindows(): void {
  const windows = windowStore.get();
  for (const w of windows) {
    w.controller.abort();
  }
  windowStore.set([]);
}
