/* wandori.us — Window Store
 * Tipos, store reactivo, workspace bounds y funciones getter para ventanas.
 * Extraído de window-manager.ts para reducir tamaño bajo límite de 300 líneas.
 * [Auditoría v3 §2.2] */

import { createStore, type Store } from '../../store';
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
  /** Cleanup callback de la app (MountedView.destroy). Se invoca al cerrar. */
  readonly onDestroy?: () => void;
  /** Bounds anteriores al maximizar (para restaurar). */
  preMaximizeBounds?: WindowBounds;
}

/* === Store reactivo === */
export const windowStore: Store<WindowEntry[]> = createStore([]);

/* === Workspace bounds (set by shell after DOM creation) === */
export let workspaceW = 1200;
export let workspaceH = 800;

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

let nextWindowId = 1;
let nextZIndex = 10;

export function generateWindowId(): string {
  return `win-${nextWindowId++}`;
}

/** Siguiente z-index para apilamiento de ventanas.
 * [Auditoría v4 §1.3] Movido de window-manager.ts para consolidar estado mutable. */
export function generateNextZIndex(): number {
  return nextZIndex++;
}

/** Resetear contadores (solo para tests — prefijo _ indica API interna). */
export function _resetWindowCountersForTest(): void {
  nextWindowId = 1;
  nextZIndex = 10;
}

/** Obtener todas las ventanas. */
export function getWindows(): WindowEntry[] {
  return windowStore.get();
}

/** Obtener la ventana con foco. */
export function getFocusedWindow(): WindowEntry | undefined {
  return windowStore.get().find(w => w.focused);
}

/** Verificar si una app singleton ya está abierta. */
export function findOpenWindow(appId: string): WindowEntry | undefined {
  return windowStore.get().find(w => w.appId === appId);
}
