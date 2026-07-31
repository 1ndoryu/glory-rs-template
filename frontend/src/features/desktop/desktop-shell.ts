/* wandori.us — Desktop Shell
 * Shell reactivo del escritorio. Orquesta: menu bar, workspace, ventanas y taskbar.
 * Icon grid y taskbar están extraídos en módulos separados. */

import { FileUser } from 'lucide';
import { createEl } from '../../utils/dom';
import { createDesktopMenuBar } from './components/desktop-menu-bar';
import { createDesktopWindow } from './components/desktop-window';
import {
  windowStore, focusWindow, restoreWindow, closeWindow,
  minimizeWindow, toggleMaximizeWindow, setWorkspaceBounds, registerShellWindow,
} from '../runtime/window-manager';
import { authStore } from '../../store';
import { enableDragResize } from './utils/drag-resize';
import { openContextMenu } from './components/desktop-context-menu';
import { selectBackground } from '../runtime/selection-store';
import { makeDropTarget, onGlobalDrop } from './utils/icon-drag';
import { moveNodeToParent } from '../runtime/workspace/workspace-store';
import { createWorkspaceIconGrid } from './workspace-icon-grid';
import { createReactiveTaskbar } from './reactive-taskbar';

export interface DesktopShell {
  element: HTMLElement;
  contentWindow: HTMLElement;
  setProfileVisible(visible: boolean): void;
  destroy(): void;
}

export function createDesktopShell(
  profile: HTMLElement,
  content: HTMLElement,
): DesktopShell {
  let destroyed = false;
  const shell = createEl('section', { className: 'desktop-shell', ariaLabel: 'Escritorio' });
  const workspace = createEl('div', { className: 'desktop-workspace' });

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
  makeDropTarget({ el: workspace, dropId: 'desktop', context: 'desktop' });

  workspace.append(iconGrid.element, contentWindow);

  const onWorkspaceContextMenu = (e: MouseEvent): void => {
    if (e.target !== workspace && e.target !== iconGrid.element) return;
    e.preventDefault();
    selectBackground();
    openContextMenu({
      context: 'desktop',
      capability: authStore.get().capability,
      x: e.clientX,
      y: e.clientY,
    });
  };
  workspace.addEventListener('contextmenu', onWorkspaceContextMenu);

  const taskbar = createReactiveTaskbar();

  const stopGlobalDrop = onGlobalDrop((result) => {
    if (result.sourceId === result.targetId) return;
    moveNodeToParent(result.sourceId, result.targetId);
  });

  const menuBar = createDesktopMenuBar();
  shell.append(menuBar.element, workspace, taskbar.element);

  /* Window container */
  const windowContainer = createEl('div', { className: 'desktop-windows-container' });
  windowContainer.style.position = 'absolute';
  windowContainer.style.inset = '0';
  windowContainer.style.pointerEvents = 'none';
  workspace.appendChild(windowContainer);

  const resizeObserver = new ResizeObserver(() => {
    setWorkspaceBounds(windowContainer.clientWidth, windowContainer.clientHeight);
  });
  resizeObserver.observe(windowContainer);

  const renderedWindows = new Map<string, { el: HTMLElement; cleanup: () => void }>();

  const stopWindows = windowStore.subscribe((windows) => {
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
            /* closeWindow es el único dueño del teardown y MountedView.destroy.
             * Así app_closed no se emite dos veces desde shell y app. */
            closeWindow(win.instanceId);
          },
          onMinimize: () => { minimizeWindow(win.instanceId); },
          onMaximize: () => { toggleMaximizeWindow(win.instanceId); },
        });

        el.style.position = 'absolute';
        el.style.setProperty('--win-x', `${win.bounds.x}px`);
        el.style.setProperty('--win-y', `${win.bounds.y}px`);
        el.style.setProperty('--win-w', `${win.bounds.w}px`);
        el.style.setProperty('--win-h', `${win.bounds.h}px`);
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

        el.addEventListener('mousedown', () => { focusWindow(win.instanceId); });
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
      el.style.setProperty('--win-x', `${win.bounds.x}px`);
      el.style.setProperty('--win-y', `${win.bounds.y}px`);
      el.style.setProperty('--win-w', `${win.bounds.w}px`);
      el.style.setProperty('--win-h', `${win.bounds.h}px`);
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

  function destroy(): void {
    if (destroyed) return;
    destroyed = true;
    stopWindows();
    stopGlobalDrop();
    iconGrid.destroy();
    taskbar.destroy();
    menuBar.destroy();
    resizeObserver.disconnect();
    workspace.removeEventListener('contextmenu', onWorkspaceContextMenu);
    for (const entry of renderedWindows.values()) {
      entry.cleanup();
      entry.el.remove();
    }
    renderedWindows.clear();
    shell.remove();
  }

  return { element: shell, contentWindow, setProfileVisible, destroy };
}
