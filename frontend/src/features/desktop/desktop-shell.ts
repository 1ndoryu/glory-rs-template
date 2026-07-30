/* wandori.us — Desktop Shell
 * Shell reactivo del escritorio. Orquesta: menu bar, workspace, ventanas y taskbar.
 * Icon grid y taskbar están extraídos en módulos separados. */

import {
  FileUser,
} from 'lucide';
import { createDesktopMenuBar } from './components/desktop-menu-bar';
import { createDesktopWindow } from './components/desktop-window';
import { windowStore, focusWindow, restoreWindow, closeWindow, minimizeWindow, toggleMaximizeWindow, setWorkspaceBounds, registerShellWindow } from '../runtime/window-manager';
import { authStore } from '../../store';
import { dispatchEvent } from '../analytics/dispatcher';
import { enableDragResize } from './utils/drag-resize';
import { openContextMenu } from './components/desktop-context-menu';
import { selectBackground } from '../runtime/selection-store';
import { createWorkspaceIconGrid } from './workspace-icon-grid';
import { createReactiveTaskbar } from './reactive-taskbar';

export interface DesktopShell {
  element: HTMLElement;
  contentWindow: HTMLElement;
  setProfileVisible(visible: boolean): void;
}

export function createDesktopShell(
  profile: HTMLElement,
  content: HTMLElement,
): DesktopShell {
  const shell = document.createElement('section');
  shell.className = 'desktop-shell';
  shell.setAttribute('aria-label', 'Escritorio');

  const workspace = document.createElement('div');
  workspace.className = 'desktop-workspace';

  /* Profile como shell window */
  const profileInstanceId = 'shell-profile';
  registerShellWindow({
    instanceId: profileInstanceId,
    title: 'Perfil',
    icon: FileUser,
    content: profile,
    initialBounds: { x: 44, y: 42, w: 470, h: 264 },
    focused: true,
    cssClass: 'desktop-profile-window',
    layout: 'full-bleed',
  });

  /* Ventana legacy para páginas no-app */
  const contentWindow = createDesktopWindow({
    title: 'Documento',
    content,
    className: 'desktop-content-window',
    resizable: true,
  });
  contentWindow.style.display = 'none';

  /* Icon grid reactivo */
  const iconGrid = createWorkspaceIconGrid({
    profile: () => {
      const existing = windowStore.get().find(w => w.instanceId === profileInstanceId);
      if (existing) {
        if (existing.state === 'minimized') restoreWindow(profileInstanceId);
        focusWindow(profileInstanceId);
      } else {
        registerShellWindow({
          instanceId: profileInstanceId,
          title: 'Perfil',
          icon: FileUser,
          content: profile,
          initialBounds: { x: 44, y: 42, w: 470, h: 224 },
          focused: true,
          cssClass: 'desktop-profile-window',
          layout: 'full-bleed',
        });
      }
    },
  });
  workspace.append(iconGrid, contentWindow);

  /* Context menu en workspace vacío */
  workspace.addEventListener('contextmenu', (e) => {
    if (e.target !== workspace && e.target !== iconGrid) return;
    e.preventDefault();
    selectBackground();
    openContextMenu({
      context: 'desktop',
      capability: authStore.get().isAuthenticated ? 'admin' : 'public',
      x: e.clientX,
      y: e.clientY,
    });
  });

  /* Clic en vacío: clearSelection se maneja dentro de workspace-icon-grid */

  /* Taskbar reactivo */
  const { element: taskbar } = createReactiveTaskbar();

  shell.append(createDesktopMenuBar(), workspace, taskbar);

  /* Window container */
  const windowContainer = document.createElement('div');
  windowContainer.className = 'desktop-windows-container';
  windowContainer.style.position = 'absolute';
  windowContainer.style.inset = '0';
  windowContainer.style.pointerEvents = 'none';
  workspace.appendChild(windowContainer);

  new ResizeObserver(() => {
    setWorkspaceBounds(windowContainer.clientWidth, windowContainer.clientHeight);
  }).observe(windowContainer);

  const renderedWindows = new Map<string, { el: HTMLElement; cleanup: () => void }>();

  windowStore.subscribe((windows) => {
    for (const win of windows) {
      if (!renderedWindows.has(win.instanceId)) {
        const el = createDesktopWindow({
          title: win.title,
          content: win.content,
          className: win.cssClass ?? `desktop-window--${win.appId}`,
          layout: win.layout,
          toolbar: win.toolbar,
          active: win.focused,
          resizable: true,
          onClose: () => {
            win.controller?.abort();
            if (win.app) dispatchEvent({ type: 'app_closed', appId: win.appId });
            closeWindow(win.instanceId);
          },
          onMinimize: () => {
            minimizeWindow(win.instanceId);
          },
          onMaximize: () => {
            toggleMaximizeWindow(win.instanceId);
          },
        });

        el.style.position = 'absolute';
        el.style.left = `${win.bounds.x}px`;
        el.style.top = `${win.bounds.y}px`;
        el.style.width = `${win.bounds.w}px`;
        el.style.height = `${win.bounds.h}px`;
        el.style.pointerEvents = 'auto';

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

    for (const [id, entry] of renderedWindows) {
      if (!windows.find(w => w.instanceId === id)) {
        entry.cleanup();
        entry.el.remove();
        renderedWindows.delete(id);
      }
    }

    for (const win of windows) {
      const entry = renderedWindows.get(win.instanceId);
      if (!entry) continue;
      const el = entry.el;

      el.style.display = win.state === 'minimized' ? 'none' : '';
      el.style.zIndex = String(win.zIndex);
      el.classList.toggle('desktop-window--active', win.focused);
      el.style.left = `${win.bounds.x}px`;
      el.style.top = `${win.bounds.y}px`;
      el.style.width = `${win.bounds.w}px`;
      el.style.height = `${win.bounds.h}px`;
    }
  });

  function setProfileVisible(visible: boolean): void {
    const win = windowStore.get().find(w => w.instanceId === profileInstanceId);
    if (!win) return;
    if (visible) {
      if (win.state === 'minimized') restoreWindow(profileInstanceId);
      focusWindow(profileInstanceId);
    } else {
      minimizeWindow(profileInstanceId);
    }
  }

  return { element: shell, contentWindow, setProfileVisible };
}
