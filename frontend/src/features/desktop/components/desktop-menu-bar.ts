/* wandori.us — Desktop Menu Bar
 * Barra superior del OS con menús desplegables.
 * Archivo: artículos del blog. Aplicaciones: apps del AppRegistry.
 * Configuración: abre la app de settings.
 * [Plan §2.3] Los menús proyectan CommandRegistry/AppRegistry. */

import { createElement, FileUser, Folder, type IconNode } from 'lucide';
import { createEl } from '../../../utils/dom';
import { AppRegistry, type Capability } from '../../runtime/app-registry';
import { authStore } from '../../../store';
import { ArticleService } from '../../../services';

let openEntry: HTMLElement | null = null;

const onOpenCallbacks = new WeakMap<HTMLElement, () => void>();

function closeOpenMenu(): void {
  if (openEntry) {
    openEntry.classList.remove('desktop-menu-bar__entry--open');
    const menu = openEntry.querySelector('.desktop-context-menu') as HTMLElement | null;
    if (menu) menu.hidden = true;
    const btn = openEntry.querySelector('.desktop-menu-bar__item') as HTMLButtonElement | null;
    if (btn) btn.setAttribute('aria-expanded', 'false');
    openEntry = null;
  }
  document.removeEventListener('click', onOutsideClick);
  document.removeEventListener('keydown', onEscapeKey);
}

function onOutsideClick(e: MouseEvent): void {
  if (openEntry && !openEntry.contains(e.target as Node)) {
    closeOpenMenu();
  }
}

function onEscapeKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') closeOpenMenu();
}

function toggleEntry(entry: HTMLElement): void {
  if (openEntry === entry) {
    closeOpenMenu();
    return;
  }
  closeOpenMenu();

  const menu = entry.querySelector('.desktop-context-menu') as HTMLElement | null;
  const btn = entry.querySelector('.desktop-menu-bar__item') as HTMLButtonElement | null;

  if (menu && onOpenCallbacks.has(menu)) {
    onOpenCallbacks.get(menu)!();
  }

  entry.classList.add('desktop-menu-bar__entry--open');
  if (menu) menu.hidden = false;
  if (btn) btn.setAttribute('aria-expanded', 'true');

  openEntry = entry;

  setTimeout(() => {
    document.addEventListener('click', onOutsideClick);
    document.addEventListener('keydown', onEscapeKey);
  }, 0);
}

function createMenuLabel(label: string): HTMLButtonElement {
  return createEl('button', {
    type: 'button', className: 'desktop-menu-bar__item', textContent: label,
    ariaHaspopup: 'menu', ariaExpanded: 'false',
  });
}

function createMenuItem(
  label: string,
  options?: { icon?: IconNode; shortcut?: string; disabled?: boolean; onClick?: () => void },
): HTMLElement {
  const children: (string | HTMLElement)[] = [];

  if (options?.icon) {
    children.push(createEl('span', { className: 'desktop-context-menu__icon' }, createElement(options.icon)));
  }

  children.push(createEl('span', { className: 'desktop-context-menu__label', textContent: label }));

  if (options?.shortcut) {
    children.push(createEl('span', { className: 'desktop-context-menu__shortcut', textContent: options.shortcut }));
  }

  const item = createEl('div', { className: 'desktop-context-menu__item', role: 'menuitem' }, ...children);

  if (options?.disabled) {
    item.classList.add('desktop-context-menu__item--disabled');
    item.setAttribute('aria-disabled', 'true');
  } else if (options?.onClick) {
    item.addEventListener('click', (e) => {
      e.stopPropagation();
      closeOpenMenu();
      options.onClick!();
    });
  }

  return item;
}

