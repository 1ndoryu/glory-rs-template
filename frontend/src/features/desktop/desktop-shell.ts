/* wandori.us — Desktop Shell
 * Shell reactivo del escritorio. Se suscribe a windowStore para renderizar ventanas.
 * Reemplaza desktop-concept.ts que tenía ventanas hardcodeadas.
 * Solo crea el contenedor del workspace; las ventanas se derivan del estado. */

import {
  FileUser,
  Folder,
  FolderCode,
  Gamepad2,
  Settings,
  ShieldUser,
  type IconNode,
} from 'lucide';
import { createDesktopIcon } from './components/desktop-icon';
import { createDesktopMenuBar } from './components/desktop-menu-bar';
import { createDesktopWindow } from './components/desktop-window';
import { windowStore, focusWindow } from '../runtime/window-manager';
import { openAppWindow } from '../runtime/route-app-adapter';
import { navigate } from '../../router';
import { authStore } from '../../store';
import { dispatchEvent } from '../analytics/dispatcher';
import { enableDragResize } from './utils/drag-resize';

export interface DesktopShell {
  element: HTMLElement;
  profileWindow: HTMLElement;
  contentWindow: HTMLElement;
}

interface DesktopIconItem {
  id: string;
  label: string;
  type: 'folder' | 'document' | 'application';
  icon: IconNode;
  selected?: boolean;
  appId?: string;
  action?: () => void;
}

function getDesktopIcons(showAdmin: boolean): DesktopIconItem[] {
  const items: DesktopIconItem[] = [
    { id: 'gallery', label: 'Galería', type: 'folder', icon: Folder, selected: true, appId: 'finder' },
    { id: 'projects', label: 'Proyectos', type: 'folder', icon: FolderCode, appId: 'projects' },
    { id: 'about', label: 'About', type: 'document', icon: FileUser, appId: 'about' },
    { id: 'snake', label: 'Snake', type: 'application', icon: Gamepad2 },
  ];

  if (showAdmin) {
    items.push(
      { id: 'settings', label: 'Configuración', type: 'application', icon: Settings, appId: 'settings' },
      { id: 'admin', label: 'Admin', type: 'application', icon: ShieldUser, action: () => navigate('/admin') },
    );
  }

  return items;
}

function createIconGrid(showAdmin: boolean): HTMLElement {
  const grid = document.createElement('div');
  grid.className = 'desktop-icon-grid';
  grid.setAttribute('aria-label', 'Objetos del escritorio');

  for (const item of getDesktopIcons(showAdmin)) {
    const onActivate = item.appId
      ? () => { void openAppWindow(item.appId!); }
      : item.action;

    grid.appendChild(createDesktopIcon({
      label: item.label,
      type: item.type,
      selected: item.selected,
      lucideIcon: item.icon,
      onActivate,
    }));
  }

  return grid;
}

/**
 * Crea el shell del escritorio reactivo.
 * El workspace se suscribe a windowStore para renderizar ventanas.
 */
