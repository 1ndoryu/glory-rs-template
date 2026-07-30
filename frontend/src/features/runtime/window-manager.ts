/* wandori.us — Window Manager
 * Máquina de estados reactiva para ventanas del escritorio.
 * Usa el store pub/sub existente. Cada ventana tiene bounds, estado y AbortController.
 * El taskbar y el shell se suscriben a windowStore para reaccionar a cambios. */

import { createStore, type Store } from '../../store';
import type { MountedView } from '../../core/lifecycle';
import type { AppDefinition, AppToolbarGroup } from './app-registry';
import type { IconNode } from 'lucide';

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
  /** AbortController de esta instancia — se aborta al cerrar. Shell windows no tienen controller. */
  readonly controller?: AbortController;
  /** Definición de la app (referencia). Shell windows no tienen app. */
  readonly app?: AppDefinition;
  /** Icono override para shell windows (sin AppDefinition). */
  readonly icon?: IconNode;
  /** CSS class override para shell windows (ej: 'desktop-profile-window'). */
  readonly cssClass?: string;
  /** Layout del body: 'padded' (default) o 'full-bleed'. */
  readonly layout?: 'padded' | 'full-bleed';
  /** Grupos del toolbar de la app (referencian Command IDs). */
  readonly toolbar?: AppToolbarGroup[];
  /** Parámetros de instancia (folderId para Finder, resourceId para Reader, etc.). */
  readonly params?: Readonly<Record<string, string>>;
  /** Clave derivada de params para buscar ventanas con los mismos parámetros. */
  readonly _paramKey?: string;
}

/* === Store reactivo === */
export const windowStore: Store<WindowEntry[]> = createStore([]);

let nextZIndex = 10;
let nextWindowId = 1;

/* === Workspace bounds (set by shell after DOM creation) === */
let workspaceW = 1200;
let workspaceH = 800;

/** Called once by desktop-shell after the window container is in the DOM. */
export function setWorkspaceBounds(w: number, h: number): void {
  workspaceW = w;
  workspaceH = h;
}

/** Shared clamp: keeps a window partially visible within the workspace. */
export function clampWindowBounds(x: number, y: number, w: number, h: number): { x: number; y: number; w: number; h: number } {
  const minVisible = 60;
  const titleH = 24;
  return {
    x: Math.max(-w + minVisible, Math.min(workspaceW - minVisible, x)),
    y: Math.max(0, Math.min(workspaceH - titleH, y)),
    w: Math.min(w, workspaceW),
    h: Math.min(h, workspaceH),
  };
}

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

  /* Desenfocar todas las existentes */
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
  };

  windowStore.set([...updated, entry]);
  return instanceId;
}

/** Cerrar una ventana (destruye contenido y aborta signal). */
export function closeWindow(instanceId: string): void {
  const windows = windowStore.get();
  const target = windows.find(w => w.instanceId === instanceId);
  if (!target) return;

  /* Abortar el signal de la app (shell windows no tienen controller) */
  target.controller?.abort();
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

/** Registrar una shell window (perfil, etc.) directamente en windowStore.
 * Las shell windows no tienen AppDefinition ni AbortController. */
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
  /* No registrar dos veces */
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

/** Actualizar bounds de una ventana (drag/resize/keyboard).
 * [Plan §4] Aplica boundary clamping automáticamente. */
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

/** Verificar si una app singleton ya está abierta. */
export function findOpenWindow(appId: string): WindowEntry | undefined {
  return windowStore.get().find(w => w.appId === appId);
}

/** Cerrar todas las ventanas (para cleanup). */
export function closeAllWindows(): void {
  const windows = windowStore.get();
  for (const w of windows) {
    w.controller?.abort();
  }
  windowStore.set([]);
}
