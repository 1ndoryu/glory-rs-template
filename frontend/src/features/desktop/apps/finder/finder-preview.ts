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
import { workspaceStore, getChildren, moveNodeToParent } from '../../../runtime/workspace/workspace-store';
import { resolveResourceType } from '../../../runtime/resource-type-registry';
import { openContextMenu } from '../../components/desktop-context-menu';
import { selectSingle } from '../../../runtime/selection-store';
import { authStore } from '../../../../store';
import type { ResolvedNode } from '../../../runtime/workspace/types';

export interface FinderOptions {
  /** ID del nodo carpeta inicial. 'desktop' = raíz. */
  folderId: string;
  /** Callback para abrir una app con parámetros (solo para recursos/apps, NO carpetas). */
  onOpenApp: (appId: string, params?: Record<string, string>) => void;
  /** Callback para crear carpeta dentro de este Finder. */
  onCreateFolder?: () => void;
  /** Callback para notificar cambio de carpeta (actualiza título de ventana). */
  onNavigate?: (folderId: string, label: string) => void;
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
 * Mantiene estado interno de carpeta actual — navegar entre carpetas
 * re-renderiza dentro de la misma ventana en vez de abrir una nueva.
 */
export function createFinderPreview(options: FinderOptions): HTMLElement {
  const finder = document.createElement('div');
  finder.className = 'desktop-finder';

  /* Estado interno de navegación */
  let currentFolderId = options.folderId;

  /* Breadcrumb path */
  const pathEl = document.createElement('div');
  pathEl.className = 'desktop-finder__path';

  /* Grid de items */
  const grid = document.createElement('div');
  grid.className = 'desktop-finder__grid';

  /** Navegar a una carpeta dentro de esta misma ventana Finder. */
  function navigateTo(folderId: string): void {
    currentFolderId = folderId;
    render();

    /* Notificar cambio de título de ventana */
    const ws = workspaceStore.get();
    const node = ws.nodes[folderId];
    const label = node?.label ?? (folderId === 'desktop' ? 'Escritorio' : 'Galería');
    options.onNavigate?.(folderId, label);
  }

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
      moveNodeToParent(sourceId, currentFolderId);
    }
  });

  finder.append(pathEl, grid);

  /** Renderizar breadcrumb y contenido de la carpeta actual. */
  function render(): void {
    const ws = workspaceStore.get();

    /* Breadcrumb */
    const crumbs = buildBreadcrumb(currentFolderId, ws.nodes as Readonly<Record<string, ResolvedNode>>);
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
      const crumbId = crumbs[i].id;
      crumb.addEventListener('click', () => {
        navigateTo(crumbId);
      });
      pathEl.appendChild(crumb);
    }

    /* Hijos de este nodo */
    const children = getChildren(currentFolderId);
    grid.innerHTML = '';

    if (children.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'desktop-finder__empty';
      empty.textContent = 'Carpeta vacía';
      grid.appendChild(empty);
      return;
    }

    for (const child of children) {
      const item = createFinderItem(child, navigateTo, options);

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
  }

  /* Suscribirse a workspaceStore para re-renderizar cuando cambie */
  workspaceStore.subscribe(() => {
    render();
  });

  return finder;
}

/** Crear un item individual del Finder (carpeta, recurso, o app). */
function createFinderItem(
  node: ResolvedNode,
  navigateTo: (folderId: string) => void,
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
    activateNode(node, navigateTo, options);
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

/** Activar un nodo: carpetas navegan dentro de la ventana, recursos/apps abren en nueva ventana. */
function activateNode(
  node: ResolvedNode,
  navigateTo: (folderId: string) => void,
  options: FinderOptions,
): void {
  if (node.type === 'folder') {
    /* Navegar dentro de la misma ventana Finder */
    navigateTo(node.id);
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
