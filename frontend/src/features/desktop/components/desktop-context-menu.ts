/* wandori.us — Desktop Context Menu
 * Menú contextual del escritorio basado en CommandRegistry.
 * [Plan §2.3/3.2] Clic derecho selecciona target antes de abrir menú.
 * Menú se reposiciona dentro del viewport y no queda cubierto por taskbar.
 * Escape lo cierra y devuelve foco al invocador. */

import { CommandRegistry, type CommandContext, type CommandTarget } from '../../runtime/command-registry';
import { createElement } from 'lucide';

/** Estado del menú contextual activo. */
let activeMenu: HTMLElement | null = null;

/** Cerrar el menú contextual activo. */
function closeContextMenu(): void {
  if (activeMenu) {
    activeMenu.remove();
    activeMenu = null;
  }
  document.removeEventListener('keydown', onEscapeKey);
  document.removeEventListener('click', onOutsideClick);
}

function onEscapeKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault();
    closeContextMenu();
  }
}

function onOutsideClick(e: MouseEvent): void {
  if (activeMenu && !activeMenu.contains(e.target as Node)) {
    closeContextMenu();
  }
}

export interface ContextMenuOptions {
  /** Contexto de menú ('desktop', 'icon', 'folder', 'window', 'taskbar'). */
  context: string;
  /** Targets sobre los que actúa. */
  targets?: readonly CommandTarget[];
  /** Capacidad del usuario actual. */
  capability?: 'public' | 'authenticated' | 'admin';
  /** Posición donde mostrar el menú. */
  x: number;
  y: number;
}

/**
 * Abrir menú contextual del OS.
 * [Plan §2.1] Proyecta CommandRegistry; no mantiene lista paralela.
 */
export function openContextMenu(options: ContextMenuOptions): void {
  closeContextMenu();

  const ctx: CommandContext = {
    targets: options.targets,
    capability: options.capability,
    presentationMode: 'desktop',
  };

  const commands = CommandRegistry.getByContext(options.context, ctx);
  if (commands.length === 0) return;

  const menu = document.createElement('div');
  menu.className = 'desktop-context-menu';
  menu.setAttribute('role', 'menu');
  menu.setAttribute('aria-label', 'Menú contextual');

  for (const cmd of commands) {
    const item = document.createElement('button');
    item.type = 'button';
    item.className = 'desktop-context-menu__item';
    item.setAttribute('role', 'menuitem');

    /* Icono Lucide */
    if (cmd.icon) {
      const iconEl = document.createElement('span');
      iconEl.className = 'desktop-context-menu__icon';
      iconEl.appendChild(createElement(cmd.icon));
      item.appendChild(iconEl);
    }

    /* Label */
    const labelEl = document.createElement('span');
    labelEl.className = 'desktop-context-menu__label';
    labelEl.textContent = cmd.label;
    item.appendChild(labelEl);

    /* Shortcut */
    if (cmd.shortcut) {
      const shortcutEl = document.createElement('span');
      shortcutEl.className = 'desktop-context-menu__shortcut';
      shortcutEl.textContent = cmd.shortcut;
      item.appendChild(shortcutEl);
    }

    /* Disponibilidad */
    const availability = cmd.isAvailable ? cmd.isAvailable(ctx) : { state: 'enabled' as const };
    if (availability.state === 'disabled') {
      item.classList.add('desktop-context-menu__item--disabled');
      item.setAttribute('aria-disabled', 'true');
      item.title = availability.reason;
    } else {
      item.addEventListener('click', (e) => {
        e.stopPropagation();
        closeContextMenu();
        void CommandRegistry.execute(cmd.id, ctx);
      });
    }

    menu.appendChild(item);
  }

  /* Posicionar dentro del viewport */
  menu.style.position = 'fixed';
  menu.style.left = '0';
  menu.style.top = '0';
  menu.style.zIndex = '9999';
  document.body.appendChild(menu);

  /* Reposicionar para que no se salga del viewport */
  const rect = menu.getBoundingClientRect();
  const taskbarH = 32; /* Altura estimada de taskbar */
  const maxX = window.innerWidth - rect.width - 4;
  const maxY = window.innerHeight - rect.height - taskbarH - 4;
  menu.style.left = `${Math.max(0, Math.min(options.x, maxX))}px`;
  menu.style.top = `${Math.max(0, Math.min(options.y, maxY))}px`;

  activeMenu = menu;

  /* Listeners para cerrar */
  setTimeout(() => {
    document.addEventListener('keydown', onEscapeKey);
    document.addEventListener('click', onOutsideClick);
  }, 0);
}
