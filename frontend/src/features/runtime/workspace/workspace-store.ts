/* wandori.us — Workspace Store
 * API y re-exports del workspace. Los stores viven en stores.ts
 * para romper el ciclo de importación con overlay-mutations.
 * [Plan 297A-11 §9.1–9.4] [Auditoría v2] */

import { WorkspaceService } from '../../../services';
import { showToast } from '../../../components/ui/toast';
import { rebaseOverlay } from './merge';
import type {
  NodeId,
  WorkspaceNode,
  WorkspaceTree,
} from './types';

/* Re-export stores y constantes desde stores.ts */
export { releaseStore, overlayStore, workspaceStore, EMPTY_OVERLAY, previewPublicStore } from './stores';

/* Import local para funciones API */
import { releaseStore, overlayStore, workspaceStore, EMPTY_OVERLAY } from './stores';

/* === API === */

export async function fetchWorkspaceRelease(): Promise<void> {
  try {
    const data = await WorkspaceService.getActiveRelease();
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
      resourceKind: node.resourceKind,
      publicLocator: node.publicLocator,
      position: node.position,
      mobilePosition: node.mobilePosition,
      mobileOrder: node.mobileOrder,
      requires: node.requires,
    };
  }
  const tree: WorkspaceTree = { version: resolved.releaseVersion + 1, nodes };
  const result = await WorkspaceService.publish(tree);
  if (result?.version) {
    releaseStore.set(result.tree);
    overlayStore.set(EMPTY_OVERLAY);
    return { version: result.version };
  }
  return null;
}

/** Rollback a una versión anterior del release (admin).
 *  Re-publica el árbol antiguo como nueva versión y limpia el overlay. */
export async function rollbackWorkspace(targetVersion: number): Promise<boolean> {
  try {
    const oldRelease = await WorkspaceService.getReleaseByVersion(targetVersion);
    if (!oldRelease?.tree) return false;
    const result = await WorkspaceService.publish(oldRelease.tree);
    if (result?.version) {
      releaseStore.set(result.tree);
      overlayStore.set(EMPTY_OVERLAY);
      showToast(`Rollback exitoso (v${result.version})`);
      return true;
    }
  } catch {
    showToast('Error al restaurar versión anterior');
  }
  return false;
}

/* Re-export submodules for backward compatibility */
export { moveNodePosition, moveNodesPosition, moveMobileNodesPosition, moveNodeToParent, addOverlayNode, tombstoneNode, restoreNode, resetOverlay, reorderDesktopNodes, reorderWorkspaceNodes, createFolder, getTombstonedNodes, getChildren } from './overlay-mutations';
export { getClipboard, setClipboard, clearClipboard, pasteFromClipboard } from './clipboard';
export type { ClipboardMode, ClipboardEntry } from './clipboard';
