/* wandori.us — Reactive Taskbar
 * Taskbar del OS suscrito a windowStore. Extraído de desktop-shell.ts. */

import {
  createElement,
  FileUser,
  PanelLeft,
  X,
} from 'lucide';
import { windowStore, closeWindow, restoreWindow, focusWindow } from '../runtime/window-manager';
import { showSidebar } from '../../store';
import { dispatchEvent } from '../analytics/dispatcher';
import { reconcileChildren } from '../../utils/reconcile';

export function createReactiveTaskbar(): { element: HTMLElement; taskList: HTMLElement } {
  const taskbar = document.createElement('footer');
  taskbar.className = 'desktop-taskbar';
  taskbar.setAttribute('aria-label', 'Ventanas abiertas');

  const navControl = document.createElement('button');
  navControl.type = 'button';
  navControl.className = 'desktop-taskbar__nav-control';
  navControl.setAttribute('aria-label', showSidebar.get() ? 'Ocultar navegación' : 'Mostrar navegación');
  const navIcon = createElement(PanelLeft);
  navIcon.classList.add('desktop-taskbar__icon');
  navControl.appendChild(navIcon);
  navControl.addEventListener('click', () => {
    showSidebar.update(v => !v);
    navControl.setAttribute('aria-label', showSidebar.get() ? 'Ocultar navegación' : 'Mostrar navegación');
  });

  const taskList = document.createElement('div');
  taskList.className = 'desktop-taskbar__tasks';

  taskbar.append(navControl, taskList);

  windowStore.subscribe((windows) => {
    reconcileChildren(
      taskList,
      windows,
      (win) => win.instanceId,
      /* createElement */
      (win) => {
        const item = document.createElement('button');
        item.type = 'button';

        const icon = document.createElement('span');
        icon.className = 'desktop-taskbar__icon';
        const iconSvg = createElement(win.icon ?? win.app?.icon ?? FileUser);
        iconSvg.classList.add('desktop-taskbar__icon');
        icon.appendChild(iconSvg);

        const label = document.createElement('span');
        label.className = 'desktop-taskbar__label';
        label.textContent = win.title;

        const closeBtn = document.createElement('button');
        closeBtn.type = 'button';
        closeBtn.className = 'desktop-taskbar__close';
        closeBtn.setAttribute('aria-label', `Cerrar ${win.title}`);
        closeBtn.appendChild(createElement(X));

        item.append(icon, label, closeBtn);

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
      /* updateElement */
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
