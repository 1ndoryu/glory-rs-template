/* GAME-01 — Vista del Editor de mapa 2D del Bosque (tab "mapa").
 * [297A-64] Canvas 2D top-down con grid de terreno, instancias por categoría
 * y spawns; paleta de assets del catálogo activo, herramientas (seleccionar /
 * colocar / spawn), undo/redo y publicación atómica con expectedVersion.
 * Reutiliza el contrato puro de game-core (validateMapVersion) y el servicio
 * GameMapAdminService; el runtime real sigue consumiendo el fixture. El
 * dibujo vive en game-map-editor-canvas.ts y el toolbar en
 * game-map-editor-toolbar.ts (este módulo queda <300 líneas). */

import { createEl } from '../../../../utils/dom';
import { createSelect } from '../../../../components/ui/select';
import { showConfirm } from '../../../../components/ui/confirm';
import { showToast } from '../../../../components/ui/toast';
import { tryCatch } from '../../../../utils/result';
import {
  GAME_MAP_ID,
  GameMapAdminService,
  type LoadedGameMap,
} from '../../../../services/game-map-admin.service';
import { GameAssetAdminService, type GameAssetAdminEntry } from '../../../../services/game-asset-admin.service';
import { FIXTURE_MAP_VERSION } from './game-fixture-map';
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
  paintSurface,
  setActiveSurface,
  undo,
  redo,
  setTool,
  select,
  setActiveAsset,
  getValidationIssues,
  type MapEditorState,
  type MapEditorTool,
  type TerrainSurfaceValue,
} from './game-map-editor-core';
import { createEditorToolbar } from './game-map-editor-toolbar';
import {
  CATEGORY_LABEL,
  drawMap,
  fitTransform,
  resizeCanvas,
  screenToWorld,
} from './game-map-editor-canvas';
import type { AssetCategory } from '../../../game-core';

export interface GameMapEditorHandle {
  readonly element: HTMLElement;
  readonly destroy: () => void;
}

function isAllowedCategory(category: string): category is AssetCategory {
  return category === 'terrain' || category === 'tree' || category === 'rock'
    || category === 'water' || category === 'character' || category === 'generic';
}

/** Crea el editor dentro del contenedor del tab "mapa". Devuelve el handle
 * con teardown (listeners, resize observer, cargas pendientes). */
