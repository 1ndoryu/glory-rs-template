/* wandori.us — Overlay Mutations
 * Funciones que mutan el overlay del workspace. */

import type { GridPosition, NodeId, WorkspaceNode, ResolvedNode } from './types';

import { overlayStore, workspaceStore, releaseStore, EMPTY_OVERLAY } from './stores';

export function moveNodePosition(nodeId: NodeId, position: { col: number; row: number }): void {
  overlayStore.update((prev) => ({
    ...prev,
    fieldOverrides: {
      ...prev.fieldOverrides,
      [nodeId]: { ...prev.fieldOverrides[nodeId], position },
    },
  }));
}

/** [297A-20] Mueve varios nodos en un SOLO update del overlay.
 * Evita N re-renders al reencuadrar tras un resize del grid. */
export function moveNodesPosition(
  moves: ReadonlyArray<{ nodeId: NodeId; position: { col: number; row: number } }>,
): void {
  if (moves.length === 0) return;
  overlayStore.update((prev) => {
    const fieldOverrides = { ...prev.fieldOverrides };
    for (const move of moves) {
      fieldOverrides[move.nodeId] = { ...fieldOverrides[move.nodeId], position: move.position };
    }
    return { ...prev, fieldOverrides };
  });
}

export function moveMobileNodesPosition(
  moves: ReadonlyArray<{ nodeId: NodeId; mobilePosition: GridPosition }>,
): void {
  if (moves.length === 0) return;
  overlayStore.update((prev) => {
    const fieldOverrides = { ...prev.fieldOverrides };
    for (const move of moves) {
      fieldOverrides[move.nodeId] = {
        ...fieldOverrides[move.nodeId],
        mobilePosition: move.mobilePosition,
      };
    }
    return { ...prev, fieldOverrides };
  });
}

export function moveNodeToParent(nodeId: NodeId, parentId: NodeId | 'desktop' | null): void {
  overlayStore.update((prev) => ({
    ...prev,
    fieldOverrides: {
      ...prev.fieldOverrides,
      [nodeId]: { ...prev.fieldOverrides[nodeId], parentId },
    },
  }));
}

export function addOverlayNode(node: WorkspaceNode): void {
  overlayStore.update((prev) => ({
    ...prev,
    addedItems: { ...prev.addedItems, [node.id]: node },
  }));
}

export function tombstoneNode(nodeId: NodeId): void {
  overlayStore.update((prev) => ({
    ...prev,
    tombstones: [...prev.tombstones, nodeId],
    addedItems: (() => {
      const items = { ...prev.addedItems };
      if (items[nodeId]) delete items[nodeId];
      return items;
    })(),
    fieldOverrides: (() => {
      const overrides = { ...prev.fieldOverrides };
      delete overrides[nodeId];
      return overrides;
    })(),
  }));
}

export function restoreNode(nodeId: NodeId): void {
  overlayStore.update((prev) => ({
    ...prev,
    tombstones: prev.tombstones.filter((id) => id !== nodeId),
  }));
}

export function resetOverlay(): void {
  overlayStore.set(EMPTY_OVERLAY);
}

/** Compatibilidad para datos legacy. El launcher nuevo escribe mobilePosition;
 * esta función solo se conserva para importar overlays antiguos y no participa en
 * la política de orden de Finder ni en la geometría desktop. */
export function reorderWorkspaceNodes(orderedIds: readonly NodeId[]): void {
  overlayStore.update((prev) => {
    const overrides = { ...prev.fieldOverrides };
    for (let i = 0; i < orderedIds.length; i++) {
      overrides[orderedIds[i]] = { ...overrides[orderedIds[i]], mobileOrder: i };
    }
    return { ...prev, fieldOverrides: overrides };
  });
}

/** Compatibilidad con el drag desktop existente. */
export function reorderDesktopNodes(orderedIds: NodeId[]): void {
  reorderWorkspaceNodes(orderedIds);
}

export function createFolder(parentId: NodeId | 'desktop', label: string): NodeId {
  const ws = workspaceStore.get();
  const siblings = Object.values(ws.nodes).filter((n) => n.parentId === parentId);

  /* Evitar nombres duplicados en el mismo padre — añadir sufijo numérico */
  let uniqueLabel = label;
  const existingLabels = new Set(siblings.map(n => n.label));
  if (existingLabels.has(uniqueLabel)) {
    let counter = 2;
    while (existingLabels.has(`${label} (${counter})`)) counter++;
    uniqueLabel = `${label} (${counter})`;
  }

  const id = `folder-${Date.now()}`;
  addOverlayNode({
    id,
    parentId,
    type: 'folder',
    label: uniqueLabel,
    mobilePosition: {
      col: siblings.length % 3,
      row: Math.floor(siblings.length / 3),
    },
    mobileOrder: siblings.length,
    requires: 'public',
  });
  return id;
}

export function getTombstonedNodes(): WorkspaceNode[] {
  const release = releaseStore.get();
  const overlay = overlayStore.get();
  return overlay.tombstones
    .map((id) => release.nodes[id])
    .filter((n): n is WorkspaceNode => n !== undefined);
}

/** Devuelve hijos sin imponer una política de presentación.
 * Finder y otras superficies de contenido no deben heredar el orden del launcher;
 * la superficie móvil usa `sortMobileNodes` explícitamente cuando lo necesita. */
export function getChildren(parentId: NodeId | 'desktop'): ResolvedNode[] {
  const ws = workspaceStore.get();
  return Object.values(ws.nodes).filter((n) => n.parentId === parentId);
}
