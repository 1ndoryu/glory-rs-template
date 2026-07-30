import { createElement, Maximize2, Minus, X, type IconNode } from 'lucide';
import type { AppToolbarGroup, ToolbarItemRef } from '../../runtime/app-registry';
import { CommandRegistry, type CommandContext } from '../../runtime/command-registry';
import { authStore } from '../../../store';

export interface DesktopWindowOptions {
  title: string;
  content: HTMLElement;
  className?: string;
  active?: boolean;
  resizable?: boolean;
  layout?: 'padded' | 'full-bleed';
  toolbar?: AppToolbarGroup[];
  onClose?: () => void;
  onMinimize?: () => void;
  onMaximize?: () => void;
}

function createWindowControl(
  icon: IconNode,
  className: string,
  label: string,
  onActivate?: () => void,
): HTMLButtonElement {
  const control = document.createElement('button');
  control.type = 'button';
  control.className = `desktop-window__control ${className}`;
  control.disabled = !onActivate;
  control.setAttribute('aria-label', label);
  control.appendChild(createElement(icon));

  if (onActivate) control.addEventListener('click', onActivate);
  return control;
}

/* [297A-2] Receta visual única para todas las futuras aplicaciones.
 * Los controles son decorativos hasta que exista el gestor de ventanas. */
export function createDesktopWindow(options: DesktopWindowOptions): HTMLElement {
  const windowElement = document.createElement('section');
  windowElement.className = 'desktop-window';
  windowElement.setAttribute('aria-label', `Ventana ${options.title}`);

  if (options.className) windowElement.classList.add(...options.className.split(' '));
  if (options.active) windowElement.classList.add('desktop-window--active');
  if (options.resizable) windowElement.classList.add('desktop-window--resizable');

  const titleBar = document.createElement('header');
  titleBar.className = 'desktop-window__titlebar';

  const closeControl = createWindowControl(
    X,
    'desktop-window__control--close',
    `Cerrar ${options.title}`,
    options.onClose,
  );

  const title = document.createElement('span');
  title.className = 'desktop-window__title';
  title.textContent = options.title;

  const minimizeControl = createWindowControl(
    Minus,
    'desktop-window__control--minimize',
    `Minimizar ${options.title}`,
    options.onMinimize,
  );

  titleBar.append(closeControl, title, minimizeControl);
  windowElement.append(titleBar);

  /* App toolbar — siempre presente. Menú 'Ventana' por defecto + toolbar de la app. */
  const allGroups: AppToolbarGroup[] = [
    {
      label: 'Ventana',
      items: [
        { id: 'window:minimize', label: 'Minimizar', icon: Minus },
        { id: 'window:maximize', label: 'Maximizar', icon: Maximize2 },
        '---',
        { id: 'window:close', label: 'Cerrar', icon: X },
      ],
    },
    ...(options.toolbar ?? []),
  ];
  windowElement.appendChild(createAppToolbar(allGroups, { onClose: options.onClose, onMinimize: options.onMinimize, onMaximize: options.onMaximize }));

  const body = document.createElement('div');
  body.className = 'desktop-window__body';
  if (options.layout !== 'full-bleed') {
    body.classList.add('desktop-window__body--padded');
  }
  body.appendChild(options.content);
  windowElement.appendChild(body);

  return windowElement;
}

/* === App Toolbar === */

let openToolbarEntry: HTMLElement | null = null;

function closeToolbarMenu(): void {
  if (openToolbarEntry) {
    openToolbarEntry.classList.remove('desktop-app-toolbar__entry--open');
    const menu = openToolbarEntry.querySelector('.desktop-app-toolbar__dropdown') as HTMLElement | null;
    if (menu) menu.hidden = true;
    openToolbarEntry = null;
  }
  document.removeEventListener('click', onToolbarOutsideClick);
  document.removeEventListener('keydown', onToolbarEscape);
}

function onToolbarOutsideClick(e: MouseEvent): void {
  if (openToolbarEntry && !openToolbarEntry.contains(e.target as Node)) {
    closeToolbarMenu();
  }
}

function onToolbarEscape(e: KeyboardEvent): void {
  if (e.key === 'Escape') closeToolbarMenu();
}

function toggleToolbarEntry(entry: HTMLElement): void {
  if (openToolbarEntry === entry) {
    closeToolbarMenu();
    return;
  }
  closeToolbarMenu();

  const menu = entry.querySelector('.desktop-app-toolbar__dropdown') as HTMLElement | null;
  entry.classList.add('desktop-app-toolbar__entry--open');
  if (menu) menu.hidden = false;

  openToolbarEntry = entry;
  setTimeout(() => {
    document.addEventListener('click', onToolbarOutsideClick);
    document.addEventListener('keydown', onToolbarEscape);
  }, 0);
}

