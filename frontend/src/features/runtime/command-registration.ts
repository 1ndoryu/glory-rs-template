/* wandori.us — Command Registration
 * Registra los comandos del sistema del OS.
 * Se importa como side-effect en main.ts junto con app-registration. */

import { CommandRegistry } from './command-registry';
import {
  getFocusedWindow,
  closeWindow,
  minimizeWindow,
  restoreWindow,
  focusWindow,
  getWindows,
} from './window-manager';
import { dispatchEvent } from '../analytics/dispatcher';

/* === Comandos de ventana === */

CommandRegistry.register({
  id: 'window:close',
  label: 'Cerrar ventana',
  shortcut: 'Escape',
  execute: () => {
    const win = getFocusedWindow();
    if (!win) return;
    win.controller.abort();
    dispatchEvent({ type: 'app_closed', appId: win.appId });
    closeWindow(win.instanceId);
  },
});

CommandRegistry.register({
  id: 'window:minimize',
  label: 'Minimizar ventana',
  shortcut: 'Meta+m',
  execute: () => {
    const win = getFocusedWindow();
    if (!win) return;
    minimizeWindow(win.instanceId);
  },
});

CommandRegistry.register({
  id: 'window:focus-next',
  label: 'Siguiente ventana',
  shortcut: 'Ctrl+Shift+ArrowRight',
  execute: () => {
    const windows = getWindows();
    if (windows.length < 2) return;
    const focused = windows.find(w => w.focused);
    const idx = focused ? windows.indexOf(focused) : -1;
    const next = windows[(idx + 1) % windows.length];
    if (next.state === 'minimized') {
      restoreWindow(next.instanceId);
    } else {
      focusWindow(next.instanceId);
    }
  },
});

/* === Handler global de teclado === */

/**
 * Inicializa el handler de atajos de teclado del OS.
 * Se llama una vez desde main.ts después de registrar comandos.
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
