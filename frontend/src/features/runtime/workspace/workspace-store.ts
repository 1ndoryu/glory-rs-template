/* wandori.us — Workspace Store
 * Store reactivo del workspace. Re-exporta merge, mutations y clipboard.
 * Este módulo contiene: stores, subscriptions, persistence y API.
 * [Plan 297A-11 §9.1–9.4] */

import { createStore, authStore } from '../../../store';
import { api } from '../../../api/client';
import { DEFAULT_RELEASE } from './default-release';
import { mergeWorkspace, rebaseOverlay } from './merge';
import type {
  NodeId,
  WorkspaceNode,
  WorkspaceTree,
  WorkspaceOverlay,
  ResolvedWorkspace,
} from './types';

/* === Constants === */
const OVERLAY_KEY = 'wandorius:workspace-overlay';
const OVERLAY_VERSION = 1;

export const EMPTY_OVERLAY: WorkspaceOverlay = {
  version: OVERLAY_VERSION,
  addedItems: {},
  fieldOverrides: {},
  tombstones: [],
};

/* === Persistence === */

function loadOverlay(): WorkspaceOverlay {
  try {
    const raw = localStorage.getItem(OVERLAY_KEY);
    if (!raw) return EMPTY_OVERLAY;
    const parsed = JSON.parse(raw) as WorkspaceOverlay;
    if (parsed.version !== OVERLAY_VERSION) return EMPTY_OVERLAY;
    return parsed;
  } catch {
    return EMPTY_OVERLAY;
  }
}

function saveOverlay(overlay: WorkspaceOverlay): void {
  try {
    localStorage.setItem(OVERLAY_KEY, JSON.stringify(overlay));
  } catch {
    /* localStorage full or unavailable */
  }
}

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

/* === Stores === */

export const releaseStore = createStore<WorkspaceTree>(DEFAULT_RELEASE);
export const overlayStore = createStore<WorkspaceOverlay>(loadOverlay());
export const workspaceStore = createStore<ResolvedWorkspace>(
  mergeWorkspace(DEFAULT_RELEASE, loadOverlay(), 'public'),
);

overlayStore.subscribe((overlay) => {
  saveOverlay(overlay);
});

let recomputeScheduled = false;
function scheduleRecompute(): void {
  if (recomputeScheduled) return;
  recomputeScheduled = true;
  queueMicrotask(() => {
    recomputeScheduled = false;
    const release = releaseStore.get();
    const overlay = overlayStore.get();
    const auth = authStore.get();
    const capability: 'public' | 'authenticated' | 'admin' = auth.isAuthenticated ? 'admin' : 'public';
    workspaceStore.set(mergeWorkspace(release, overlay, capability));
  });
}

releaseStore.subscribe(() => scheduleRecompute());
overlayStore.subscribe(() => scheduleRecompute());
authStore.subscribe(() => scheduleRecompute());

/* Re-export submodules for backward compatibility */
export { moveNodePosition, moveNodeToParent, addOverlayNode, tombstoneNode, restoreNode, resetOverlay, reorderDesktopNodes, createFolder, getTombstonedNodes, getChildren } from './overlay-mutations';
export { getClipboard, setClipboard, clearClipboard, pasteFromClipboard } from './clipboard';
export type { ClipboardMode, ClipboardEntry } from './clipboard';
