/* wandori.us — Finder (File Browser)
 * Explorador de archivos del OS.
 * Lee hijos de un nodo del workspace y los renderiza usando ResourceTypeRegistry.
 * Soporta: navegación entre carpetas, abrir recursos, context menu, drag.
 * [Plan 297A-11] Reemplaza el preview hardcodeado de 297A-2.
 * [Auditoría v2] Navega dentro de la misma ventana en vez de abrir una nueva por carpeta.
 */

import {
  Folder,
  FileText,
  Image,
  Music,
  Video,
  File,
  Package,
  createElement,
  type IconNode,
} from 'lucide';
import { createEl } from '../../../../utils/dom';
import { workspaceStore, getChildren } from '../../../runtime/workspace/workspace-store';
import { resolveResourceType } from '../../../runtime/resource-type-registry';
import { openContextMenu } from '../../components/desktop-context-menu';
import { selectSingle } from '../../../runtime/selection-store';
import { enableDrag, makeDropTarget } from '../../utils/icon-drag';
import { authStore } from '../../../../store';
import type { ResolvedNode } from '../../../runtime/workspace/types';

export interface FinderOptions {
  folderId: string;
  onOpenApp: (appId: string, params?: Record<string, string>) => void;
  onCreateFolder?: () => void;
  onNavigate?: (folderId: string, label: string) => void;
}

const RESOURCE_ICON_MAP: Record<string, IconNode> = {
  article: FileText,
  about: FileText,
  project: Package,
  product: Package,
  image: Image,
  audio: Music,
  video: Video,
  document: FileText,
  folder: Folder,
  generic: File,
};

function getNodeIcon(node: ResolvedNode): IconNode {
  if (node.type === 'folder') return Folder;
  if (node.type === 'resource' && node.resourceKind) {
    return RESOURCE_ICON_MAP[node.resourceKind] ?? File;
  }
  if (node.type === 'app') {
    return RESOURCE_ICON_MAP[node.refId ?? ''] ?? Package;
  }
  return File;
}

function getNodeIconType(node: ResolvedNode): 'folder' | 'document' | 'application' {
  if (node.type === 'folder') return 'folder';
  if (node.type === 'resource') return 'document';
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
    render();

    makeDropTarget({ el: grid, dropId: currentFolderId, context: 'finder' });

    const ws = workspaceStore.get();
    const node = ws.nodes[folderId];
    const label = node?.label ?? (folderId === 'desktop' ? 'Escritorio' : 'Galería');
    options.onNavigate?.(folderId, label);
  }

  makeDropTarget({ el: grid, dropId: currentFolderId, context: 'finder' });

  finder.append(pathEl, grid);

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
  } else if (node.type === 'resource' && node.resourceKind) {
    const entry = resolveResourceType(node.resourceKind);
    const appId = entry?.appId ?? 'finder';
    options.onOpenApp(appId, { resourceId: node.refId ?? node.id });
  } else if (node.type === 'app' && node.refId) {
    options.onOpenApp(node.refId);
  }
}