export function createGameMapEditor(container: HTMLElement): GameMapEditorHandle {
  let disposed = false;
  let state: MapEditorState | null = null;
  let generation = 0;
  const cleanups: Array<() => void> = [];

  /* === Toolbar (módulo propio) === */
  const toolbarElements = createEditorToolbar();

  const canvasHost = createEl('div', { className: 'juegoConfig__editor-canvas' });
  const canvas = createEl('canvas', { className: 'juegoConfig__editor-surface' }) as HTMLCanvasElement;
  canvasHost.appendChild(canvas);

  const footer = createEl('div', { className: 'juegoConfig__editor-footer' },
    toolbarElements.hint, toolbarElements.issuesEl);
  container.append(toolbarElements.toolbar, canvasHost, footer);

  const redraw = (): void => {
    if (disposed || !state) return;
    resizeCanvas(canvas, canvasHost);
    drawMap(canvas, state);
    toolbarElements.refresh(state);
  };

  const onResize = (): void => redraw();
  /* [297A-64] ResizeObserver no existe en todos los entornos (jsdom de
   * pruebas); sin él el canvas mantiene su tamaño hasta el próximo dibujo. */
  if (typeof ResizeObserver !== 'undefined') {
    const resizeObserver = new ResizeObserver(onResize);
    resizeObserver.observe(canvasHost);
    cleanups.push(() => resizeObserver.disconnect());
  } else {
    window.addEventListener('resize', onResize);
    cleanups.push(() => window.removeEventListener('resize', onResize));
  }

  /* === Interacciones === */
  let dragTargetId: string | null = null;

  const onPointerDown = (event: PointerEvent): void => {
    if (!state) return;
    const rect = canvas.getBoundingClientRect();
    const bounds = state.document.terrain.bounds;
    const transform = fitTransform(bounds, canvas.width, canvas.height);
    const world = screenToWorld(event.clientX - rect.left, event.clientY - rect.top, transform);

    if (state.tool === 'place') {
      state = placeInstance(state, world);
      redraw();
      return;
    }
    if (state.tool === 'spawn') {
      state = addSpawnPoint(state, world);
      redraw();
      return;
    }
    /* [297A-66] Pincel: pintar la superficie de la celda bajo el cursor. */
    if (state.tool === 'paint') {
      state = paintSurface(state, world, state.activeSurface);
      redraw();
      return;
    }

    /* Tool select: clic selecciona la instancia/spawn más cercano. */
    const threshold = 14;
    let nearest: { id: string; distance: number } | null = null;
    for (const instance of state.document.instances) {
      const distance = Math.hypot(instance.position.x - world.x, instance.position.z - world.z);
      if (distance < threshold && (!nearest || distance < nearest.distance)) nearest = { id: instance.id, distance };
    }
    for (const spawn of state.document.spawnPoints) {
      const distance = Math.hypot(spawn.position.x - world.x, spawn.position.z - world.z);
      if (distance < threshold && (!nearest || distance < nearest.distance)) nearest = { id: spawn.id, distance };
    }
    state = select(state, nearest?.id ?? null);
    dragTargetId = nearest?.id ?? null;
    redraw();
  };

  const onPointerMove = (event: PointerEvent): void => {
    if (!state) return;
    const rect = canvas.getBoundingClientRect();
    const bounds = state.document.terrain.bounds;
    const transform = fitTransform(bounds, canvas.width, canvas.height);
    const world = screenToWorld(event.clientX - rect.left, event.clientY - rect.top, transform);
    /* [297A-66] El pincel pinta al arrastrar (cada celda distinta commitea). */
    if (state.tool === 'paint') {
      state = paintSurface(state, world, state.activeSurface);
      redraw();
      return;
    }
    if (!dragTargetId) return;
    const isSpawn = state.document.spawnPoints.some((s) => s.id === dragTargetId);
    state = isSpawn
      ? moveSpawnPoint(state, dragTargetId, world)
      : moveInstance(state, dragTargetId, world);
    redraw();
  };

  const onPointerUp = (): void => { dragTargetId = null; };

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  cleanups.push(() => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
  });

  const onTool = (tool: MapEditorTool): void => {
    if (state) {
      state = setTool(state, tool);
      redraw();
    }
  };
  toolbarElements.toolButtons.get('select')!.addEventListener('click', () => onTool('select'));
  toolbarElements.toolButtons.get('place')!.addEventListener('click', () => onTool('place'));
  toolbarElements.toolButtons.get('spawn')!.addEventListener('click', () => onTool('spawn'));
  toolbarElements.toolButtons.get('paint')!.addEventListener('click', () => onTool('paint'));
  toolbarElements.btnUndo.addEventListener('click', () => { if (state) { state = undo(state); redraw(); } });
  toolbarElements.btnRedo.addEventListener('click', () => { if (state) { state = redo(state); redraw(); } });
  toolbarElements.btnDelete.addEventListener('click', () => {
    if (!state?.selectedId) return;
    const selectedId = state.selectedId;
    const isSpawn = state.document.spawnPoints.some((s) => s.id === selectedId);
    state = isSpawn
      ? deleteSpawnPoint(state, selectedId)
      : deleteInstance(state, selectedId);
    redraw();
  });
  toolbarElements.btnDuplicate.addEventListener('click', () => {
    if (!state?.selectedId) return;
    const selectedId = state.selectedId;
    const isSpawn = state.document.spawnPoints.some((s) => s.id === selectedId);
    if (!isSpawn) state = duplicateInstance(state, selectedId);
    redraw();
  });

  const onPublish = async (): Promise<void> => {
    if (!state) return;
    /* [297A-64] Snapshot local: el narrowing de `state` no sobrevive a los
     * awaits de showConfirm/publish (variable mutable del closure). */
    const current = state;
    const issues = getValidationIssues(current);
    if (issues.length > 0) {
      showToast(`el mapa no es válido: ${issues[0].path} ${issues[0].message}`);
      return;
    }
    const confirmed = await showConfirm(`publicar una versión inmutable del mapa (v${current.activeVersion + 1})?`);
    if (!confirmed) return;
    const result = await tryCatch(GameMapAdminService.publish(current.document, current.activeVersion));
    if (!result.ok) {
      const message = result.error;
      showToast(message.includes('409') || message.includes('cambió')
        ? 'conflicto: el mapa cambió en el servidor; recarga y vuelve a editar'
        : `error al publicar: ${message}`);
      return;
    }
    showToast(`mapa publicado · v${result.value.version}`);
    /* Actualizar la base: la publicación es la nueva referencia de cambios. */
    const catalog = current.catalog;
    const reloaded = await tryCatch(loadMap());
    if (reloaded.ok && reloaded.value) {
      state = createMapEditorState(reloaded.value.document, reloaded.value.activeVersion, catalog);
      redraw();
    }
  };
  toolbarElements.btnPublish.addEventListener('click', () => void onPublish());
  /* [297A-66] Selector de superficie del pincel (handler nombrado para
   * poder retirarlo en destroy). */
  const onSurfaceChange = (): void => {
    if (!state) return;
    const value = Number(toolbarElements.surfaceSelect.value) as TerrainSurfaceValue;
    if (value === 0 || value === 1) state = setActiveSurface(state, value);
  };
  toolbarElements.surfaceSelect.addEventListener('change', onSurfaceChange);
  cleanups.push(() => toolbarElements.surfaceSelect.removeEventListener('change', onSurfaceChange));

  /* === Carga === */
  async function loadMap(): Promise<LoadedGameMap | null> {
    const existing = await GameMapAdminService.getActive(GAME_MAP_ID);
    return existing ?? { document: FIXTURE_MAP_VERSION, activeVersion: 0 };
  }

  async function init(): Promise<void> {
    const myGeneration = ++generation;
    toolbarElements.hint.textContent = 'cargando catálogo y mapa…';

    const catalogResult = await tryCatch(GameAssetAdminService.listAll());
    if (disposed || myGeneration !== generation) return;
    const catalog: GameAssetAdminEntry[] = catalogResult.ok
      ? catalogResult.value.filter((entry) => entry.isActive)
      : [];
    const activeAssets = catalog.filter((entry) => entry.isActive);

    const mapResult = await tryCatch(loadMap());
    if (disposed || myGeneration !== generation) return;
    if (!mapResult.ok || !mapResult.value) {
      toolbarElements.hint.textContent = 'no se pudo cargar el mapa';
      return;
    }
    const base = mergeCatalogIntoManifest(mapResult.value.document, activeAssets);
    state = createMapEditorState(base, mapResult.value.activeVersion, activeAssets);

    /* Poblar la paleta con los assets activos del catálogo. */
    const options = activeAssets.map((entry) => ({
      value: entry.id,
      label: `${entry.displayName} · ${CATEGORY_LABEL[isAllowedCategory(entry.category) ? entry.category : 'generic']}`,
    }));
    const palette = createSelect({
      label: 'asset',
      options,
      value: state.activeAssetId ?? options[0]?.value ?? '',
      onChange: (value) => { if (state) state = setActiveAsset(state, value || null); },
    });
    toolbarElements.assetSlot.replaceChildren(palette);

    toolbarElements.hint.textContent = catalogResult.ok
      ? `mapa ${base.id} · v${mapResult.value.activeVersion || 'sin publicar'} · ${base.instances.length} instancias · ${base.spawnPoints.length} spawns`
      : 'mapa cargado sin catálogo (solo assets existentes)';
    redraw();
  }

  void init();

  return {
    element: container,
    destroy: () => {
      if (disposed) return;
      disposed = true;
      generation += 1;
      for (const cleanup of cleanups) cleanup();
      container.textContent = '';
    },
  };
}
