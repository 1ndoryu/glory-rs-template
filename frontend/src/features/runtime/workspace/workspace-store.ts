/* wandori.us — Workspace Store
 * Store reactivo que mergea release + overlay → resolved workspace.
 * Persiste overlay del invitado en localStorage.
 * [Plan 297A-11 §9.1–9.4] */

import { createStore, authStore } from '../../../store';
import { DEFAULT_RELEASE } from './default-release';
import type {
  NodeId,
  WorkspaceNode,
  WorkspaceTree,
  WorkspaceOverlay,
  ResolvedNode,
  ResolvedWorkspace,
} from './types';

/* === Constants === */
const OVERLAY_KEY = 'wandorius:workspace-overlay';
const OVERLAY_VERSION = 1;

/* === Empty overlay === */
const EMPTY_OVERLAY: WorkspaceOverlay = {
  version: OVERLAY_VERSION,
  addedItems: {},
  fieldOverrides: {},
  tombstones: [],
};

/* === Merge algorithm === */

/**
 * Merge release + overlay → resolved workspace.
 * 1. Clone release nodes
 * 2. Remove tombstones (and orphan children)
 * 3. Apply field overrides (position, label, parentId, mobileOrder)
 * 4. Add overlay items
 * 5. Filter by auth capability
 */
function mergeWorkspace(
  release: WorkspaceTree,
  overlay: WorkspaceOverlay,
  capability: 'public' | 'authenticated' | 'admin',
): ResolvedWorkspace {
  const result: Record<NodeId, ResolvedNode> = {};

  /* Step 1: Clone release nodes */
  for (const [id, node] of Object.entries(release.nodes)) {
    result[id] = { ...node, origin: 'release' };
  }

  /* Step 2: Remove tombstones and orphan children */
  const tombstoneSet = new Set(overlay.tombstones);
  for (const tombId of tombstoneSet) {
    delete result[tombId];
  }
  /* Remove children of tombstoned parents */
  for (const id of tombstoneSet) {
    for (const node of Object.values(result)) {
      if (node.parentId === id) {
        tombstoneSet.add(node.id);
        delete result[node.id];
      }
    }
  }

  /* Step 3: Apply field overrides */
  for (const [id, overrides] of Object.entries(overlay.fieldOverrides)) {
    const existing = result[id];
    if (existing) {
      Object.assign(existing, overrides);
    }
  }

  /* Step 4: Add overlay items */
  for (const [id, node] of Object.entries(overlay.addedItems)) {
    result[id] = { ...node, origin: 'overlay' };
  }

  /* Step 5: Filter by capability */
  const hierarchy = ['public', 'authenticated', 'admin'] as const;
  const level = hierarchy.indexOf(capability);
  for (const [id, node] of Object.entries(result)) {
    if (node.requires && hierarchy.indexOf(node.requires) > level) {
      delete result[id];
    }
  }

  return { releaseVersion: release.version, nodes: result };
}

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
    /* localStorage full or unavailable — silent fail */
  }
}

/* === Stores === */

/** Release inmutable (admin publica esto). */
export const releaseStore = createStore<WorkspaceTree>(DEFAULT_RELEASE);

/** Overlay del usuario/invitado (persistido en localStorage). */
export const overlayStore = createStore<WorkspaceOverlay>(loadOverlay());

/** Workspace resuelto (merge de release + overlay). */
export const workspaceStore = createStore<ResolvedWorkspace>(
  mergeWorkspace(DEFAULT_RELEASE, loadOverlay(), 'public'),
);

/* Persistir overlay al cambiar */
overlayStore.subscribe((overlay) => {
  saveOverlay(overlay);
});

/* Re-merge cuando cambia release, overlay o auth */
function recompute(): void {
  const release = releaseStore.get();
  const overlay = overlayStore.get();
  const auth = authStore.get();
  const capability: 'public' | 'authenticated' | 'admin' = auth.isAuthenticated ? 'admin' : 'public';
  workspaceStore.set(mergeWorkspace(release, overlay, capability));
}

releaseStore.subscribe(() => recompute());
overlayStore.subscribe(() => recompute());
authStore.subscribe(() => recompute());

/* === Overlay mutation helpers === */

/** Mover un nodo a una nueva posición grid. */
export function moveNodePosition(nodeId: NodeId, position: { col: number; row: number }): void {
  overlayStore.update((prev) => ({
    ...prev,
    fieldOverrides: {
      ...prev.fieldOverrides,
      [nodeId]: { ...prev.fieldOverrides[nodeId], position },
    },
  }));
}

/** Cambiar el padre de un nodo (mover a carpeta). */
export function moveNodeToParent(nodeId: NodeId, parentId: NodeId | 'desktop' | null): void {
  overlayStore.update((prev) => ({
    ...prev,
    fieldOverrides: {
      ...prev.fieldOverrides,
      [nodeId]: { ...prev.fieldOverrides[nodeId], parentId },
    },
  }));
}

/** Añadir un nodo nuevo al overlay. */
export function addOverlayNode(node: WorkspaceNode): void {
  overlayStore.update((prev) => ({
    ...prev,
    addedItems: { ...prev.addedItems, [node.id]: node },
  }));
}

/** Eliminar un nodo (tombstone). */
export function tombstoneNode(nodeId: NodeId): void {
  overlayStore.update((prev) => ({
    ...prev,
    tombstones: [...prev.tombstones, nodeId],
    /* Si fue añadido por overlay, eliminarlo de addedItems en vez de tombstone */
    addedItems: (() => {
      const items = { ...prev.addedItems };
      if (items[nodeId]) {
        delete items[nodeId];
        return items;
      }
      return items;
    })(),
    fieldOverrides: (() => {
      const overrides = { ...prev.fieldOverrides };
      delete overrides[nodeId];
      return overrides;
    })(),
  }));
}

/** Restaurar un nodo de la papelera. */
export function restoreNode(nodeId: NodeId): void {
  overlayStore.update((prev) => ({
    ...prev,
    tombstones: prev.tombstones.filter((id) => id !== nodeId),
  }));
}

/** Resetear overlay al estado por defecto. */
export function resetOverlay(): void {
  overlayStore.set(EMPTY_OVERLAY);
}

/** Obtener nodos hijos directos de un padre. */
export function getChildren(parentId: NodeId | 'desktop'): ResolvedNode[] {
  const ws = workspaceStore.get();
  return Object.values(ws.nodes)
    .filter((n) => n.parentId === parentId)
    .sort((a, b) => (a.mobileOrder ?? 0) - (b.mobileOrder ?? 0));
}
