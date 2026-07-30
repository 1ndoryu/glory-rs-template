import { createElement, Minus, X, type IconNode } from 'lucide';
import type { AppMenu } from '../../runtime/app-registry';

export interface DesktopWindowOptions {
  title: string;
  content: HTMLElement;
  className?: string;
  active?: boolean;
  resizable?: boolean;
  layout?: 'padded' | 'full-bleed';
  menus?: AppMenu[];
  onClose?: () => void;
  onMinimize?: () => void;
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

  /* App toolbar — barra de menús declarativa de la app */
  if (options.menus && options.menus.length > 0) {
    const toolbar = createAppToolbar(options.menus);
    windowElement.appendChild(toolbar);
  }

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

/**
 * Crea la barra de herramientas de una app a partir de sus menús declarativos.
 * Reutiliza el mismo patrón visual que desktop-menu-bar pero a nivel de ventana.
 */
export function createAppToolbar(menus: AppMenu[]): HTMLElement {
  const toolbar = document.createElement('div');
  toolbar.className = 'desktop-app-toolbar';

  for (const menu of menus) {
    const entry = document.createElement('div');
    entry.className = 'desktop-app-toolbar__entry';

    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'desktop-app-toolbar__item';
    btn.textContent = menu.label;
    btn.setAttribute('aria-haspopup', 'menu');
    btn.setAttribute('aria-expanded', 'false');

    const dropdown = document.createElement('div');
    dropdown.className = 'desktop-app-toolbar__dropdown';
    dropdown.setAttribute('role', 'menu');
    dropdown.hidden = true;

    for (const item of menu.items) {
      if (item.label === '---') {
        /* Separador */
        const sep = document.createElement('div');
        sep.className = 'desktop-app-toolbar__separator';
        dropdown.appendChild(sep);
        continue;
      }

      const menuItem = document.createElement('button');
      menuItem.type = 'button';
      menuItem.className = 'desktop-app-toolbar__menu-item';
      menuItem.setAttribute('role', 'menuitem');

      if (item.icon) {
        const iconEl = document.createElement('span');
        iconEl.className = 'desktop-app-toolbar__icon';
        iconEl.appendChild(createElement(item.icon));
        menuItem.appendChild(iconEl);
      }

      const labelEl = document.createElement('span');
      labelEl.className = 'desktop-app-toolbar__label';
      labelEl.textContent = item.label;
      menuItem.appendChild(labelEl);

      if (item.shortcut) {
        const shortcutEl = document.createElement('span');
        shortcutEl.className = 'desktop-app-toolbar__shortcut';
        shortcutEl.textContent = item.shortcut;
        menuItem.appendChild(shortcutEl);
      }

      if (item.disabled) {
        menuItem.classList.add('desktop-app-toolbar__menu-item--disabled');
        menuItem.setAttribute('aria-disabled', 'true');
      } else {
        menuItem.addEventListener('click', (e) => {
          e.stopPropagation();
          closeToolbarMenu();
          item.execute();
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