export function createDesktopShell(
  profile: HTMLElement,
  content: HTMLElement,
): DesktopShell {
  const shell = document.createElement('section');
  shell.className = 'desktop-shell';
  shell.setAttribute('aria-label', 'Escritorio');

  const workspace = document.createElement('div');
  workspace.className = 'desktop-workspace';

  /* Ventana de perfil (siempre presente, no gestionada por WindowManager) */
  const profileWindow = createDesktopWindow({
    title: 'Perfil',
    content: profile,
    className: 'desktop-profile-window',
    active: true,
    resizable: true,
  });

  /* Ventana de contenido (para páginas legacy) */
  const contentWindow = createDesktopWindow({
    title: 'Documento',
    content,
    className: 'desktop-content-window',
    resizable: true,
  });
  contentWindow.style.display = 'none';

  /* Icon grid */
  const iconGrid = createIconGrid(authStore.get().isAuthenticated);
  workspace.append(iconGrid, profileWindow, contentWindow);

  /* Taskbar reactivo */
  const taskbar = createReactiveTaskbar();

  shell.append(createDesktopMenuBar(), workspace, taskbar);

  /* Suscribirse a windowStore para renderizar ventanas de apps */
  const windowContainer = document.createElement('div');
  windowContainer.className = 'desktop-windows-container';
  windowContainer.style.position = 'relative';
  workspace.appendChild(windowContainer);

  const renderedWindows = new Map<string, { el: HTMLElement; cleanup: () => void }>();

  windowStore.subscribe((windows) => {
    /* Añadir nuevas ventanas */
    for (const win of windows) {
      if (!renderedWindows.has(win.instanceId)) {
        const el = createDesktopWindow({
          title: win.title,
          content: win.content,
          className: `desktop-app-window desktop-app-window--${win.appId}`,
          active: win.focused,
          resizable: true,
          onClose: () => {
            win.controller.abort();
            dispatchEvent({ type: 'app_closed', appId: win.appId });
            import('../runtime/window-manager').then(m => m.closeWindow(win.instanceId));
          },
          onMinimize: () => {
            import('../runtime/window-manager').then(m => m.minimizeWindow(win.instanceId));
          },
        });

        /* Posición inicial */
        el.style.position = 'absolute';
        el.style.left = `${win.bounds.x}px`;
        el.style.top = `${win.bounds.y}px`;
        el.style.width = `${win.bounds.w}px`;
        el.style.height = `${win.bounds.h}px`;

        /* Activar drag/resize */
        const titleBar = el.querySelector('.desktop-window__titlebar') as HTMLElement;
        let cleanup = () => {};
        if (titleBar) {
          cleanup = enableDragResize({
            windowEl: el,
            instanceId: win.instanceId,
            dragHandle: titleBar,
            resizable: true,
          });
        }

        el.addEventListener('mousedown', () => {
          focusWindow(win.instanceId);
        });
        windowContainer.appendChild(el);
        renderedWindows.set(win.instanceId, { el, cleanup });
      }
    }

    /* Eliminar ventanas cerradas */
    for (const [id, entry] of renderedWindows) {
      if (!windows.find(w => w.instanceId === id)) {
        entry.cleanup();
        entry.el.remove();
        renderedWindows.delete(id);
      }
    }

    /* Actualizar estado visual de ventanas existentes */
    for (const win of windows) {
      const entry = renderedWindows.get(win.instanceId);
      if (!entry) continue;
      const el = entry.el;

      /* Mostrar/ocultar según estado */
      el.style.display = win.state === 'minimized' ? 'none' : '';
      el.style.zIndex = String(win.zIndex);

      /* Clase activa */
      el.classList.toggle('desktop-window--active', win.focused);

      /* Actualizar posición y tamaño */
      el.style.left = `${win.bounds.x}px`;
      el.style.top = `${win.bounds.y}px`;
      el.style.width = `${win.bounds.w}px`;
      el.style.height = `${win.bounds.h}px`;
    }
  });

  return { element: shell, profileWindow, contentWindow };
}

/**
 * Taskbar reactivo que se suscribe a windowStore.
 * Renderiza una tarea por ventana abierta.
 */
function createReactiveTaskbar(): HTMLElement {
  const taskbar = document.createElement('footer');
  taskbar.className = 'desktop-taskbar';
  taskbar.setAttribute('aria-label', 'Ventanas abiertas');

  const navControl = document.createElement('button');
  navControl.type = 'button';
  navControl.className = 'desktop-taskbar__nav-control';
  navControl.setAttribute('aria-label', 'Mostrar navegación');

  const taskList = document.createElement('div');
  taskList.className = 'desktop-taskbar__tasks';

  taskbar.append(navControl, taskList);

  /* Suscribirse a windowStore */
  windowStore.subscribe((windows) => {
    taskList.innerHTML = '';

    for (const win of windows) {
      const item = document.createElement('button');
      item.type = 'button';
      item.className = `desktop-taskbar__task desktop-taskbar__task--${win.state === 'open' && win.focused ? 'active' : win.state === 'minimized' ? 'minimized' : ''}`;

      const icon = document.createElement('span');
      icon.className = 'desktop-taskbar__icon';

      const label = document.createElement('span');
      label.className = 'desktop-taskbar__label';
      label.textContent = win.title;

      const closeBtn = document.createElement('button');
      closeBtn.type = 'button';
      closeBtn.className = 'desktop-taskbar__close';
      closeBtn.setAttribute('aria-label', `Cerrar ${win.title}`);

      item.append(icon, label, closeBtn);

      /* Click: enfocar o restaurar */
      item.addEventListener('click', (e) => {
        if (e.target === closeBtn) {
          import('../runtime/window-manager').then(m => {
            m.closeWindow(win.instanceId);
            dispatchEvent({ type: 'app_closed', appId: win.appId });
          });
          return;
        }
        if (win.state === 'minimized') {
          import('../runtime/window-manager').then(m => m.restoreWindow(win.instanceId));
        } else {
          focusWindow(win.instanceId);
        }
      });

      taskList.appendChild(item);
    }
  });

  return taskbar;
}
