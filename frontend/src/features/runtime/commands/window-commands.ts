/* wandori.us — Window Commands
 * Comandos de gestión de ventanas: cerrar, minimizar, restaurar, enfocar, ciclar. */

import { CommandRegistry, type CommandContext, type CommandResult } from '../command-registry';
import {
  getFocusedWindow,
  closeWindow,
  minimizeWindow,
  restoreWindow,
  focusWindow,
  getWindows,
} from '../window-manager';
import { dispatchEvent } from '../../analytics/dispatcher';

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
    /* closeWindow es el único dueño del abort, teardown y app_closed. */
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
    return { status: 'success' };
  },
});