/** Resolver un ToolbarItemRef a label, icon y command. */
function resolveToolbarItem(
  ref: ToolbarItemRef,
): { id: string; label: string; icon?: IconNode; shortcut?: string } | null {
  if (ref === '---' || (typeof ref === 'object' && ref.id === '---')) return null;
  if (typeof ref === 'string') {
    const cmd = CommandRegistry.get(ref);
    if (!cmd) return null;
    return { id: ref, label: cmd.label, icon: cmd.icon, shortcut: cmd.shortcut };
  }
  const cmd = CommandRegistry.get(ref.id);
  if (!cmd && !ref.label) return null;
  return {
    id: ref.id,
    label: ref.label ?? (cmd ? cmd.label : ref.id),
    icon: (ref.icon === null ? undefined : ref.icon) ?? cmd?.icon,
    shortcut: cmd?.shortcut,
  };
}

/**
 * Crea la barra de herramientas de una app a partir de sus grupos declarativos.
 * Los items referencian comandos del CommandRegistry — fuente única de verdad.
 */
export function createAppToolbar(
  groups: AppToolbarGroup[],
  callbacks?: { onClose?: () => void; onMinimize?: () => void; onMaximize?: () => void },
): HTMLElement {
  const toolbar = document.createElement('div');
  toolbar.className = 'desktop-app-toolbar';

  const ctx: CommandContext = {
    capability: authStore.get().isAuthenticated ? 'admin' : 'public',
    presentationMode: 'desktop',
  };

  for (const group of groups) {
    const entry = document.createElement('div');
    entry.className = 'desktop-app-toolbar__entry';

    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'desktop-app-toolbar__item';
    btn.textContent = group.label;
    btn.setAttribute('aria-haspopup', 'menu');
    btn.setAttribute('aria-expanded', 'false');

    const dropdown = document.createElement('div');
    dropdown.className = 'desktop-app-toolbar__dropdown';
    dropdown.setAttribute('role', 'menu');
    dropdown.hidden = true;

    for (const ref of group.items) {
      /* Separador */
      if (ref === '---' || (typeof ref === 'object' && ref.id === '---')) {
        const sep = document.createElement('div');
        sep.className = 'desktop-app-toolbar__separator';
        dropdown.appendChild(sep);
        continue;
      }

      const resolved = resolveToolbarItem(ref);
      if (!resolved) continue;

      /* Verificar disponibilidad desde CommandRegistry */
      const cmd = CommandRegistry.get(resolved.id);
      type Avail = { state: 'enabled' } | { state: 'disabled'; reason: string } | { state: 'hidden' };
      let availability: Avail = { state: 'enabled' };
      if (cmd?.isAvailable) {
        availability = cmd.isAvailable(ctx);
        if (availability.state === 'hidden') continue;
      }

      const menuItem = document.createElement('button');
      menuItem.type = 'button';
      menuItem.className = 'desktop-app-toolbar__menu-item';
      menuItem.setAttribute('role', 'menuitem');

      if (resolved.icon) {
        const iconEl = document.createElement('span');
        iconEl.className = 'desktop-app-toolbar__icon';
        iconEl.appendChild(createElement(resolved.icon));
        menuItem.appendChild(iconEl);
      }

      const labelEl = document.createElement('span');
      labelEl.className = 'desktop-app-toolbar__label';
      labelEl.textContent = resolved.label;
      menuItem.appendChild(labelEl);

      if (resolved.shortcut) {
        const shortcutEl = document.createElement('span');
        shortcutEl.className = 'desktop-app-toolbar__shortcut';
        shortcutEl.textContent = resolved.shortcut;
        menuItem.appendChild(shortcutEl);
      }

      /* Callbacks especiales para window:minimize, window:maximize y window:close */
      const windowCallbackMap: Record<string, 'onClose' | 'onMinimize' | 'onMaximize'> = {
        'window:close': 'onClose',
        'window:minimize': 'onMinimize',
        'window:maximize': 'onMaximize',
      };
      const callbackKey = windowCallbackMap[resolved.id];
      const isWindowCmd = !!callbackKey;
      const callbackDisabled = isWindowCmd && !callbacks?.[callbackKey];

      if (availability.state !== 'enabled' || callbackDisabled) {
        menuItem.classList.add('desktop-app-toolbar__menu-item--disabled');
        menuItem.setAttribute('aria-disabled', 'true');
      } else {
        menuItem.addEventListener('click', (e) => {
          e.stopPropagation();
          closeToolbarMenu();
          /* Ejecutar callback especial o command del registry */
          if (resolved.id === 'window:minimize') { callbacks?.onMinimize?.(); return; }
          if (resolved.id === 'window:maximize') { callbacks?.onMaximize?.(); return; }
          if (resolved.id === 'window:close') { callbacks?.onClose?.(); return; }
          void CommandRegistry.execute(resolved.id, ctx);
        });
      }

      dropdown.appendChild(menuItem);
    }

    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      toggleToolbarEntry(entry);
    });

    entry.append(btn, dropdown);
    toolbar.appendChild(entry);
  }

  return toolbar;
}
