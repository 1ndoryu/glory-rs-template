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
    taskList.innerHTML = '';

    for (const win of windows) {
      const item = document.createElement('button');
      item.type = 'button';
      item.className = `desktop-taskbar__task desktop-taskbar__task--${win.state === 'open' && win.focused ? 'active' : win.state === 'minimized' ? 'minimized' : ''}`;

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
        if ((e.target as HTMLElement).closest('.desktop-taskbar__close')) {
          closeWindow(win.instanceId);
          dispatchEvent({ type: 'app_closed', appId: win.appId });
          return;
        }
        if (win.state === 'minimized') {
          restoreWindow(win.instanceId);
        } else {
          focusWindow(win.instanceId);
        }
      });

      taskList.appendChild(item);
    }
  });

  return { element: taskbar, taskList };
}
