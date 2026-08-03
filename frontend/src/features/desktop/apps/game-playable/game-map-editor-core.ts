/* GAME-01 — Core puro del Editor de mapa 2D del Bosque.
 * [297A-64] Estado y operaciones de edición SIN DOM: documento MapVersion
 * (borrador), selección, herramienta, paleta y command stack con undo/redo.
 * Cada mutación produce un documento nuevo (inmutabilidad por spreads); el
 * stack guarda snapshots del documento para deshacer/rehacer. El documento se
 * revalida con `validateMapVersion` en cada operación y al publicar. */

import type { GameAssetAdminEntry } from '../../../../services/game-asset-admin.service';
import {
  validateMapVersion,
  type AssetInstance,
  type GameAssetVersion,
  type MapVersion,
  type MapValidationIssue,
  type SpawnPoint,
  type Vector2,
} from '../../../game-core';

export type MapEditorTool = 'select' | 'place' | 'spawn';

export interface MapEditorState {
  /** Borrador actual (inmutable por operación). */
  readonly document: MapVersion;
  /** Documento cargado original: `hasChanges` compara contra él. */
  readonly baseDocument: MapVersion;
  /** Versión activa al cargar: `expectedVersion` para publicar (0 si ninguna). */
  readonly activeVersion: number;
  readonly tool: MapEditorTool;
  /** Id de instancia o spawn seleccionado (tool 'select'). */
  readonly selectedId: string | null;
  /** Asset de la paleta activo para colocar (tool 'place'). */
  readonly activeAssetId: string | null;
  /** Catálogo de assets activos que alimenta la paleta y el manifest. */
  readonly catalog: readonly GameAssetAdminEntry[];
  readonly undoStack: readonly MapVersion[];
  readonly redoStack: readonly MapVersion[];
}

const DEFAULT_INSTANCE_SCALE = 1;
const DEFAULT_SPAWN_RADIUS = 0.5;

/** Ids de assets del catálogo permitidos en el manifest (evita colisiones con
 * ids reservados del documento). */
const RESERVED_MANIFEST_IDS = new Set(['__proto__', 'prototype', 'constructor', 'toString', 'valueOf', 'hasOwnProperty']);

export function createMapEditorState(
  baseDocument: MapVersion,
  activeVersion: number,
  catalog: readonly GameAssetAdminEntry[],
): MapEditorState {
  return {
    document: baseDocument,
    baseDocument,
    activeVersion,
    tool: 'select',
    selectedId: null,
    activeAssetId: catalog[0]?.id ?? null,
    catalog,
    undoStack: [],
    redoStack: [],
  };
}

/** Clona un documento para que los snapshots del stack sean independientes. */
function cloneDocument(document: MapVersion): MapVersion {
  return JSON.parse(JSON.stringify(document)) as MapVersion;
}

/** Manifest del documento con los assets del catálogo activo fusionados: el
 * documento cargado (fixture o publicación) conserva sus entradas (una
 * instancia existente puede referenciar un id fuera del catálogo) y el editor
 * añade los assets activos del catálogo para poder colocarlos. */
export function mergeCatalogIntoManifest(
  document: MapVersion,
  catalog: readonly GameAssetAdminEntry[],
): MapVersion {
  const manifest: Record<string, GameAssetVersion> = { ...document.assetManifest };
  for (const asset of catalog) {
    if (!asset.isActive) continue;
    if (RESERVED_MANIFEST_IDS.has(asset.id)) continue;
    if (manifest[asset.id]) continue;
    manifest[asset.id] = {
      id: asset.id,
      category: isAllowedCategory(asset.category) ? asset.category : 'generic',
      contentHash: `catalog:${asset.id}`,
    };
  }
  return { ...document, assetManifest: manifest };
}

function isAllowedCategory(category: string): category is GameAssetVersion['category'] {
  return category === 'terrain' || category === 'tree' || category === 'rock'
    || category === 'water' || category === 'character' || category === 'generic';
}

/** Siguiente id numérico de instancia (`inst-N`) o spawn (`spawn-N`). */
function nextNumericId(ids: readonly string[], prefix: string): string {
  let max = 0;
  for (const id of ids) {
    if (id.startsWith(prefix)) {
      const suffix = Number(id.slice(prefix.length));
      if (Number.isInteger(suffix) && suffix > max) max = suffix;
    }
  }
  return `${prefix}${max + 1}`;
}

/** Aplica una mutación y apila el snapshot anterior (undo), limpiando redo. */
function commit(
  state: MapEditorState,
  nextDocument: MapVersion,
): MapEditorState {
  return {
    ...state,
    document: nextDocument,
    undoStack: [...state.undoStack.slice(-49), cloneDocument(state.document)],
    redoStack: [],
  };
}

