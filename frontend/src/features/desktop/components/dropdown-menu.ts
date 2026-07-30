/* wandori.us — Dropdown Menu
 * Componente compartido para menús desplegables del OS.
 * Unifica la lógica de apertura/cierre, keyboard handling y positioning
 * que estaba duplicada en context-menu, menu-bar y app-toolbar.
 * [Auditoría v2] */

import { createElement, type IconNode } from 'lucide';

export interface DropdownMenuItem {
  readonly icon?: IconNode;
  readonly label: string;
  readonly shortcut?: string;
  readonly disabled?: boolean;
  readonly onClick?: () => void;
}

export interface DropdownMenuOptions {
  /** Elementos del menú. */
  readonly items: readonly DropdownMenuItem[];
  /** Label para accesibilidad. */
  readonly ariaLabel?: string;
  /** Posicionamiento: 'fixed' para context menus, 'absolute' para toolbars. */
  readonly positioning?: 'fixed' | 'absolute';
  /** Posición X (solo para positioning: 'fixed'). */
  readonly x?: number;
  /** Posición Y (solo para positioning: 'fixed'). */
  readonly y?: number;
  /** Callback al cerrar el menú. */
  readonly onClose?: () => void;
}

/** Estado global: solo un dropdown abierto a la vez. */
let activeDropdown: { el: HTMLElement; cleanup: () => void } | null = null;
let pendingSetup: ReturnType<typeof setTimeout> | null = null;

/** Cerrar el dropdown activo. Cancela timeouts pendientes para evitar listener leak. */
function closeActiveDropdown(): void {
  if (pendingSetup) {
    clearTimeout(pendingSetup);
    pendingSetup = null;
  }
  if (activeDropdown) {
    activeDropdown.cleanup();
    activeDropdown.el.remove();
    activeDropdown = null;
  }
}

function onGlobalEscape(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault();
    closeActiveDropdown();
  }
}

function onGlobalClick(e: MouseEvent): void {
  if (activeDropdown && !activeDropdown.el.contains(e.target as Node)) {
    closeActiveDropdown();
  }
}

/** Crear un item del menú con icono, label, shortcut y estado disabled. */
export function createDropdownItem(item: DropdownMenuItem): HTMLElement {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = 'desktop-context-menu__item';
  el.setAttribute('role', 'menuitem');

  if (item.icon) {
    const iconEl = document.createElement('span');
    iconEl.className = 'desktop-context-menu__icon';
    iconEl.appendChild(createElement(item.icon));
    el.appendChild(iconEl);
  }

  const labelEl = document.createElement('span');
  labelEl.className = 'desktop-context-menu__label';
  labelEl.textContent = item.label;
  el.appendChild(labelEl);

  if (item.shortcut) {
    const shortcutEl = document.createElement('span');
    shortcutEl.className = 'desktop-context-menu__shortcut';
    shortcutEl.textContent = item.shortcut;
    el.appendChild(shortcutEl);
  }

  if (item.disabled) {
    el.classList.add('desktop-context-menu__item--disabled');
    el.setAttribute('aria-disabled', 'true');
  } else if (item.onClick) {
    el.addEventListener('click', (e) => {
      e.stopPropagation();
      closeActiveDropdown();
      item.onClick!();
    });
  }

  return el;
}

/**
 * Abrir un dropdown menu compartido.
 * Cierra cualquier dropdown activo antes de abrir uno nuevo.
 * Maneja Escape, outside click y positioning automáticamente.
 */
export function openDropdownMenu(options: DropdownMenuOptions): HTMLElement | null {
  closeActiveDropdown();

  if (options.items.length === 0) return null;

  const menu = document.createElement('div');
  menu.className = 'desktop-context-menu';
  menu.setAttribute('role', 'menu');
  if (options.ariaLabel) menu.setAttribute('aria-label', options.ariaLabel);

  for (const item of options.items) {
    menu.appendChild(createDropdownItem(item));
  }

  /* Posicionar */
  if (options.positioning === 'fixed') {
    menu.style.position = 'fixed';
    menu.style.left = '0';
    menu.style.top = '0';
    menu.style.zIndex = '9999';
  }

  document.body.appendChild(menu);

  /* Ajustar posición para no salir del viewport (solo fixed) */
  if (options.positioning === 'fixed' && options.x !== undefined && options.y !== undefined) {
    const rect = menu.getBoundingClientRect();
    const taskbarH = 32;
    const maxX = window.innerWidth - rect.width - 4;
    const maxY = window.innerHeight - rect.height - taskbarH - 4;
    menu.style.left = `${Math.max(0, Math.min(options.x, maxX))}px`;
    menu.style.top = `${Math.max(0, Math.min(options.y, maxY))}px`;
  }

  const cleanup = (): void => {
    document.removeEventListener('keydown', onGlobalEscape);
    document.removeEventListener('click', onGlobalClick);
    options.onClose?.();
  };

  activeDropdown = { el: menu, cleanup };

  pendingSetup = setTimeout(() => {
    pendingSetup = null;
    document.addEventListener('keydown', onGlobalEscape);
    document.addEventListener('click', onGlobalClick);
  }, 0);

  return menu;
}
