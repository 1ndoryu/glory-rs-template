/* wandori.us — Reactive Taskbar
 * Taskbar del OS suscrito a windowStore. Extraído de desktop-shell.ts. */

import {
  createElement,
  FileUser,
  PanelLeft,
  X,
} from 'lucide';
import { createEl } from '../../utils/dom';
import type { WindowIdentity, WindowContent } from '../runtime/window-store';
import { windowStore, closeWindow, restoreWindow, focusWindow } from '../runtime/window-manager';
import { showSidebar } from '../../store';
import { dispatchEvent } from '../analytics/dispatcher';
import { reconcileChildren } from '../../utils/reconcile';

export function createReactiveTaskbar(): { element: HTMLElement; taskList: HTMLElement } {
  const taskbar = createEl('footer', { className: 'desktop-taskbar', ariaLabel: 'Ventanas abiertas' });

  const navControl = createEl('button', { type: 'button', className: 'desktop-taskbar__nav-control' });
  navControl.setAttribute('aria-label', showSidebar.get() ? 'Ocultar navegación' : 'Mostrar navegación');
  const navIcon = createElement(PanelLeft);
  navIcon.classList.add('desktop-taskbar__icon');
  navControl.appendChild(navIcon);
  navControl.addEventListener('click', () => {
    showSidebar.update(v => !v);
    navControl.setAttribute('aria-label', showSidebar.get() ? 'Ocultar navegación' : 'Mostrar navegación');
  });

  const taskList = createEl('div', { className: 'desktop-taskbar__tasks' });

  taskbar.append(navControl, taskList);

  type TaskbarWin = WindowIdentity & Pick<WindowContent, 'icon' | 'app'>;
  windowStore.subscribe((windows: readonly TaskbarWin[]) => {
    reconcileChildren(
      taskList,
      windows,
      (win) => win.instanceId,
      (win) => {
        const svgIcon = createElement(win.icon ?? win.app?.icon ?? FileUser);
        svgIcon.classList.add('desktop-taskbar__icon');

        const item = createEl('button', { type: 'button' },
          svgIcon,
          createEl('span', { className: 'desktop-taskbar__label', textContent: win.title }),
          createEl('button', { type: 'button', className: 'desktop-taskbar__close', ariaLabel: `Cerrar ${win.title}` },
            createElement(X),
          ),
        );

        item.addEventListener('click', (e) => {
          const id = item.dataset.key;
          if (!id) return;
          if ((e.target as HTMLElement).closest('.desktop-taskbar__close')) {
            closeWindow(id);
            dispatchEvent({ type: 'app_closed', appId: id });
            return;
          }
          const win = windowStore.get().find(w => w.instanceId === id);
          if (win?.state === 'minimized') {
            restoreWindow(id);
          } else {
            focusWindow(id);
          }
        });

        return item;
      },
      (el, win) => {
        const activeClass = win.state === 'open' && win.focused ? 'active' : win.state === 'minimized' ? 'minimized' : '';
        el.className = `desktop-taskbar__task desktop-taskbar__task--${activeClass}`;
        const label = el.querySelector('.desktop-taskbar__label');
        if (label && label.textContent !== win.title) label.textContent = win.title;
        const closeBtn = el.querySelector('.desktop-taskbar__close');
        if (closeBtn) closeBtn.setAttribute('aria-label', `Cerrar ${win.title}`);
      },
    );
  });

  return { element: taskbar, taskList };
}
