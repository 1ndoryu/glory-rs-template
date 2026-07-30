/* wandori.us — Desktop Shell
 * Shell reactivo del escritorio. Se suscribe a windowStore para renderizar ventanas.
 * Reemplaza desktop-concept.ts que tenía ventanas hardcodeadas.
 * Solo crea el contenedor del workspace; las ventanas se derivan del estado. */

import {
  createElement,
  FileUser,
  Folder,
  FolderCode,
  Gamepad2,
  PanelLeft,
  Settings,
  ShieldUser,
  X,
  type IconNode,
} from 'lucide';
import { createDesktopIcon } from './components/desktop-icon';
import { createDesktopMenuBar } from './components/desktop-menu-bar';
import { createDesktopWindow } from './components/desktop-window';
import { windowStore, focusWindow, restoreWindow, closeWindow, minimizeWindow, setWorkspaceBounds, registerShellWindow } from '../runtime/window-manager';
import { openAppWindow } from '../runtime/route-app-adapter';
import { navigate } from '../../router';
import { authStore, showSidebar } from '../../store';
import { dispatchEvent } from '../analytics/dispatcher';
import { enableDragResize } from './utils/drag-resize';
import { openContextMenu } from './components/desktop-context-menu';
import { selectSingle, clearSelection, selectBackground } from '../runtime/selection-store';

export interface DesktopShell {
  element: HTMLElement;
  contentWindow: HTMLElement;
  /** Controlar visibilidad del profile via windowStore. */
  setProfileVisible(visible: boolean): void;
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
    { id: 'profile', label: 'Perfil', type: 'document', icon: FileUser },
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

function createIconGrid(showAdmin: boolean, extraActions?: Record<string, () => void>): HTMLElement {
  const grid = document.createElement('div');
  grid.className = 'desktop-icon-grid';
  grid.setAttribute('aria-label', 'Objetos del escritorio');

  for (const item of getDesktopIcons(showAdmin)) {
    const onActivate = extraActions?.[item.id]
      ?? (item.appId ? () => { void openAppWindow(item.appId!); } : item.action);
    if (!onActivate) continue;

    const iconEl = createDesktopIcon({
      label: item.label,
      type: item.type,
      selected: item.selected,
      lucideIcon: item.icon,
      onActivate,
    });

    /* [Plan §3] Selección: clic selecciona el icono (solo clic simple) */
    iconEl.addEventListener('mousedown', (e) => {
      if (e.button === 0 && e.detail === 1) {
        selectSingle(item.id);
      }
    });

    /* [Plan §3.2] Context menu: clic derecho abre menú contextual del icono */
    iconEl.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      selectSingle(item.id);
      openContextMenu({
        context: 'icon',
        targets: [{ id: item.appId ?? item.id, kind: item.appId ? 'app' : 'shortcut' }],
        capability: showAdmin ? 'admin' : 'public',
        x: e.clientX,
        y: e.clientY,
      });
    });

    grid.appendChild(iconEl);
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

  /* Perfil: registrar como shell window en windowStore (no sistema paralelo) */
  const profileInstanceId = 'shell-profile';
  registerShellWindow({
    instanceId: profileInstanceId,
    title: 'Perfil',
    icon: FileUser,
    content: profile,
    initialBounds: { x: 44, y: 42, w: 470, h: 360 },
    focused: true,
    cssClass: 'desktop-profile-window',
    layout: 'full-bleed',
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
  const iconGrid = createIconGrid(authStore.get().isAuthenticated, {
    profile: () => {
      const existing = windowStore.get().find(w => w.instanceId === profileInstanceId);
      if (existing) {
        if (existing.state === 'minimized') restoreWindow(profileInstanceId);
        focusWindow(profileInstanceId);
      }
    },
  });
  workspace.append(iconGrid, contentWindow);

  /* [Plan §3] Context menu en workspace vacío */
  workspace.addEventListener('contextmenu', (e) => {
    /* Solo si el clic fue en el workspace mismo, no en un icono/ventana */
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

  /* [Plan §3] Clic en vacío limpia selección */
  workspace.addEventListener('mousedown', (e) => {
    if (e.button === 0 && (e.target === workspace || e.target === iconGrid)) {
      clearSelection();
    }
  });

  /* Taskbar reactivo — UNA sola fuente: windowStore (incluye perfil) */
  const { element: taskbar } = createReactiveTaskbar();

  shell.append(createDesktopMenuBar(), workspace, taskbar);

  /* Contenedor de ventanas de apps — llena todo el workspace con position absolute */
  const windowContainer = document.createElement('div');
  windowContainer.className = 'desktop-windows-container';
  windowContainer.style.position = 'absolute';
  windowContainer.style.inset = '0';
  windowContainer.style.pointerEvents = 'none';
  workspace.appendChild(windowContainer);

  /* Notify window-manager of workspace dimensions for boundary clamping.
   * ResizeObserver fires immediately once the element is in the DOM, so no sync call needed. */
  new ResizeObserver(() => {
    setWorkspaceBounds(windowContainer.clientWidth, windowContainer.clientHeight);
  }).observe(windowContainer);

  const renderedWindows = new Map<string, { el: HTMLElement; cleanup: () => void }>();

  windowStore.subscribe((windows) => {
    /* Añadir nuevas ventanas */
    for (const win of windows) {
      if (!renderedWindows.has(win.instanceId)) {
        const el = createDesktopWindow({
          title: win.title,
          content: win.content,
          className: win.cssClass ?? `desktop-window--${win.appId}`,
          layout: win.layout,
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
        });

        /* Posición inicial */
        el.style.position = 'absolute';
        el.style.left = `${win.bounds.x}px`;
        el.style.top = `${win.bounds.y}px`;
        el.style.width = `${win.bounds.w}px`;
        el.style.height = `${win.bounds.h}px`;
        el.style.pointerEvents = 'auto';

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

/**
 * Taskbar reactivo que se suscribe a windowStore.
 * Renderiza una tarea por ventana abierta.
 */
function createReactiveTaskbar(): { element: HTMLElement; taskList: HTMLElement } {
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

  /* Suscribirse a windowStore */
  windowStore.subscribe((windows) => {
    /* Limpiar todas las entradas de ventanas */
    taskList.innerHTML = '';

    for (const win of windows) {
      const item = document.createElement('button');
      item.type = 'button';
      item.className = `desktop-taskbar__task desktop-taskbar__task--${win.state === 'open' && win.focused ? 'active' : win.state === 'minimized' ? 'minimized' : ''}`;

      const icon = document.createElement('span');
      icon.className = 'desktop-taskbar__icon';
      /* Renderizar el icono Lucide de la app */
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

      /* Click: enfocar, restaurar o cerrar */
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
