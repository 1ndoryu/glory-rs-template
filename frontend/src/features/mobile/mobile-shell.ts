/* wandori.us — Mobile Shell
 * Presentación móvil del mismo runtime del OS: launcher + vista full-screen.
 * Las apps entregan el mismo MountedView que desktop; este módulo solo aporta
 * navegación y chrome móvil. [297A-12 §2–4] */

import { ArrowLeft, Circle, FileUser, createElement, type IconNode } from 'lucide';
import { createEl } from '../../utils/dom';
import { createThemeToggleButton, type ThemeToggleButton } from '../../components/ui/theme-toggle-button';
import { getCurrentPathname } from '../../utils/viewport';
import { getCanonicalAppPath, type AppOpenHistory } from '../runtime/deep-links';
import { openContextMenu } from '../desktop/components/desktop-context-menu';
import { bindLongPress } from './mobile-gestures';
import { authStore } from '../../store';
import { isInternalPushHistoryEntry, navigate, replacePath } from '../../router';
import type { MountedView } from '../../core/lifecycle';
import { AppRegistry } from '../runtime/app-registry';
import { resolveResourceType, type ResourceKind } from '../runtime/resource-type-registry';
import { workspaceStore } from '../runtime/workspace/workspace-store';
import type { ResolvedNode } from '../runtime/workspace/types';
import {
  clearMobileStack,
  getTopMobileApp,
  mobileStackStore,
  openMobileApp,
  openMobileView,
  popMobileApp,
  type MobileStackEntry,
} from './mobile-stack';

export interface MobileShell {
  readonly element: HTMLElement;
  readonly routerOutlet: HTMLElement;
  readonly setLegacyContentVisible: (visible: boolean) => void;
  readonly destroy: () => void;
  readonly openApp: (appId: string, params?: Readonly<Record<string, string>>, options?: { history?: AppOpenHistory }) => Promise<void>;
  readonly openProfile: () => Promise<void>;
  readonly goHome: () => void;
  readonly goBack: () => void;
}

function resolveNodeIcon(node: ResolvedNode): IconNode {
  if (node.id === 'profile' || node.refId === 'shell-profile') return FileUser;
  if (node.type === 'app' && node.refId) return AppRegistry.get(node.refId)?.icon ?? Circle;
  if (node.type === 'folder') return AppRegistry.get('finder')?.icon ?? Circle;
  if (node.type === 'resource' && node.resourceKind) {
    return AppRegistry.get(resolveResourceType(node.resourceKind as ResourceKind)?.appId ?? '')?.icon ?? Circle;
  }
  return Circle;
}

function resolveNodeAction(
  node: ResolvedNode,
  openApp: (appId: string, params?: Readonly<Record<string, string>>) => Promise<void>,
  openProfile: () => Promise<void>,
): (() => void) | undefined {
  if (node.id === 'profile' || node.refId === 'shell-profile') {
    return () => {
      void openProfile().catch(() => {
        /* El shell puede desmontarse durante el click; no reabrir vistas. */
      });
    };
  }
  if (node.type === 'folder') return () => { void openApp('finder', { folderId: node.id }); };
  if (node.type === 'resource' && node.resourceKind) {
    const appId = resolveResourceType(node.resourceKind as ResourceKind)?.appId ?? 'finder';
    return () => { void openApp(appId, { resourceId: node.refId ?? node.id }); };
  }
  if (node.refId && AppRegistry.get(node.refId)) return () => { void openApp(node.refId!); };
  return undefined;
}

function createIconButton(
  node: ResolvedNode,
  openApp: (appId: string, params?: Readonly<Record<string, string>>) => Promise<void>,
  openProfile: () => Promise<void>,
): HTMLButtonElement | null {
  const action = resolveNodeAction(node, openApp, openProfile);
  if (!action) return null;

  const button = createEl('button', {
    type: 'button',
    className: 'movilLauncher__app',
    ariaLabel: `Abrir ${node.label}`,
    role: 'listitem',
  });
  const pictogram = createEl('span', { className: 'movilLauncher__pictograma', ariaHidden: 'true' });
  const icon = createElement(resolveNodeIcon(node));
  icon.classList.add('movilLauncher__icono');
  pictogram.appendChild(icon);
  button.append(
    pictogram,
    createEl('span', { className: 'movilLauncher__etiqueta', textContent: node.label }),
  );
  button.addEventListener('click', action);
  return button;
}

