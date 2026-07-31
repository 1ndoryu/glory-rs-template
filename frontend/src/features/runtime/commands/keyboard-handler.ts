/* wandori.us — Keyboard Handler
 * Handler global de atajos de teclado del OS. [Plan §2.1] */

import { CommandRegistry } from '../command-registry';

export function initKeyboardShortcuts(): () => void {
  const onKeyDown = (e: KeyboardEvent): void => {
    const target = e.target as HTMLElement;
    if (
      target.tagName === 'INPUT'
      || target.tagName === 'TEXTAREA'
      || target.isContentEditable
    ) {
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
  };

  document.addEventListener('keydown', onKeyDown);
  return () => { document.removeEventListener('keydown', onKeyDown); };
}

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
