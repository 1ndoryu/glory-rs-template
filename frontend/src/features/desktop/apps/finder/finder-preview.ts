/* wandori.us — Finder (File Browser)
 * Explorador de archivos del OS.
 * Lee hijos de un nodo del workspace y los renderiza usando ResourceTypeRegistry.
 * Soporta: navegación entre carpetas, abrir recursos, context menu, drag.
 * [Plan 297A-11] Reemplaza el preview hardcodeado de 297A-2.
 * [Auditoría v2] Navega dentro de la misma ventana en vez de abrir una nueva por carpeta.
 */

import {
  File,
  Package,
  createElement,
  type IconNode,
} from 'lucide';
import { createEl } from '../../../../utils/dom';
import { workspaceStore, getChildren } from '../../../runtime/workspace/workspace-store';
import { openContextMenu } from '../../components/desktop-context-menu';
import {
  selectionStore,
  selectSingle,
  selectBackground,
  isSelected,
  clearSelection,
} from '../../../runtime/selection-store';
import { enableDrag, makeDropTarget } from '../../utils/icon-drag';
import { authStore } from '../../../../store';
import type { ResolvedNode } from '../../../runtime/workspace/types';
import { resolvePublicResourceTarget } from '../../../runtime/workspace/public-resource-locator';
import { AppRegistry } from '../../../runtime/app-registry';
import { resolveResourceIcon, resolveResourceIconType } from '../../../runtime/resource-type-registry';
import { showToast } from '../../../../components/ui/toast';
import { createModal } from '../../../../components/ui/modal';
import { trackImageDownload } from '../../../analytics/tracker';

export interface FinderOptions {
  folderId: string;
  onOpenApp: (appId: string, params?: Record<string, string>) => void;
  onNavigate?: (folderId: string, label: string) => void;
}

/* [018A-79] Los iconos ya no viven en un mapa local: los recursos resuelven en
 * ResourceTypeRegistry (fuente única con escritorio y móvil) y las apps en
 * AppRegistry. Antes este mapa local divergía del escritorio (artículo → carpeta). */
function getNodeIcon(node: ResolvedNode): IconNode {
  if (node.type === 'folder') return resolveResourceIcon('folder');
  if (node.type === 'resource' && node.resourceKind) {
    return resolveResourceIcon(node.resourceKind);
  }
  if (node.type === 'app' && node.refId) {
    return AppRegistry.get(node.refId)?.icon ?? Package;
  }
  return File;
}

function getNodeIconType(node: ResolvedNode): 'folder' | 'document' | 'application' {
  if (node.type === 'folder') return 'folder';
  if (node.type === 'resource' && node.resourceKind) {
    return resolveResourceIconType(node.resourceKind);
  }
  if (node.type === 'app') return 'application';
  return 'document';
}

function buildBreadcrumb(
  folderId: string,
  nodes: Readonly<Record<string, ResolvedNode>>,
): { id: string; label: string }[] {
  const crumbs: { id: string; label: string }[] = [];
  let current: string | null = folderId;

  while (current && current !== 'desktop') {
    const n: ResolvedNode | undefined = nodes[current];
    if (!n) break;
    crumbs.unshift({ id: n.id, label: n.label });
    current = n.parentId;
  }

  crumbs.unshift({ id: 'desktop', label: 'Escritorio' });
  return crumbs;
}

export function createFinderPreview(options: FinderOptions): HTMLElement {
  const finder = createEl('div', { className: 'desktop-finder' });

  let currentFolderId = options.folderId;

  const pathEl = createEl('div', { className: 'desktop-finder__path' });
  const grid = createEl('div', { className: 'desktop-finder__grid' });

  function navigateTo(folderId: string): void {
    currentFolderId = folderId;
    /* [018A-88] Al cambiar de carpeta la selección anterior deja de existir:
     * se limpia para que no queden ids huérfanos en el store global. */
    clearSelection();
    render();

    makeDropTarget({ el: grid, dropId: currentFolderId, context: 'finder' });

    const ws = workspaceStore.get();
    const node = ws.nodes[folderId];
    /* [018A-87] Ya no hay carpeta "Galería" en el workspace: el fallback es genérico. */
    const label = node?.label ?? (folderId === 'desktop' ? 'Escritorio' : folderId);
    options.onNavigate?.(folderId, label);
  }

  makeDropTarget({ el: grid, dropId: currentFolderId, context: 'finder' });

  /* [018A-88] Menú contextual del fondo de carpeta: el grid (o su estado
   * vacío) abre las acciones de creación del contexto 'finder'. Los ítems
   * del grid tienen su propio handler con stopPropagation, así que este
   * solo dispara sobre el fondo. Patrón espejo de desktop-shell.ts. */
  grid.addEventListener('contextmenu', ((e: MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target !== grid && !target.classList.contains('desktop-finder__empty')) return;
    e.preventDefault();
    selectBackground();
    openContextMenu({
      context: 'finder',
      targets: [{ id: currentFolderId, kind: 'folder' }],
      capability: authStore.get().capability,
      x: e.clientX,
      y: e.clientY,
    });
  }) as EventListener);

  finder.append(pathEl, grid);

  /* [018A-90] Navegación programática desde comandos globales (workspace:open):
   * el comando no puede invocar navigateTo (closure), así que dispara un
   * evento sobre el content de la ventana del Finder enfocada; el preview lo
   * traduce a navigateTo y onNavigate sincroniza título y params en el
   * windowStore (taskbar + reapertura de la carpeta de origen). */
  finder.addEventListener('finder:navigate', ((e: Event) => {
    const folderId = (e as CustomEvent<{ folderId?: string }>).detail?.folderId;
    if (folderId) navigateTo(folderId);
  }) as EventListener);

  function render(): void {
    const ws = workspaceStore.get();

    const crumbs = buildBreadcrumb(currentFolderId, ws.nodes as Readonly<Record<string, ResolvedNode>>);
    pathEl.innerHTML = '';
    for (let i = 0; i < crumbs.length; i++) {
      if (i > 0) {
        pathEl.appendChild(createEl('span', { className: 'desktop-finder__path-sep', textContent: ' / ' }));
      }
      const crumb = createEl('button', {
        type: 'button', className: 'desktop-finder__path-crumb', textContent: crumbs[i].label,
      });
      const crumbId = crumbs[i].id;
      crumb.addEventListener('click', () => { navigateTo(crumbId); });
      pathEl.appendChild(crumb);
    }

    const children = getChildren(currentFolderId);
    grid.innerHTML = '';

    if (children.length === 0) {
      grid.appendChild(createEl('div', { className: 'desktop-finder__empty', textContent: 'Carpeta vacía' }));
      return;
    }

    for (const child of children) {
      const item = createFinderItem(child, navigateTo, options);

      enableDrag({
        el: item, nodeId: child.id, context: 'finder', gridEl: grid,
      });

      if (child.type === 'folder') {
        makeDropTarget({ el: item, dropId: child.id, context: 'finder' });
      }

      grid.appendChild(item);
    }
  }

  workspaceStore.subscribe(() => { render(); });

  /* [018A-88] Re-render al cambiar la selección: los ítems aplican la clase
   * --selected según selectionStore (antes la selección existía solo en el
   * store, sin reflejo visual). */
  selectionStore.subscribe(() => { render(); });

  return finder;
}

