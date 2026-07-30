import { createElement, type IconNode } from 'lucide';

export type DesktopIconType = 'folder' | 'document' | 'application';

export interface DesktopIconOptions {
  label: string;
  type: DesktopIconType;
  selected?: boolean;
  lucideIcon: IconNode;
  onActivate?: () => void;
}

/* [297A-2] Cada objeto conserva su tipo semántico, pero usa nodos oficiales de Lucide
 * para mantener una gramática monocroma uniforme; la activación llegará con el registro de apps. */
export function createDesktopIcon(options: DesktopIconOptions): HTMLElement {
  const onActivate = options.onActivate;
  const icon = onActivate
    ? document.createElement('button')
    : document.createElement('div');
  icon.className = 'desktop-icon';
  icon.setAttribute('aria-label', options.label);

  if (icon instanceof HTMLButtonElement && onActivate) {
    icon.type = 'button';
    icon.classList.add('desktop-icon--interactive');
    icon.addEventListener('click', onActivate);
  }

  if (options.selected) {
    icon.classList.add('desktop-icon--selected');
  }

  const pictogram = document.createElement('span');
  pictogram.className = `desktop-icon__pictogram desktop-icon__pictogram--${options.type}`;
  pictogram.setAttribute('aria-hidden', 'true');

  pictogram.classList.add('desktop-icon__pictogram--lucide');
  pictogram.appendChild(createElement(options.lucideIcon));

  const label = document.createElement('span');
  label.className = 'desktop-icon__label';
  label.textContent = options.label;

  icon.append(pictogram, label);
  return icon;
}
