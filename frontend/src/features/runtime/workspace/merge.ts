/* wandori.us — Workspace Merge
 * Algoritmo puro: merge release + overlay → resolved workspace. */

import type {
  NodeId,
  WorkspaceNode,
  WorkspaceTree,
  WorkspaceOverlay,
  ResolvedNode,
  ResolvedWorkspace,
} from './types';

/**
 * Merge release + overlay → resolved workspace.
 * 1. Clone release nodes
 * 2. Remove tombstones (and orphan children)
 * 3. Apply field overrides (position, label, parentId, mobileOrder)
 * 4. Add overlay items
 * 5. Filter by auth capability
 */
export function mergeWorkspace(
  release: WorkspaceTree,
  overlay: WorkspaceOverlay,
  capability: 'public' | 'authenticated' | 'admin',
): ResolvedWorkspace {
  const result: Record<NodeId, ResolvedNode> = {};

  for (const [id, node] of Object.entries(release.nodes)) {
    result[id] = { ...node, origin: 'release' };
  }

  const tombstoneSet = new Set(overlay.tombstones);
  for (const tombId of tombstoneSet) {
    delete result[tombId];
  }
  for (const id of tombstoneSet) {
    for (const node of Object.values(result)) {
      if (node.parentId === id) {
        tombstoneSet.add(node.id);
        delete result[node.id];
      }
    }
  }

  for (const [id, overrides] of Object.entries(overlay.fieldOverrides)) {
    const existing = result[id];
    if (existing) Object.assign(existing, overrides);
  }

  for (const [id, node] of Object.entries(overlay.addedItems)) {
    result[id] = { ...node, origin: 'overlay' };
  }

  const hierarchy = ['public', 'authenticated', 'admin'] as const;
  const level = hierarchy.indexOf(capability);
  for (const [id, node] of Object.entries(result)) {
    if (node.requires && hierarchy.indexOf(node.requires) > level) {
      delete result[id];
    }
  }

  return { releaseVersion: release.version, nodes: result };
}

/** Rebase overlay ante un release nuevo. */
export function rebaseOverlay(
  newRelease: WorkspaceTree,
  currentOverlay: WorkspaceOverlay,
): WorkspaceOverlay {
  const releaseIds = new Set(Object.keys(newRelease.nodes));

  const validTombstones = currentOverlay.tombstones.filter((id) => releaseIds.has(id));

  const validOverrides: Record<NodeId, Partial<Pick<WorkspaceNode, 'position' | 'label' | 'parentId' | 'mobileOrder'>>> = {};
  for (const [id, overrides] of Object.entries(currentOverlay.fieldOverrides)) {
    if (releaseIds.has(id)) {
      validOverrides[id] = overrides;
    }
  }

  if (
    validTombstones.length === currentOverlay.tombstones.length
    && Object.keys(validOverrides).length === Object.keys(currentOverlay.fieldOverrides).length
  ) {
    return currentOverlay;
  }

  return {
    version: currentOverlay.version,
    addedItems: currentOverlay.addedItems,
    fieldOverrides: validOverrides,
    tombstones: validTombstones,
  };
}