function createArchiveMenu(): HTMLElement {
  const menu = createEl('div', { className: 'desktop-context-menu', role: 'menu', ariaLabel: 'Archivo' });
  menu.hidden = true;

  const loading = createMenuItem('cargando…', { disabled: true });
  menu.appendChild(loading);

  void ArticleService.list(1, 20)
    .then(({ items }) => {
      loading.remove();

      if (items.length === 0) {
        menu.appendChild(createMenuItem('sin artículos', { disabled: true }));
        return;
      }

      for (const article of items) {
        menu.appendChild(createMenuItem(article.title, {
          icon: FileUser,
          onClick: () => {
            void import('../../../router').then(r => r.navigate(`/article/${article.slug}`));
          },
        }));
      }
    })
    .catch(() => {
      loading.remove();
      menu.appendChild(createMenuItem('error al cargar', { disabled: true }));
    });

  return menu;
}

function createApplicationsMenu(): HTMLElement {
  const menu = createEl('div', { className: 'desktop-context-menu', role: 'menu', ariaLabel: 'Aplicaciones' });
  menu.hidden = true;

  let loaded = false;

  function refresh(): void {
    menu.innerHTML = '';

    void import('../../runtime/workspace/workspace-store').then(({ workspaceStore }) => {
      const ws = workspaceStore.get();
      const capability: Capability = authStore.get().isAuthenticated ? 'admin' : 'public';

      const launcherItems = Object.values(ws.nodes)
        .filter(n => n.parentId === 'desktop' && (n.type === 'app' || n.type === 'folder'))
        .sort((a, b) => (a.mobileOrder ?? 0) - (b.mobileOrder ?? 0));

      for (const node of launcherItems) {
        const icon: IconNode = node.type === 'app' && node.refId
          ? (AppRegistry.get(node.refId)?.icon ?? Folder)
          : Folder;

        if (node.type === 'app' && node.refId) {
          const appDef = AppRegistry.get(node.refId);
          if (appDef) {
            const hierarchy: Capability[] = ['public', 'authenticated', 'admin'];
            if (hierarchy.indexOf(appDef.requires) > hierarchy.indexOf(capability)) continue;
          }
        }

        menu.appendChild(createMenuItem(node.label, {
          icon,
          onClick: () => {
            const params: Record<string, string> | undefined =
              node.type === 'folder' ? { folderId: node.id }
              : node.refId ? undefined
              : undefined;
            const appId = node.type === 'app' && node.refId ? node.refId : 'finder';
            void import('../../runtime/route-app-adapter').then(m => m.openAppWindow(appId, params));
          },
        }));
      }

      if (launcherItems.length === 0) {
        menu.appendChild(createMenuItem('sin aplicaciones', { disabled: true }));
      }

      loaded = true;
    }).catch(() => {
      menu.innerHTML = '';
      menu.appendChild(createMenuItem('error al cargar', { disabled: true }));
    });
  }

  refresh();

  onOpenCallbacks.set(menu, () => {
    if (loaded) refresh();
  });

  return menu;
}

function createMenuEntry(label: string, menu: HTMLElement): HTMLElement {
  const btn = createMenuLabel(label);
  const entry = createEl('div', { className: 'desktop-menu-bar__entry' }, btn, menu);

  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    toggleEntry(entry);
  });

  return entry;
}

export function createDesktopMenuBar(): HTMLElement {
  const brand = createEl('span', { className: 'desktop-menu-bar__brand', ariaLabel: 'Menú del sistema' });

  const archiveEntry = createMenuEntry('Archivo', createArchiveMenu());
  const appsEntry = createMenuEntry('Aplicaciones', createApplicationsMenu());

  const settingsBtn = createMenuLabel('Configuración');
  settingsBtn.addEventListener('click', () => {
    void import('../../runtime/route-app-adapter').then(m => m.openAppWindow('settings'));
  });
  const settingsEntry = createEl('div', { className: 'desktop-menu-bar__entry' }, settingsBtn);

  const menus = createEl('div', { className: 'desktop-menu-bar__menus' },
    brand, archiveEntry, appsEntry, settingsEntry,
  );

  const clock = createEl('time', { className: 'desktop-menu-bar__clock' });

  function updateClock(): void {
    const now = new Date();
    clock.textContent = now.toLocaleTimeString('es', { hour: '2-digit', minute: '2-digit', hour12: false });
  }
  updateClock();
  setInterval(updateClock, 30_000);

  return createEl('header', { className: 'desktop-menu-bar' }, menus, clock);
}