export function setTool(state: MapEditorState, tool: MapEditorTool): MapEditorState {
  return { ...state, tool };
}

export function select(state: MapEditorState, id: string | null): MapEditorState {
  return { ...state, selectedId: id };
}

export function setActiveAsset(state: MapEditorState, assetId: string | null): MapEditorState {
  return { ...state, activeAssetId: assetId };
}

/** Coloca una instancia del asset de la paleta en la posición dada (mundo). */
export function placeInstance(state: MapEditorState, position: Vector2): MapEditorState {
  const assetId = state.activeAssetId;
  if (state.tool !== 'place' || !assetId) return state;
  const instanceId = nextNumericId(state.document.instances.map((i) => i.id), 'inst-');
  const instance: AssetInstance = {
    id: instanceId,
    assetVersionId: assetId,
    position: { x: position.x, z: position.z },
    rotationY: 0,
    scale: DEFAULT_INSTANCE_SCALE,
    terrainAnchor: 'surface',
  };
  const next: MapVersion = {
    ...state.document,
    instances: [...state.document.instances, instance],
  };
  return { ...commit(state, next), selectedId: instanceId };
}

export function moveInstance(state: MapEditorState, id: string, position: Vector2): MapEditorState {
  const index = state.document.instances.findIndex((i) => i.id === id);
  if (index < 0) return state;
  const instances = state.document.instances.map((instance, i) => (
    i === index
      ? { ...instance, position: { x: position.x, z: position.z } }
      : instance
  ));
  return commit(state, { ...state.document, instances });
}

export function duplicateInstance(state: MapEditorState, id: string): MapEditorState {
  const source = state.document.instances.find((i) => i.id === id);
  if (!source) return state;
  const newId = nextNumericId(state.document.instances.map((i) => i.id), 'inst-');
  const instance: AssetInstance = {
    ...source,
    id: newId,
    position: { x: source.position.x + 1, z: source.position.z + 1 },
  };
  const next: MapVersion = {
    ...state.document,
    instances: [...state.document.instances, instance],
  };
  return { ...commit(state, next), selectedId: newId };
}

export function deleteInstance(state: MapEditorState, id: string): MapEditorState {
  const next: MapVersion = {
    ...state.document,
    instances: state.document.instances.filter((i) => i.id !== id),
  };
  return { ...commit(state, next), selectedId: null };
}

/** Añade un spawn en la posición dada (tool 'spawn'). */
export function addSpawnPoint(state: MapEditorState, position: Vector2): MapEditorState {
  if (state.tool !== 'spawn') return state;
  const spawnId = nextNumericId(state.document.spawnPoints.map((s) => s.id), 'spawn-');
  const spawn: SpawnPoint = {
    id: spawnId,
    position: { x: position.x, z: position.z },
    radius: DEFAULT_SPAWN_RADIUS,
  };
  const next: MapVersion = {
    ...state.document,
    spawnPoints: [...state.document.spawnPoints, spawn],
  };
  return { ...commit(state, next), selectedId: spawnId };
}

export function moveSpawnPoint(state: MapEditorState, id: string, position: Vector2): MapEditorState {
  const index = state.document.spawnPoints.findIndex((s) => s.id === id);
  if (index < 0) return state;
  const spawnPoints = state.document.spawnPoints.map((spawn, i) => (
    i === index
      ? { ...spawn, position: { x: position.x, z: position.z } }
      : spawn
  ));
  return commit(state, { ...state.document, spawnPoints });
}

export function deleteSpawnPoint(state: MapEditorState, id: string): MapEditorState {
  const next: MapVersion = {
    ...state.document,
    spawnPoints: state.document.spawnPoints.filter((s) => s.id !== id),
  };
  return { ...commit(state, next), selectedId: null };
}

export function undo(state: MapEditorState): MapEditorState {
  const previous = state.undoStack[state.undoStack.length - 1];
  if (!previous) return state;
  return {
    ...state,
    document: previous,
    undoStack: state.undoStack.slice(0, -1),
    redoStack: [...state.redoStack, cloneDocument(state.document)],
    selectedId: null,
  };
}

export function redo(state: MapEditorState): MapEditorState {
  const next = state.redoStack[state.redoStack.length - 1];
  if (!next) return state;
  return {
    ...state,
    document: next,
    redoStack: state.redoStack.slice(0, -1),
    undoStack: [...state.undoStack, cloneDocument(state.document)],
    selectedId: null,
  };
}

export function getValidationIssues(state: MapEditorState): readonly MapValidationIssue[] {
  return validateMapVersion(state.document);
}

export function hasChanges(state: MapEditorState): boolean {
  return JSON.stringify(state.document) !== JSON.stringify(state.baseDocument);
}
