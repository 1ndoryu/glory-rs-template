/* wandori.us — Workspace Store
 * API y re-exports del workspace. Los stores viven en stores.ts
 * para romper el ciclo de importación con overlay-mutations.
 * [Plan 297A-11 §9.1–9.4] [Auditoría v2] */

import { api } from '../../../api/client';
import { rebaseOverlay } from './merge';
import type {
  NodeId,
  WorkspaceNode,
  WorkspaceTree,
} from './types';

/* Re-export stores y constantes desde stores.ts */
export { releaseStore, overlayStore, workspaceStore, EMPTY_OVERLAY } from './stores';

/* Import local para funciones API */
import { releaseStore, overlayStore, workspaceStore, EMPTY_OVERLAY } from './stores';

/* === API === */

export async function fetchWorkspaceRelease(): Promise<void> {
  try {
    const data = await api.get<{ version: number; tree: WorkspaceTree }>('/api/workspace/release');
    if (data?.tree?.nodes) {
      const currentRelease = releaseStore.get();
      if (data.tree.version !== currentRelease.version) {
        const currentOverlay = overlayStore.get();
        const rebased = rebaseOverlay(data.tree, currentOverlay);
        if (rebased !== currentOverlay) {
          overlayStore.set(rebased);
        }
      }
      releaseStore.set(data.tree);
    }
  } catch {
    /* API no disponible — usar DEFAULT_RELEASE */
  }
}

export async function publishWorkspace(): Promise<{ version: number } | null> {
  const resolved = workspaceStore.get();
  const nodes: Record<NodeId, WorkspaceNode> = {};
  for (const [id, node] of Object.entries(resolved.nodes)) {
    nodes[id] = {
      id: node.id,
      parentId: node.parentId,
      type: node.type,
      label: node.label,
      refId: node.refId,
      position: node.position,
      mobileOrder: node.mobileOrder,
      requires: node.requires,
    };
  }
  const tree: WorkspaceTree = { version: resolved.releaseVersion + 1, nodes };
  const result = await api.post<{ version: number; tree: WorkspaceTree }>('/api/admin/workspace/publish', { tree });
  if (result?.version) {
    releaseStore.set(result.tree);
    overlayStore.set(EMPTY_OVERLAY);
    return { version: result.version };
  }
  return null;
}

/* Re-export submodules for backward compatibility */
export { moveNodePosition, moveNodeToParent, addOverlayNode, tombstoneNode, restoreNode, resetOverlay, reorderDesktopNodes, createFolder, getTombstonedNodes, getChildren } from './overlay-mutations';
export { getClipboard, setClipboard, clearClipboard, pasteFromClipboard } from './clipboard';
export type { ClipboardMode, ClipboardEntry } from './clipboard';
