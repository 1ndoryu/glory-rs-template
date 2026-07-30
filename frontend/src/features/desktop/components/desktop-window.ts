import { createElement, Maximize2, Minus, X, type IconNode } from 'lucide';
import { createEl } from '../../../utils/dom';
import type { AppToolbarGroup, ToolbarItemRef } from '../../runtime/app-registry';
import { CommandRegistry, type CommandContext } from '../../runtime/command-registry';
import { authStore } from '../../../store';
import { openDropdownMenu, type DropdownMenuItem } from './dropdown-menu';

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
  const control = createEl('button', { type: 'button', className: `desktop-window__control ${className}`, ariaLabel: label },
    createElement(icon),
  );
  control.disabled = !onActivate;

  if (onActivate) control.addEventListener('click', onActivate);
  return control;
}

/* [297A-2] Receta visual única para todas las futuras aplicaciones.
 * Los controles son decorativos hasta que exista el gestor de ventanas. */
export function createDesktopWindow(options: DesktopWindowOptions): HTMLElement {
  const windowElement = createEl('section', { className: 'desktop-window', ariaLabel: `Ventana ${options.title}` });

  if (options.className) windowElement.classList.add(...options.className.split(' '));
  if (options.active) windowElement.classList.add('desktop-window--active');
  if (options.resizable) windowElement.classList.add('desktop-window--resizable');

  const closeControl = createWindowControl(
    X, 'desktop-window__control--close', `Cerrar ${options.title}`, options.onClose,
  );

  const minimizeControl = createWindowControl(
    Minus, 'desktop-window__control--minimize', `Minimizar ${options.title}`, options.onMinimize,
  );

  const titleBar = createEl('header', { className: 'desktop-window__titlebar' },
    closeControl,
    createEl('span', { className: 'desktop-window__title', textContent: options.title }),
    minimizeControl,
  );

  windowElement.append(titleBar);

  /* App toolbar */
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
  windowElement.appendChild(createAppToolbar(allGroups, {
    onClose: options.onClose, onMinimize: options.onMinimize, onMaximize: options.onMaximize,
  }));

  const body = createEl('div', { className: 'desktop-window__body' });
  if (options.layout !== 'full-bleed') {
    body.classList.add('desktop-window__body--padded');
  }
  body.appendChild(options.content);
  windowElement.appendChild(body);

  return windowElement;
}

/* === App Toolbar === */

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

export function createAppToolbar(
  groups: AppToolbarGroup[],
  callbacks?: { onClose?: () => void; onMinimize?: () => void; onMaximize?: () => void },
): HTMLElement {
  const toolbar = createEl('div', { className: 'desktop-app-toolbar' });

  const ctx: CommandContext = {
    capability: authStore.get().isAuthenticated ? 'admin' : 'public',
    presentationMode: 'desktop',
  };

  const windowCallbackMap: Record<string, 'onClose' | 'onMinimize' | 'onMaximize'> = {
    'window:close': 'onClose',
    'window:minimize': 'onMinimize',
    'window:maximize': 'onMaximize',
  };

  for (const group of groups) {
    const btn = createEl('button', { type: 'button', className: 'desktop-app-toolbar__item', textContent: group.label, ariaHaspopup: 'menu' });

    const entry = createEl('div', { className: 'desktop-app-toolbar__entry' }, btn);

    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      btn.setAttribute('aria-expanded', 'true');

      const items: DropdownMenuItem[] = [];
      for (const ref of group.items) {
        if (ref === '---' || (typeof ref === 'object' && ref.id === '---')) {
          items.push({ label: '', separator: true });
          continue;
        }

        const resolved = resolveToolbarItem(ref);
        if (!resolved) continue;

        const cmd = CommandRegistry.get(resolved.id);
        type Avail = { state: 'enabled' } | { state: 'disabled'; reason: string } | { state: 'hidden' };
        let availability: Avail = { state: 'enabled' };
        if (cmd?.isAvailable) {
          availability = cmd.isAvailable(ctx);
          if (availability.state === 'hidden') continue;
        }

        const callbackKey = windowCallbackMap[resolved.id];
        const isWindowCmd = !!callbackKey;
        const callbackDisabled = isWindowCmd && !callbacks?.[callbackKey];
        const disabled = availability.state !== 'enabled' || callbackDisabled;

        items.push({
          icon: resolved.icon,
          label: resolved.label,
          shortcut: resolved.shortcut,
          disabled,
          onClick: disabled ? undefined : () => {
            if (resolved.id === 'window:minimize') { callbacks?.onMinimize?.(); return; }
            if (resolved.id === 'window:maximize') { callbacks?.onMaximize?.(); return; }
            if (resolved.id === 'window:close') { callbacks?.onClose?.(); return; }
            void CommandRegistry.execute(resolved.id, ctx);
          },
        });
      }

      const rect = btn.getBoundingClientRect();
      openDropdownMenu({
        items,
        positioning: 'fixed',
        x: rect.left,
        y: rect.bottom,
        ariaLabel: group.label,
        onClose: () => { btn.setAttribute('aria-expanded', 'false'); },
      });
    });

    toolbar.appendChild(entry);
  }

  return toolbar;
}