function createFinderItem(
  node: ResolvedNode,
  navigateTo: (folderId: string) => void,
  options: FinderOptions,
): HTMLElement {
  const icon = getNodeIcon(node);
  const isFolder = node.type === 'folder';
  const isImage = node.type === 'resource' && node.resourceKind === 'image';

  const item: HTMLElement = isImage
    ? createEl('figure', { className: `desktop-finder__item desktop-finder__item--${getNodeIconType(node)}`, ariaLabel: node.label })
    : createEl('button', {
        type: 'button', className: `desktop-finder__item desktop-finder__item--${getNodeIconType(node)}`, ariaLabel: node.label,
      });
  item.setAttribute('data-node-id', node.id);

  /* [018A-88] Estado de selección visible: la clase --selected reutiliza los
   * tokens de selección del OS (--sistema-inverso-*) igual que el escritorio. */
  const selected = isSelected(node.id);
  item.classList.toggle('desktop-finder__item--selected', selected);
  item.setAttribute('aria-selected', String(selected));

  if (isImage && node.refId) {
    const img = createEl('img', {
      className: 'desktop-finder__thumbnail', src: `/api/media/${node.refId}/preview`, alt: node.label, loading: 'lazy',
    });
    img.onerror = () => { img.style.display = 'none'; };
    item.appendChild(img);
  } else {
    const iconSvg = createElement(icon);
    iconSvg.classList.add('desktop-finder__icon');
    item.appendChild(iconSvg);
  }

  item.appendChild(createEl('span', { className: 'desktop-finder__label', textContent: node.label }));

  item.addEventListener('dblclick', (e) => {
    e.stopPropagation();
    activateNode(node, navigateTo, options);
  });

  item.addEventListener('mousedown', ((e: MouseEvent) => {
    if (e.button === 0 && e.detail === 1) {
      selectSingle(node.id);
    }
  }) as EventListener);

  item.addEventListener('contextmenu', ((e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    selectSingle(node.id);

    const context = isFolder ? 'folder' : 'icon';
    const kind = isFolder ? 'folder' as const : (node.type === 'resource' ? 'shortcut' as const : 'app' as const);
    const targets = [{ id: node.refId ?? node.id, kind }];

    openContextMenu({
      context,
      targets,
      capability: authStore.get().capability,
      x: e.clientX,
      y: e.clientY,
    });
  }) as EventListener);

  return item;
}

function activateNode(
  node: ResolvedNode,
  navigateTo: (folderId: string) => void,
  options: FinderOptions,
): void {
  if (node.type === 'folder') {
    navigateTo(node.id);
  } else if (node.type === 'resource' && node.resourceKind === 'image' && node.refId) {
    /* [018A-87] Las imágenes de Documentos se abren con visor local (preview
     * pública + descargar), sin pasar por un deep link de app. */
    openImagePreview(node.refId, node.label);
  } else if (node.type === 'resource' && node.resourceKind) {
    const publicTarget = resolvePublicResourceTarget(node);
    if (publicTarget) {
      options.onOpenApp(publicTarget.appId, publicTarget.params);
      return;
    }
    showToast('Este recurso todavía no tiene una referencia pública disponible');
  } else if (node.type === 'app' && node.refId) {
    options.onOpenApp(node.refId);
  }
}

/* [018A-87] Visor modal de imagen para los recursos de Documentos.
 * Usa la preview pública (/api/media/{id}/preview) y permite descargar.
 * Reutiliza createModal y trackImageDownload (mismo patrón que la página
 * /gallery) para no duplicar recetas visuales. */
function openImagePreview(mediaId: string, label: string): void {
  const url = `/api/media/${mediaId}/preview`;

  const fullImg = createEl('img', { src: url, alt: label });
  fullImg.style.width = '100%';
  fullImg.style.border = 'var(--borde)';

  const btnDescargar = createEl('button', { className: 'boton', textContent: 'descargar' });
  btnDescargar.addEventListener('click', () => {
    trackImageDownload(url);
    const a = createEl('a', { href: url, download: label || 'imagen' });
    a.click();
  });

  const container = createEl('div', {}, fullImg, btnDescargar);
  createModal({ titulo: label, contenido: container, ancho: '800px' });
}
