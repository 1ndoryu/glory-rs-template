import { describe, expect, it } from 'vitest';
import {
  createMapEditorState,
  mergeCatalogIntoManifest,
  placeInstance,
  moveInstance,
  duplicateInstance,
  deleteInstance,
  addSpawnPoint,
  moveSpawnPoint,
  deleteSpawnPoint,
  undo,
  redo,
  setTool,
  select,
  setActiveAsset,
  getValidationIssues,
  hasChanges,
} from './game-map-editor-core';
import { FIXTURE_MAP_VERSION } from './game-fixture-map';
import type { GameAssetAdminEntry } from '../../../../services/game-asset-admin.service';

const CATALOG: readonly GameAssetAdminEntry[] = [
  { id: 'tree', displayName: 'Árbol', category: 'tree', isActive: true, createdAt: '2026-08-02T00:00:00Z' },
  { id: 'rock', displayName: 'Roca', category: 'rock', isActive: true, createdAt: '2026-08-02T00:00:00Z' },
  { id: 'water', displayName: 'Agua', category: 'water', isActive: true, createdAt: '2026-08-02T00:00:00Z' },
  { id: 'inactive-tree', displayName: 'Inactivo', category: 'tree', isActive: false, createdAt: '2026-08-02T00:00:00Z' },
];

function makeState(): ReturnType<typeof createMapEditorState> {
  const base = mergeCatalogIntoManifest(FIXTURE_MAP_VERSION, CATALOG);
  return createMapEditorState(base, 3, CATALOG);
}

describe('game-map-editor-core (297A-64)', () => {
  it('fusiona el catálogo activo en el manifest sin tocar las entradas existentes', () => {
    const merged = mergeCatalogIntoManifest(FIXTURE_MAP_VERSION, CATALOG);
    expect(merged.assetManifest['tree']).toEqual({
      id: 'tree',
      category: 'tree',
      contentHash: 'catalog:tree',
    });
    /* Las entradas del fixture (fuera del catálogo) se conservan. */
    expect(merged.assetManifest['asset-conifer']).toBeDefined();
    /* Assets inactivos no entran al manifest. */
    expect(merged.assetManifest['inactive-tree']).toBeUndefined();
  });

  it('coloca una instancia del asset activo con id generado', () => {
    let state = makeState();
    state = setTool(state, 'place');
    state = setActiveAsset(state, 'tree');
    state = placeInstance(state, { x: 1, z: 2 });

    expect(state.document.instances).toHaveLength(FIXTURE_MAP_VERSION.instances.length + 1);
    const placed = state.document.instances[state.document.instances.length - 1];
    expect(placed.id).toBe('inst-1');
    expect(placed.assetVersionId).toBe('tree');
    expect(placed.position).toEqual({ x: 1, z: 2 });
    expect(state.selectedId).toBe('inst-1');
  });

  it('no coloca si la herramienta no es place o no hay asset activo', () => {
    let state = makeState();
    state = setTool(state, 'select');
    const before = state.document.instances.length;
    state = placeInstance(state, { x: 1, z: 2 });
    expect(state.document.instances).toHaveLength(before);
  });

  it('mueve, duplica y borra instancias', () => {
    let state = makeState();
    state = setTool(state, 'place');
    state = setActiveAsset(state, 'rock');
    state = placeInstance(state, { x: 1, z: 2 });
    const id = state.selectedId as string;

    state = moveInstance(state, id, { x: 5, z: 6 });
    expect(state.document.instances.find((i) => i.id === id)?.position).toEqual({ x: 5, z: 6 });

    state = duplicateInstance(state, id);
    const dupe = state.document.instances[state.document.instances.length - 1];
    expect(dupe.id).not.toBe(id);
    expect(dupe.position).toEqual({ x: 6, z: 7 });

    state = deleteInstance(state, id);
    expect(state.document.instances.some((i) => i.id === id)).toBe(false);
    expect(state.selectedId).toBeNull();
  });

  it('añade, mueve y borra spawns solo con la herramienta spawn', () => {
    let state = makeState();
    state = setTool(state, 'spawn');
    state = addSpawnPoint(state, { x: 0, z: 0 });
    const spawnId = state.selectedId as string;
    expect(state.document.spawnPoints.some((s) => s.id === spawnId)).toBe(true);

    state = moveSpawnPoint(state, spawnId, { x: 2, z: 3 });
    expect(state.document.spawnPoints.find((s) => s.id === spawnId)?.position).toEqual({ x: 2, z: 3 });

    state = setTool(state, 'select');
    const before = state.document.spawnPoints.length;
    state = addSpawnPoint(state, { x: 9, z: 9 });
    expect(state.document.spawnPoints).toHaveLength(before);

    state = deleteSpawnPoint(state, spawnId);
    expect(state.document.spawnPoints.some((s) => s.id === spawnId)).toBe(false);
  });

  it('deshace y rehace la secuencia de operaciones', () => {
    let state = makeState();
    state = setTool(state, 'place');
    state = setActiveAsset(state, 'tree');
    state = placeInstance(state, { x: 1, z: 2 });
    const afterPlace = state.document.instances.length;
    state = placeInstance(state, { x: 3, z: 4 });
    const afterTwo = state.document.instances.length;

    state = undo(state);
    expect(state.document.instances).toHaveLength(afterPlace);
    state = undo(state);
    expect(state.document.instances).toHaveLength(FIXTURE_MAP_VERSION.instances.length);
    state = redo(state);
    expect(state.document.instances).toHaveLength(afterPlace);
    state = redo(state);
    expect(state.document.instances).toHaveLength(afterTwo);
  });

  it('undo vacío no muta el estado', () => {
    const state = makeState();
    expect(undo(state)).toBe(state);
    expect(redo(state)).toBe(state);
  });

  it('hasChanges refleja el borrador frente a la base', () => {
    let state = makeState();
    expect(hasChanges(state)).toBe(false);
    /* Cambiar herramienta no altera el documento. */
    state = setTool(state, 'place');
    expect(hasChanges(state)).toBe(false);
    /* Colocar una instancia sí marca el borrador. */
    state = setActiveAsset(state, 'tree');
    state = placeInstance(state, { x: 1, z: 2 });
    expect(hasChanges(state)).toBe(true);
    /* Deshacer hasta la base limpia el estado. */
    state = undo(state);
    expect(hasChanges(state)).toBe(false);
  });

  it('colocar dentro de bounds produce un documento válido', () => {
    let state = makeState();
    state = setTool(state, 'place');
    state = setActiveAsset(state, 'tree');
    state = placeInstance(state, { x: 1, z: 2 });
    expect(getValidationIssues(state)).toEqual([]);
  });

  it('select y setActiveAsset solo cambian la selección/herramienta', () => {
    let state = makeState();
    state = select(state, 'rock-north');
    expect(state.selectedId).toBe('rock-north');
    state = setActiveAsset(state, 'water');
    expect(state.activeAssetId).toBe('water');
    expect(state.document).toBe(state.baseDocument);
  });
});
