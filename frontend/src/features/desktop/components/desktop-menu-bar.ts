/* wandori.us — Desktop Menu Bar
 * Barra superior del OS con menús desplegables.
 * Archivo: artículos del blog. Aplicaciones: apps del AppRegistry.
 * Configuración: abre la app de settings.
 * [Plan §2.3] Los menús proyectan CommandRegistry/AppRegistry. */

import { createElement, FileUser, Folder, type IconNode } from 'lucide';
import { AppRegistry, type Capability } from '../../runtime/app-registry';
import { authStore } from '../../../store';
import { ArticleService } from '../../../services';

/* === Estado del menú abierto === */
let openEntry: HTMLElement | null = null;

/** [Auditoría v3 §2.6] WeakMap tipado para callbacks de apertura — reemplaza _onOpen en DOM. */
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

/* === Toggle menú === */
function toggleEntry(entry: HTMLElement): void {
  if (openEntry === entry) {
    closeOpenMenu();
    return;
  }
  closeOpenMenu();

  const menu = entry.querySelector('.desktop-context-menu') as HTMLElement | null;
  const btn = entry.querySelector('.desktop-menu-bar__item') as HTMLButtonElement | null;

  /* Refresh reactive menus before showing */
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

/* === Crear label de menú === */
function createMenuLabel(label: string): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'desktop-menu-bar__item';
  button.textContent = label;
  button.setAttribute('aria-haspopup', 'menu');
  button.setAttribute('aria-expanded', 'false');
  return button;
}

/* === Crear item de menú contextual === */
function createMenuItem(
  label: string,
  options?: { icon?: IconNode; shortcut?: string; disabled?: boolean; onClick?: () => void },
): HTMLElement {
  const item = document.createElement('div');
  item.className = 'desktop-context-menu__item';
  item.setAttribute('role', 'menuitem');

  if (options?.icon) {
    const iconEl = document.createElement('span');
    iconEl.className = 'desktop-context-menu__icon';
    iconEl.appendChild(createElement(options.icon));
    item.appendChild(iconEl);
  }

  const labelEl = document.createElement('span');
  labelEl.className = 'desktop-context-menu__label';
  labelEl.textContent = label;
  item.appendChild(labelEl);

  if (options?.shortcut) {
    const shortcutEl = document.createElement('span');
    shortcutEl.className = 'desktop-context-menu__shortcut';
    shortcutEl.textContent = options.shortcut;
    item.appendChild(shortcutEl);
  }

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

/* === Menú Archivo — artículos del blog === */
function createArchiveMenu(): HTMLElement {
  const menu = document.createElement('div');
  menu.className = 'desktop-context-menu';
  menu.setAttribute('role', 'menu');
  menu.setAttribute('aria-label', 'Archivo');
  menu.hidden = true;

  /* Placeholder mientras carga */
  const loading = createMenuItem('cargando…', { disabled: true });
  menu.appendChild(loading);

  /* Cargar artículos desde la API */
  void ArticleService.list(1, 20)
    .then((data: any) => {
      loading.remove();

      if (!data || data.items?.length === 0) {
        menu.appendChild(createMenuItem('sin artículos', { disabled: true }));
        return;
      }

      for (const article of data.items) {
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

/* === Menú Aplicaciones — derivado de workspaceStore ===
 * [Plan 297A-11] Fuente única de verdad: los hijos del root del workspace
 * definen qué aparece en el menú. AppRegistry solo resuelve implementaciones.
 * Se re-consulta workspaceStore cada vez que se abre para reflejar cambios. */
function createApplicationsMenu(): HTMLElement {
  const menu = document.createElement('div');
  menu.className = 'desktop-context-menu';
  menu.setAttribute('role', 'menu');
  menu.setAttribute('aria-label', 'Aplicaciones');
  menu.hidden = true;

  /* Flag para saber si ya se cargó el workspace */
  let loaded = false;

  /** Re-construir los items del menú desde workspaceStore. */
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

  /* Cargar por primera vez */
  refresh();

  /* Exponer refresh para que toggleEntry lo llame al abrir */
  onOpenCallbacks.set(menu, () => {
    if (loaded) refresh();
  });

  return menu;
}

/* === Crear entrada de menú === */
function createMenuEntry(label: string, menu: HTMLElement): HTMLElement {
  const entry = document.createElement('div');
  entry.className = 'desktop-menu-bar__entry';

  const btn = createMenuLabel(label);
  entry.append(btn, menu);

  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    toggleEntry(entry);
  });

  return entry;
}

/* === Barra principal === */
export function createDesktopMenuBar(): HTMLElement {
  const bar = document.createElement('header');
  bar.className = 'desktop-menu-bar';

  const menus = document.createElement('div');
  menus.className = 'desktop-menu-bar__menus';

  /* Brand logo (cículo negro de marca) */
  const brand = document.createElement('span');
  brand.className = 'desktop-menu-bar__brand';
  brand.setAttribute('aria-label', 'Menú del sistema');

  /* Archivo — artículos */
  const archiveEntry = createMenuEntry('Archivo', createArchiveMenu());

  /* Aplicaciones — apps del OS */
  const appsEntry = createMenuEntry('Aplicaciones', createApplicationsMenu());

  /* Configuración — abre la app settings directamente */
  const settingsEntry = document.createElement('div');
  settingsEntry.className = 'desktop-menu-bar__entry';
  const settingsBtn = createMenuLabel('Configuración');
  settingsBtn.addEventListener('click', () => {
    void import('../../runtime/route-app-adapter').then(m => m.openAppWindow('settings'));
  });
  settingsEntry.appendChild(settingsBtn);

  menus.append(brand, archiveEntry, appsEntry, settingsEntry);

  /* Reloj */
  const clock = document.createElement('time');
  clock.className = 'desktop-menu-bar__clock';

  function updateClock(): void {
    const now = new Date();
    clock.textContent = now.toLocaleTimeString('es', { hour: '2-digit', minute: '2-digit', hour12: false });
  }
  updateClock();
  setInterval(updateClock, 30_000);

  bar.append(menus, clock);
  return bar;
}
