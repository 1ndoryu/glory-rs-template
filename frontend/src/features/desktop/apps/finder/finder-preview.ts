/* wandori.us — Finder (File Browser)
 * Explorador de archivos del OS.
 * Lee hijos de un nodo del workspace y los renderiza usando ResourceTypeRegistry.
 * Soporta: navegación entre carpetas, abrir recursos, context menu, drag.
 * [Plan 297A-11] Reemplaza el preview hardcodeado de 297A-2.
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
import { workspaceStore, getChildren, moveNodeToParent } from '../../../runtime/workspace/workspace-store';
import { resolveResourceType } from '../../../runtime/resource-type-registry';
import { openContextMenu } from '../../components/desktop-context-menu';
import { selectSingle } from '../../../runtime/selection-store';
import { authStore } from '../../../../store';
import type { ResolvedNode } from '../../../runtime/workspace/types';

export interface FinderOptions {
  /** ID del nodo carpeta cuyos hijos se muestran. 'desktop' = raíz. */
  folderId: string;
  /** Callback para abrir una app con parámetros. */
  onOpenApp: (appId: string, params?: Record<string, string>) => void;
  /** Callback para crear carpeta dentro de este Finder. */
  onCreateFolder?: () => void;
}

/** Mapa de iconos Lucide por tipo de recurso. */
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

/** Construir breadcrumb path desde la raíz hasta folderId. */
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

/**
 * Crear el componente Finder (explorador de archivos).
 * Se suscribe a workspaceStore y renderiza los hijos de folderId.
 */
export function createFinderPreview(options: FinderOptions): HTMLElement {
  const finder = document.createElement('div');
  finder.className = 'desktop-finder';

  /* Breadcrumb path */
  const pathEl = document.createElement('div');
  pathEl.className = 'desktop-finder__path';

  /* Grid de items */
  const grid = document.createElement('div');
  grid.className = 'desktop-finder__grid';

  /* Drop en el grid = mover a este folder */
  grid.addEventListener('dragover', (e: DragEvent) => {
    if ((e.target as HTMLElement).closest('.desktop-finder__item')) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  });
  grid.addEventListener('drop', (e: DragEvent) => {
    if ((e.target as HTMLElement).closest('.desktop-finder__item')) return;
    e.preventDefault();
    const sourceId = e.dataTransfer?.getData('text/plain');
    if (sourceId) {
      moveNodeToParent(sourceId, options.folderId);
    }
  });

  finder.append(pathEl, grid);

  /* Suscribirse a workspaceStore para re-renderizar cuando cambie */
  workspaceStore.subscribe((ws) => {
    /* Breadcrumb */
    const crumbs = buildBreadcrumb(options.folderId, ws.nodes as Readonly<Record<string, ResolvedNode>>);
    pathEl.innerHTML = '';
    for (let i = 0; i < crumbs.length; i++) {
      if (i > 0) {
        const sep = document.createElement('span');
        sep.className = 'desktop-finder__path-sep';
        sep.textContent = ' / ';
        pathEl.appendChild(sep);
      }
      const crumb = document.createElement('button');
      crumb.type = 'button';
      crumb.className = 'desktop-finder__path-crumb';
      crumb.textContent = crumbs[i].label;
      crumb.addEventListener('click', () => {
        options.onOpenApp('finder', { folderId: crumbs[i].id });
      });
      pathEl.appendChild(crumb);
    }

    /* Hijos de este nodo */
    const children = getChildren(options.folderId);
    grid.innerHTML = '';

    if (children.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'desktop-finder__empty';
      empty.textContent = 'Carpeta vacía';
      grid.appendChild(empty);
      return;
    }

    for (const child of children) {
      const item = createFinderItem(child, options);

      /* Drag source */
      item.setAttribute('draggable', 'true');
      item.addEventListener('dragstart', (e: DragEvent) => {
        if (e.dataTransfer) {
          e.dataTransfer.effectAllowed = 'move';
          e.dataTransfer.setData('text/plain', child.id);
        }
        item.classList.add('desktop-finder__item--dragging');
      });
      item.addEventListener('dragend', () => {
        item.classList.remove('desktop-finder__item--dragging');
      });

      /* Drop target (solo carpetas) */
      if (child.type === 'folder') {
        item.addEventListener('dragover', (e: DragEvent) => {
          e.preventDefault();
          if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
          item.classList.add('desktop-finder__item--drop-target');
        });
        item.addEventListener('dragleave', () => {
          item.classList.remove('desktop-finder__item--drop-target');
        });
        item.addEventListener('drop', (e: DragEvent) => {
          e.preventDefault();
          item.classList.remove('desktop-finder__item--drop-target');
          const sourceId = e.dataTransfer?.getData('text/plain');
          if (sourceId && sourceId !== child.id) {
            moveNodeToParent(sourceId, child.id);
          }
        });
      }

      grid.appendChild(item);
    }
  });



  return finder;
}

/** Crear un item individual del Finder (carpeta, recurso, o app). */
function createFinderItem(
  node: ResolvedNode,
  options: FinderOptions,
): HTMLElement {
  const icon = getNodeIcon(node);
  const isFolder = node.type === 'folder';
  const isImage = node.type === 'resource' && node.resourceKind === 'image';

  const item = document.createElement(isImage ? 'figure' : 'button');
  if (!isImage) (item as HTMLButtonElement).type = 'button';
  item.className = `desktop-finder__item desktop-finder__item--${getNodeIconType(node)}`;
  item.setAttribute('aria-label', node.label);
  item.setAttribute('data-node-id', node.id);

  /* Thumbnail para imágenes */
  if (isImage && node.refId) {
    const img = document.createElement('img');
    img.className = 'desktop-finder__thumbnail';
    img.src = `/api/media/${node.refId}/preview`;
    img.alt = node.label;
    img.loading = 'lazy';
    img.onerror = () => { img.style.display = 'none'; };
    item.appendChild(img);
  } else {
    const iconEl = createElement(icon);
    iconEl.classList.add('desktop-finder__icon');
    item.appendChild(iconEl);
  }

  const label = document.createElement('span');
  label.className = 'desktop-finder__label';
  label.textContent = node.label;
  item.appendChild(label);

  /* Doble clic: abrir */
  item.addEventListener('dblclick', (e) => {
    e.stopPropagation();
    activateNode(node, options);
  });

  /* Clic simple: seleccionar */
  item.addEventListener('mousedown', ((e: MouseEvent) => {
    if (e.button === 0 && e.detail === 1) {
      selectSingle(node.id);
    }
  }) as EventListener);

  /* Context menu */
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
      capability: authStore.get().isAuthenticated ? 'admin' : 'public',
      x: e.clientX,
      y: e.clientY,
    });
  }) as EventListener);

  return item;
}

/** Activar un nodo: abrir en la app correcta. */
function activateNode(node: ResolvedNode, options: FinderOptions): void {
  if (node.type === 'folder') {
    /* Abrir carpeta en nueva ventana Finder */
    options.onOpenApp('finder', { folderId: node.id });
  } else if (node.type === 'resource' && node.resourceKind) {
    /* Abrir recurso en la app asignada por ResourceTypeRegistry */
    const entry = resolveResourceType(node.resourceKind);
    const appId = entry?.appId ?? 'finder';
    options.onOpenApp(appId, { resourceId: node.refId ?? node.id });
  } else if (node.type === 'app' && node.refId) {
    /* Abrir app directamente */
    options.onOpenApp(node.refId);
  }
}
