import { createElement, Minus, X, type IconNode } from 'lucide';

export interface DesktopWindowOptions {
  title: string;
  content: HTMLElement;
  className?: string;
  active?: boolean;
  resizable?: boolean;
  layout?: 'padded' | 'full-bleed';
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

  const body = document.createElement('div');
  body.className = 'desktop-window__body';
  if (options.layout !== 'full-bleed') {
    body.classList.add('desktop-window__body--padded');
  }
  body.appendChild(options.content);

  titleBar.append(closeControl, title, minimizeControl);
  windowElement.append(titleBar, body);

  return windowElement;
}
