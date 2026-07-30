/* wandori.us — Workspace Icon Grid
 * Grid reactivo de iconos del escritorio suscrito a workspaceStore.
 * [Plan 297A-11 §9.4] Extraído de desktop-shell.ts para reducir acoplamiento. */

import {
  FileUser,
  Folder,
  ShieldUser,
  type IconNode,
} from 'lucide';
import { createDesktopIcon } from './components/desktop-icon';
import { openAppWindow } from '../runtime/route-app-adapter';
import { authStore } from '../../store';
import { openContextMenu } from './components/desktop-context-menu';
import { selectSingle, clearSelection } from '../runtime/selection-store';
import { workspaceStore, reorderDesktopNodes } from '../runtime/workspace/workspace-store';
import type { ResolvedNode } from '../runtime/workspace/types';
import { AppRegistry } from '../runtime/app-registry';
import { resolveResourceType, type ResourceKind } from '../runtime/resource-type-registry';
import { enableIconDrag } from './utils/icon-drag';

const SHELL_ICON_MAP: Record<string, IconNode> = {
  'profile': FileUser,
  'admin': ShieldUser,
};

export function resolveNodeIcon(node: ResolvedNode): IconNode {
  if (node.type === 'app' && node.refId) {
    const app = AppRegistry.get(node.refId);
    if (app) return app.icon;
  }
  return SHELL_ICON_MAP[node.id] ?? Folder;
}

export function resolveNodeIconType(node: ResolvedNode): 'folder' | 'document' | 'application' {
  if (node.type === 'app' && node.refId) {
    const app = AppRegistry.get(node.refId);
    if (app?.iconType) return app.iconType;
  }
  if (node.type === 'folder') return 'folder';
  if (node.type === 'shortcut') return 'document';
  if (node.type === 'resource') return 'document';
  return 'application';
}

export function createWorkspaceIconGrid(extraActions?: Record<string, () => void>): HTMLElement {
  const grid = document.createElement('div');
  grid.className = 'desktop-icon-grid';
  grid.setAttribute('aria-label', 'Objetos del escritorio');

  const dragCleanups = new Map<string, () => void>();

  workspaceStore.subscribe((ws) => {
    for (const cleanup of dragCleanups.values()) cleanup();
    dragCleanups.clear();
    grid.innerHTML = '';

    const desktopNodes = Object.values(ws.nodes)
      .filter((n) => n.parentId === 'desktop')
      .sort((a, b) => (a.mobileOrder ?? 0) - (b.mobileOrder ?? 0));

    for (const node of desktopNodes) {
      const onActivate = extraActions?.[node.id]
        ?? (node.type === 'folder' ? () => {
          void openAppWindow('finder', { folderId: node.id });
        } : node.type === 'resource' && node.resourceKind ? () => {
          const entry = resolveResourceType(node.resourceKind! as ResourceKind);
          void openAppWindow(entry?.appId ?? 'finder', { resourceId: node.refId ?? node.id });
        } : node.refId ? () => {
          void openAppWindow(node.refId!);
        } : undefined);
      if (!onActivate) continue;

      const iconEl = createDesktopIcon({
        label: node.label,
        type: resolveNodeIconType(node),
        lucideIcon: resolveNodeIcon(node),
        onActivate,
      });

      iconEl.addEventListener('mousedown', (e) => {
        if (e.button === 0 && e.detail === 1) {
          selectSingle(node.id);
        }
      });

      iconEl.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        selectSingle(node.id);
        openContextMenu({
          context: 'icon',
          targets: [{ id: node.refId ?? node.id, kind: node.type === 'app' ? 'app' : 'shortcut' }],
          capability: authStore.get().isAuthenticated ? 'admin' : 'public',
          x: e.clientX,
          y: e.clientY,
        });
      });

      const cleanup = enableIconDrag({
        iconEl,
        nodeId: node.id,
        gridEl: grid,
        onReorder: (draggedId, targetIndex) => {
          const currentIds = Object.values(ws.nodes)
            .filter((n) => n.parentId === 'desktop')
            .sort((a, b) => (a.mobileOrder ?? 0) - (b.mobileOrder ?? 0))
            .map((n) => n.id);
          const fromIndex = currentIds.indexOf(draggedId);
          if (fromIndex < 0 || fromIndex === targetIndex) return;
          const reordered = [...currentIds];
          reordered.splice(fromIndex, 1);
          reordered.splice(targetIndex, 0, draggedId);
          reorderDesktopNodes(reordered);
        },
      });
      dragCleanups.set(node.id, cleanup);

      grid.appendChild(iconEl);
    }
  });

  /* Clic en vacío limpia selección */
  grid.addEventListener('mousedown', (e) => {
    if (e.target === grid) clearSelection();
  });

  return grid;
}
