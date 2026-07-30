/* wandori.us — Window Manager
 * Funciones de mutación para ventanas del escritorio.
 * Los tipos, store y getters viven en window-store.ts.
 * [Auditoría v3 §2.2] Split para mantener bajo límite de 300 líneas. */

import type { MountedView } from '../../core/lifecycle';
import type { AppDefinition, AppToolbarGroup } from './app-registry';
import type { IconNode } from 'lucide';
import {
  windowStore,
  workspaceW, workspaceH,
  clampWindowBounds, generateWindowId,
  type WindowEntry, type WindowBounds,
} from './window-store';

/* Re-export todo desde window-store para backward compatibility.
 * Los consumidores existentes importan de 'window-manager' y seguirán funcionando. */
export { windowStore, setWorkspaceBounds, clampWindowBounds, getWindows, getFocusedWindow, findOpenWindow } from './window-store';
export type { WindowState, WindowBounds, WindowEntry } from './window-store';

/* z-index counter — mutable module state shared via window-store */
let nextZIndex = 10;

/** Abrir una nueva ventana para una app. */
export function openWindow(
  app: AppDefinition,
  view: MountedView,
  controller: AbortController,
  initialBounds?: Partial<WindowBounds>,
  params?: Record<string, string>,
  titleOverride?: string,
): string {
  const instanceId = generateWindowId();
  const existing = windowStore.get();
  const defaults: WindowBounds = {
    x: 40 + (existing.length % 8) * 30,
    y: 40 + (existing.length % 8) * 30,
    w: 640,
    h: 480,
  };
  const raw: WindowBounds = { ...defaults, ...initialBounds };
  const bounds = clampWindowBounds(raw.x, raw.y, raw.w, raw.h);

  const updated = existing.map(w => ({ ...w, focused: false }));

  const entry: WindowEntry = {
    instanceId,
    appId: app.id,
    title: titleOverride ?? app.title,
    state: 'open',
    bounds,
    zIndex: nextZIndex++,
    focused: true,
    content: view.element,
    controller,
    app,
    layout: app.layout,
    toolbar: app.toolbar,
    params,
    _paramKey: params ? Object.values(params).join(':') : undefined,
    onDestroy: view.destroy,
  };

  windowStore.set([...updated, entry]);
  return instanceId;
}

/** Cerrar una ventana (destruye contenido y aborta signal). */
export function closeWindow(instanceId: string): void {
  const windows = windowStore.get();
  const target = windows.find(w => w.instanceId === instanceId);
  if (!target) return;

  target.onDestroy?.();
  target.controller?.abort();

  const remaining = windows.filter(w => w.instanceId !== instanceId);

  if (target.focused && remaining.length > 0) {
    const topWindow = remaining.reduce((a, b) => (a.zIndex > b.zIndex ? a : b));
    topWindow.focused = true;
  }

  windowStore.set(remaining);
}

/** Registrar una shell window (perfil, etc.) directamente en windowStore. */
export function registerShellWindow(options: {
  instanceId: string;
  title: string;
  icon: IconNode;
  content: HTMLElement;
  initialBounds?: Partial<WindowBounds>;
  focused?: boolean;
  cssClass?: string;
  layout?: 'padded' | 'full-bleed';
  toolbar?: AppToolbarGroup[];
}): string {
  const existing = windowStore.get();
  if (existing.find(w => w.instanceId === options.instanceId)) return options.instanceId;

  const defaults: WindowBounds = { x: 40, y: 40, w: 470, h: 360 };
  const raw: WindowBounds = { ...defaults, ...options.initialBounds };
  const bounds = clampWindowBounds(raw.x, raw.y, raw.w, raw.h);

  const updated = options.focused !== false
    ? existing.map(w => ({ ...w, focused: false }))
    : existing;

  const entry: WindowEntry = {
    instanceId: options.instanceId,
    appId: options.instanceId,
    title: options.title,
    state: 'open',
    bounds,
    zIndex: nextZIndex++,
    focused: options.focused !== false,
    content: options.content,
    icon: options.icon,
    cssClass: options.cssClass,
    layout: options.layout,
    toolbar: options.toolbar,
  };

  windowStore.set([...updated, entry]);
  return options.instanceId;
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

/** Maximizar/restaurar una ventana (toggle). */
export function toggleMaximizeWindow(instanceId: string): void {
  const windows = windowStore.get();
  const target = windows.find(w => w.instanceId === instanceId);
  if (!target) return;

  if (target.state === 'maximized') {
    const restored = target.preMaximizeBounds ?? { x: 40, y: 40, w: 640, h: 480 };
    const updated = windows.map(w => {
      if (w.instanceId === instanceId) {
        return { ...w, state: 'open' as const, bounds: clampWindowBounds(restored.x, restored.y, restored.w, restored.h), preMaximizeBounds: undefined };
      }
      return w;
    });
    windowStore.set(updated);
  } else {
    const updated = windows.map(w => {
      if (w.instanceId === instanceId) {
        return { ...w, state: 'maximized' as const, bounds: { x: 0, y: 0, w: workspaceW, h: workspaceH }, preMaximizeBounds: { ...w.bounds } };
      }
      return w;
    });
    windowStore.set(updated);
  }
}

/** Actualizar bounds de una ventana (drag/resize/keyboard). */
export function updateWindowBounds(instanceId: string, bounds: Partial<WindowBounds>): void {
  const windows = windowStore.get();
  const updated = windows.map(w => {
    if (w.instanceId === instanceId) {
      const merged = { ...w.bounds, ...bounds };
      const clamped = clampWindowBounds(merged.x, merged.y, merged.w, merged.h);
      return { ...w, bounds: clamped };
    }
    return w;
  });
  windowStore.set(updated);
}

/** Cerrar todas las ventanas (para cleanup). */
export function closeAllWindows(): void {
  const windows = windowStore.get();
  for (const w of windows) {
    w.controller?.abort();
  }
  windowStore.set([]);
}