function createNavigation(
  hasApp: boolean,
  goBack: () => void,
  goHome: () => void,
): HTMLElement {
  const navigation = createEl('nav', {
    className: 'movilNavegacion',
    ariaLabel: 'Navegación del sistema',
  });
  const back = createEl('button', {
    type: 'button',
    className: 'movilNavegacion__control',
    ariaLabel: 'Atrás',
  }, createElement(ArrowLeft));
  const home = createEl('button', {
    type: 'button',
    className: 'movilNavegacion__control',
    ariaLabel: 'Ir al inicio',
  }, createElement(Circle));
  back.disabled = !hasApp;
  back.addEventListener('click', goBack);
  home.addEventListener('click', goHome);
  navigation.append(back, home);
  return navigation;
}

function createAppHeader(title: string): HTMLElement {
  return createEl('header', { className: 'movilApp__cabecera' },
    createEl('span', { className: 'movilMarca', ariaHidden: 'true' }),
    createEl('h1', { className: 'movilApp__titulo', textContent: title }),
    createEl('span', { ariaHidden: 'true' }),
  );
}

export function createMobileShell(
  profile: HTMLElement,
  onToggleExternalNav: () => void,
): MobileShell {
  const shell = createEl('section', { className: 'movilOs', ariaLabel: 'Sistema móvil' });
  const viewport = createEl('div', { className: 'movilOs__viewport' });
  const routerOutlet = createEl('main', {
    className: 'movilOs__routerOutlet',
    ariaHidden: 'true',
  });
  let currentView: HTMLElement | null = null;
  let legacyContentVisible = false;
  let destroyed = false;
  const pendingControllers = new Set<AbortController>();
  const launcherGestureCleanups: Array<() => void> = [];
  /* [297A-18] Botón de tema del launcher; se destruye al re-renderizar launcher. */
  let launcherThemeToggle: ThemeToggleButton | null = null;

  const openApp = async (
    appId: string,
    params?: Readonly<Record<string, string>>,
    options: { history?: AppOpenHistory } = {},
  ): Promise<void> => {
    if (destroyed) return;
    await openMobileApp(appId, params, options);
  };

  const openProfile = async (): Promise<void> => {
    if (destroyed) return;
    const controller = new AbortController();
    pendingControllers.add(controller);
    const view: MountedView = { element: profile };
    try {
      await openMobileView('profile', 'Perfil', view, undefined, true, 'full-bleed', controller);
    } finally {
      pendingControllers.delete(controller);
    }
  };

  const resolveTargetKind = (node: ResolvedNode): 'app' | 'folder' | 'resource' | 'shortcut' => {
    if (node.type === 'app') return 'app';
    if (node.type === 'folder') return 'folder';
    if (node.type === 'resource') return 'resource';
    return 'shortcut';
  };

  const openLauncherMenu = (node: ResolvedNode, event: PointerEvent): void => {
    openContextMenu({
      context: 'icon',
      targets: [{ id: node.refId ?? node.id, kind: resolveTargetKind(node) }],
      capability: authStore.get().capability,
      presentationMode: 'mobile',
      className: 'desktop-context-menu--mobile',
      x: event.clientX,
      y: event.clientY,
    });
  };

  const clearLauncherGestures = (): void => {
    for (const cleanup of launcherGestureCleanups.splice(0)) cleanup();
  };

  const clearLauncherResources = (): void => {
    clearLauncherGestures();
    launcherThemeToggle?.destroy();
    launcherThemeToggle = null;
  };

  const renderLauncher = (): HTMLElement => {
    clearLauncherResources();
    const launcher = createEl('div', { className: 'movilLauncher' });
    const themeToggle = createThemeToggleButton('movilLauncher__tema');
    launcherThemeToggle = themeToggle;
    const header = createEl('header', { className: 'movilLauncher__cabecera' },
      createEl('span', { className: 'movilMarca', ariaHidden: 'true' }),
      createEl('p', { className: 'movilLauncher__fecha', textContent: 'inicio' }),
      themeToggle.element,
    );
    const grid = createEl('div', {
      className: 'movilLauncher__grid',
      role: 'list',
      ariaLabel: 'Aplicaciones del launcher',
    });
    const nodes = Object.values(workspaceStore.get().nodes)
      .filter((node) => node.parentId === 'desktop')
      .sort((a, b) => (a.mobileOrder ?? 0) - (b.mobileOrder ?? 0));
    for (const node of nodes) {
      const button = createIconButton(node, openApp, openProfile);
      if (!button) continue;
      launcherGestureCleanups.push(bindLongPress(button, {
        onLongPress: (event) => openLauncherMenu(node, event),
      }).destroy);
      grid.appendChild(button);
    }
    const navButton = createEl('button', {
      type: 'button',
      className: 'movilLauncher__app',
      ariaLabel: 'Mostrar navegación',
      role: 'listitem',
    },
      createEl('span', { className: 'movilLauncher__pictograma', ariaHidden: 'true' }, createElement(Circle)),
      createEl('span', { className: 'movilLauncher__etiqueta', textContent: 'Navegación' }),
    );
    navButton.addEventListener('click', onToggleExternalNav);
    grid.appendChild(navButton);
    launcher.append(header, grid);
    return launcher;
  };

  const renderCurrent = (entry: MobileStackEntry | undefined): void => {
    if (entry || legacyContentVisible) clearLauncherResources();
    if (currentView) currentView.remove();
    if (entry) {
      routerOutlet.style.display = 'none';
      const contentClass = entry.layout === 'full-bleed'
        ? 'movilApp__contenido movilApp__contenido--fullBleed'
        : 'movilApp__contenido';
      const app = createEl('div', { className: 'movilApp' },
        createAppHeader(entry.title),
        createEl('div', { className: contentClass }, entry.view.element),
      );
      currentView = app;
      viewport.prepend(app);
    } else if (legacyContentVisible) {
      routerOutlet.style.display = 'block';
      currentView = routerOutlet;
      viewport.prepend(routerOutlet);
    } else {
      routerOutlet.style.display = 'none';
      currentView = renderLauncher();
      viewport.prepend(currentView);
    }
    const navigation = viewport.querySelector('.movilNavegacion');
    navigation?.remove();
    viewport.appendChild(createNavigation(Boolean(entry), goBack, goHome));
    routerOutlet.setAttribute('aria-hidden', entry || !legacyContentVisible ? 'true' : 'false');
  };

  function syncPathToTopMobileApp(): void {
    const top = getTopMobileApp();
    const app = top ? AppRegistry.get(top.appId) : undefined;
    const targetPath = app ? getCanonicalAppPath(app, top?.params) ?? '/' : '/';
    if (getCurrentPathname() !== targetPath) replacePath(targetPath);
  }

  function goBack(): void {
    if (destroyed) return;
    const entry = getTopMobileApp();
    if (!entry) return;

    const app = AppRegistry.get(entry.appId);
    const canonicalPath = app ? getCanonicalAppPath(app, entry.params) : null;
    const ownsCurrentHistoryEntry = Boolean(
      canonicalPath
      && canonicalPath === getCurrentPathname()
      && isInternalPushHistoryEntry(),
    );

    /* Una entrada marcada por el OS y representada por la URL actual puede
     * resolverse con history.back(): retiramos primero la vista y dejamos que
     * popstate reconcilie la ruta. Una vista local o un deep link externo no
     * consume historial ajeno: se desapila y se restaura la ruta de la vista
     * inferior, o la raíz si ya no queda ninguna. */
    if (ownsCurrentHistoryEntry) {
      popMobileApp({ preserveHistoryUrl: true });
      history.back();
    } else {
      popMobileApp();
      syncPathToTopMobileApp();
    }
  }

  function goHome(): void {
    if (destroyed) return;
    clearMobileStack();
    if (getCurrentPathname() !== '/') navigate('/');
  }

  const stopStack = mobileStackStore.subscribe((stack) => renderCurrent(stack.at(-1)));
  const stopWorkspace = workspaceStore.subscribe(() => {
    if (!getTopMobileApp() && !legacyContentVisible) renderCurrent(undefined);
  });

  function setLegacyContentVisible(visible: boolean): void {
    if (destroyed) return;
    legacyContentVisible = visible;
    if (!getTopMobileApp()) renderCurrent(undefined);
  }

  function destroy(): void {
    if (destroyed) return;
    destroyed = true;
    stopStack();
    stopWorkspace();
    clearLauncherResources();
    for (const controller of pendingControllers) controller.abort();
    pendingControllers.clear();
    /* MobileShell owns the stack cleanup; callers only destroy the shell. */
    clearMobileStack();
    currentView?.remove();
    currentView = null;
  }

  shell.append(viewport);
  return { element: shell, routerOutlet, setLegacyContentVisible, destroy, openApp, openProfile, goHome, goBack };
}
