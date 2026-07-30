/* wandori.us — Command Registration
 * Registra los comandos del sistema del OS.
 * [Plan §2.2] Catálogo mínimo de shell y ventanas.
 * Se importa como side-effect en main.ts junto con app-registration. */

import { CommandRegistry, type CommandContext, type CommandResult } from './command-registry';
import {
  getFocusedWindow,
  closeWindow,
  minimizeWindow,
  restoreWindow,
  focusWindow,
  getWindows,
  findOpenWindow,
  updateWindowBounds,
} from './window-manager';
import { AppRegistry } from './app-registry';
import { dispatchEvent } from '../analytics/dispatcher';
import { tombstoneNode, restoreNode, resetOverlay, workspaceStore, publishWorkspace, setClipboard, getClipboard, pasteFromClipboard, createFolder } from './workspace/workspace-store';

/* === Comandos de ventana === */

CommandRegistry.register({
  id: 'window:close',
  label: 'Cerrar ventana',
  shortcut: 'Escape',
  order: 10,
  contexts: ['window'],
  undoPolicy: 'none',
  analyticsEvent: 'window.closed',
  isAvailable: () => {
    const win = getFocusedWindow();
    return win ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    win.controller?.abort();
    dispatchEvent({ type: 'app_closed', appId: win.appId });
    closeWindow(win.instanceId);
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:minimize',
  label: 'Minimizar ventana',
  shortcut: 'Meta+m',
  order: 11,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.minimized',
  isAvailable: () => {
    const win = getFocusedWindow();
    if (!win) return { state: 'disabled', reason: 'no focused window' };
    return win.state === 'minimized'
      ? { state: 'disabled', reason: 'already minimized' }
      : { state: 'enabled' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    minimizeWindow(win.instanceId);
    dispatchEvent({ type: 'window_minimized', appId: win.appId });
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:restore',
  label: 'Restaurar ventana',
  order: 12,
  contexts: ['window', 'taskbar'],
  undoPolicy: 'local',
  analyticsEvent: 'window.restored',
  isAvailable: (ctx) => {
    const win = ctx.targets?.[0]?.id
      ? getWindows().find(w => w.instanceId === ctx.targets![0].id)
      : getFocusedWindow();
    if (!win) return { state: 'disabled', reason: 'no window' };
    return win.state === 'minimized'
      ? { state: 'enabled' }
      : { state: 'disabled', reason: 'not minimized' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const instanceId = ctx?.targets?.[0]?.id;
    const win = instanceId
      ? getWindows().find(w => w.instanceId === instanceId)
      : getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no window' };
    restoreWindow(win.instanceId);
    dispatchEvent({ type: 'window_restored', appId: win.appId });
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:focus',
  label: 'Enfocar ventana',
  order: 13,
  contexts: ['taskbar'],
  undoPolicy: 'local',
  analyticsEvent: 'window.focused',
  isAvailable: (ctx) => {
    const instanceId = ctx.targets?.[0]?.id;
    const win = instanceId
      ? getWindows().find(w => w.instanceId === instanceId)
      : undefined;
    if (!win) return { state: 'disabled', reason: 'no window' };
    return win.focused
      ? { state: 'disabled', reason: 'already focused' }
      : { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const instanceId = ctx?.targets?.[0]?.id;
    if (!instanceId) return { status: 'failure', reason: 'no target' };
    focusWindow(instanceId);
    const win = getWindows().find(w => w.instanceId === instanceId);
    if (win) dispatchEvent({ type: 'window_focused', appId: win.appId });
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:focus-next',
  label: 'Siguiente ventana',

  order: 14,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.cycle',
  isAvailable: () => {
    const windows = getWindows();
    return windows.length >= 2
      ? { state: 'enabled' }
      : { state: 'disabled', reason: 'less than 2 windows' };
  },
  execute: (): CommandResult => {
    const windows = getWindows();
    if (windows.length < 2) return { status: 'failure', reason: 'less than 2 windows' };
    const focused = windows.find(w => w.focused);
    const idx = focused ? windows.indexOf(focused) : -1;
    const next = windows[(idx + 1) % windows.length];
    if (next.state === 'minimized') {
      restoreWindow(next.instanceId);
      dispatchEvent({ type: 'window_restored', appId: next.appId });
    } else {
      focusWindow(next.instanceId);
    }
    dispatchEvent({ type: 'window_focused', appId: next.appId });
    return { status: 'success' };
  },
});

/* === Comandos de app === */

CommandRegistry.register({
  id: 'app:open',
  label: 'Abrir aplicación',
  order: 1,
  contexts: ['desktop', 'shortcut', 'taskbar'],
  requires: 'public',
  undoPolicy: 'none',
  analyticsEvent: 'app.opened',
  isAvailable: (ctx) => {
    const appId = ctx.targets?.[0]?.id;
    if (!appId) return { state: 'disabled', reason: 'no app target' };
    const app = AppRegistry.get(appId);
    if (!app) return { state: 'hidden' };
    /* Verificar capacidad */
    if (app.requires === 'admin' && ctx.capability !== 'admin') {
      return { state: 'hidden' };
    }
    if (app.requires === 'authenticated' && !ctx.capability) {
      return { state: 'hidden' };
    }
    return { state: 'enabled' };
  },
  execute: async (ctx?: CommandContext): Promise<CommandResult> => {
    const appId = ctx?.targets?.[0]?.id;
    if (!appId) return { status: 'failure', reason: 'no app target' };
    /* Dinámico para evitar circular imports */
    const { openAppWindow } = await import('./route-app-adapter');
    await openAppWindow(appId);
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'app:focus',
  label: 'Enfocar aplicación',
  order: 2,
  contexts: ['taskbar', 'shortcut'],
  undoPolicy: 'local',
  analyticsEvent: 'app.focused',
  isAvailable: (ctx) => {
    const appId = ctx.targets?.[0]?.id;
    if (!appId) return { state: 'disabled', reason: 'no app target' };
    const win = findOpenWindow(appId);
    if (!win) return { state: 'hidden' };
    return win.focused
      ? { state: 'disabled', reason: 'already focused' }
      : { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const appId = ctx?.targets?.[0]?.id;
    if (!appId) return { status: 'failure', reason: 'no app target' };
    const win = findOpenWindow(appId);
    if (!win) return { status: 'failure', reason: 'app not open' };
    if (win.state === 'minimized') restoreWindow(win.instanceId);
    focusWindow(win.instanceId);
    dispatchEvent({ type: 'window_focused', appId });
    return { status: 'success' };
  },
});

/* === Comandos de geometría por teclado (Plan §4.1) === */

const KB_STEP = 20;
const KB_STEP_LARGE = 60;

CommandRegistry.register({
  id: 'window:move-up',
  label: 'Mover ventana arriba',
  shortcut: 'Ctrl+ArrowUp',
  order: 20,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.moved',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    updateWindowBounds(win.instanceId, { y: win.bounds.y - KB_STEP });
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:move-down',
  label: 'Mover ventana abajo',
  shortcut: 'Ctrl+ArrowDown',
  order: 21,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.moved',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    updateWindowBounds(win.instanceId, { y: win.bounds.y + KB_STEP });
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:move-left',
  label: 'Mover ventana izquierda',
  shortcut: 'Ctrl+ArrowLeft',
  order: 22,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.moved',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    updateWindowBounds(win.instanceId, { x: win.bounds.x - KB_STEP });
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:move-right',
  label: 'Mover ventana derecha',
  shortcut: 'Ctrl+ArrowRight',
  order: 23,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.moved',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    updateWindowBounds(win.instanceId, { x: win.bounds.x + KB_STEP });
    return { status: 'success' };
  },
});

/* Resize direccional: expande hacia la flecha presionada, borde opuesto fijo */

CommandRegistry.register({
  id: 'window:resize-right',
  label: 'Expandir derecha',
  shortcut: 'Ctrl+Shift+ArrowRight',
  order: 24,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.resized',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    updateWindowBounds(win.instanceId, { w: win.bounds.w + KB_STEP_LARGE });
    /* x stays fixed — right edge extends */
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:resize-left',
  label: 'Expandir izquierda',
  shortcut: 'Ctrl+Shift+ArrowLeft',
  order: 25,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.resized',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    const newW = Math.max(240, win.bounds.w + KB_STEP_LARGE);
    const dx = newW - win.bounds.w;
    updateWindowBounds(win.instanceId, { x: win.bounds.x - dx, w: newW });
    /* left edge extends, right edge stays fixed */
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:resize-down',
  label: 'Expandir abajo',
  shortcut: 'Ctrl+Shift+ArrowDown',
  order: 26,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.resized',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    updateWindowBounds(win.instanceId, { h: win.bounds.h + KB_STEP_LARGE });
    /* y stays fixed — bottom edge extends */
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'window:resize-up',
  label: 'Expandir arriba',
  shortcut: 'Ctrl+Shift+ArrowUp',
  order: 27,
  contexts: ['window'],
  undoPolicy: 'local',
  analyticsEvent: 'window.resized',
  isAvailable: () => {
    return getFocusedWindow() ? { state: 'enabled' } : { state: 'disabled', reason: 'no focused window' };
  },
  execute: (): CommandResult => {
    const win = getFocusedWindow();
    if (!win) return { status: 'failure', reason: 'no focused window' };
    const newH = Math.max(180, win.bounds.h + KB_STEP_LARGE);
    const dy = newH - win.bounds.h;
    updateWindowBounds(win.instanceId, { y: win.bounds.y - dy, h: newH });
    /* top edge extends, bottom edge stays fixed */
    return { status: 'success' };
  },
});

/* === Comandos de workspace (297A-11) === */

/** Resolver nodeId del workspace desde targetId (que puede ser refId o nodeId). */
function resolveWorkspaceNodeId(targetId: string): string | undefined {
  const ws = workspaceStore.get();
  const node = Object.values(ws.nodes).find(
    (n) => n.id === targetId || n.refId === targetId,
  );
  return node?.id;
}

CommandRegistry.register({
  id: 'workspace:trash',
  label: 'Eliminar',
  order: 30,
  contexts: ['icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.trash',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found in workspace' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    tombstoneNode(nodeId);
    return { status: 'success' };
  },
});

/* [297A-11] Restaurar nodo de papelera — requiere UI de papelera (pendiente). */
CommandRegistry.register({
  id: 'workspace:restore',
  label: 'Restaurar',
  order: 31,
  contexts: ['trash'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.restore',
  isAvailable: (ctx) => {
    const nodeId = ctx.targets?.[0]?.id;
    if (!nodeId) return { state: 'hidden', reason: 'no target' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const nodeId = ctx?.targets?.[0]?.id;
    if (!nodeId) return { status: 'failure', reason: 'no target' };
    restoreNode(nodeId);
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:reset',
  label: 'Restablecer escritorio',
  order: 32,
  contexts: ['desktop'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.reset',
  isAvailable: () => {
    /* Siempre disponible — reset con overlay vacío es no-op seguro. */
    return { state: 'enabled' };
  },
  execute: (): CommandResult => {
    resetOverlay();
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:publish',
  label: 'Publicar escritorio',
  order: 33,
  contexts: ['desktop'],
  requires: 'admin',
  undoPolicy: 'none',
  analyticsEvent: 'workspace.published',
  isAvailable: (ctx) => {
    if (ctx.capability !== 'admin') return { state: 'hidden' };
    return { state: 'enabled' };
  },
  execute: async (): Promise<CommandResult> => {
    const result = await publishWorkspace();
    if (result) {
      return { status: 'success' };
    }
    return { status: 'failure', reason: 'Error al publicar' };
  },
});

/* === Comandos de clipboard (297A-11 §9.3) === */

CommandRegistry.register({
  id: 'workspace:copy',
  label: 'Copiar',
  shortcut: 'ctrl+c',
  order: 40,
  contexts: ['icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.copy',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    setClipboard([nodeId], 'copy');
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:cut',
  label: 'Cortar',
  shortcut: 'ctrl+x',
  order: 41,
  contexts: ['icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.cut',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    setClipboard([nodeId], 'cut');
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:paste',
  label: 'Pegar',
  shortcut: 'ctrl+v',
  order: 42,
  contexts: ['desktop', 'folder', 'icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.paste',
  isAvailable: () => {
    const clip = getClipboard();
    if (!clip || clip.nodeIds.length === 0) return { state: 'disabled', reason: 'clipboard vacío' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    /* Pegar en el nodo seleccionado (si es carpeta) o en desktop */
    const targetId = ctx?.targets?.[0]?.id;
    let parentId: string = 'desktop';
    if (targetId) {
      const ws = workspaceStore.get();
      const nodeId = resolveWorkspaceNodeId(targetId);
      if (nodeId && ws.nodes[nodeId]?.type === 'folder') {
        parentId = nodeId;
      }
    }
    const pasted = pasteFromClipboard(parentId);
    if (pasted.length === 0) return { status: 'failure', reason: 'no se pudo pegar (ciclo o destino inválido)' };
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:create-folder',
  label: 'Nueva carpeta',
  order: 43,
  contexts: ['desktop', 'icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.create_folder',
  isAvailable: () => {
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    let parentId: string = 'desktop';
    if (targetId) {
      const ws = workspaceStore.get();
      const nodeId = resolveWorkspaceNodeId(targetId);
      if (nodeId && ws.nodes[nodeId]?.type === 'folder') {
        parentId = nodeId;
      }
    }
    createFolder(parentId, 'Nueva carpeta');
    return { status: 'success' };
  },
});

/* === Handler global de teclado === */

/**
 * Inicializa el handler de atajos de teclado del OS.
 * Se llama una vez desde main.ts después de registrar comandos.
 * [Plan §2.1] Atajos son proyecciones del CommandRegistry.
 */
export function initKeyboardShortcuts(): void {
  document.addEventListener('keydown', (e) => {
    /* No interceptar si el foco está en un input/textarea/contenteditable */
    const target = e.target as HTMLElement;
    if (
      target.tagName === 'INPUT'
      || target.tagName === 'TEXTAREA'
      || target.isContentEditable
    ) {
      /* Solo Escape aplica dentro de inputs (cerrar modal/ventana) */
      if (e.key !== 'Escape') return;
    }

    const commands = CommandRegistry.getWithShortcuts();
    for (const cmd of commands) {
      if (matchesShortcut(e, cmd.shortcut!)) {
        e.preventDefault();
        void CommandRegistry.execute(cmd.id);
        return;
      }
    }
  });
}

/** Verifica si un evento de teclado coincide con un atajo. */
function matchesShortcut(e: KeyboardEvent, shortcut: string): boolean {
  const parts = shortcut.toLowerCase().split('+');
  const key = parts[parts.length - 1];
  const modifiers = parts.slice(0, -1);

  const keyMatch = e.key.toLowerCase() === key || (key.length === 1 && e.code.toLowerCase() === `key${key}`);
  if (!keyMatch) return false;

  for (const mod of modifiers) {
    switch (mod) {
      case 'meta':
      case 'cmd':
        if (!e.metaKey) return false;
        break;
      case 'ctrl':
        if (!e.ctrlKey) return false;
        break;
      case 'alt':
        if (!e.altKey) return false;
        break;
      case 'shift':
        if (!e.shiftKey) return false;
        break;
    }
  }

  return true;
}
