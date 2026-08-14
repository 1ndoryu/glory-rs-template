/* GAME-01 — Comparador visual del toolkit procedural (138A-1/138A-2).
 * Monta el MISMO seed con dos estilos derivados de la misma base de altura:
 * 'bloques' reutiliza el mesher del experimento 128A-1 vía adaptador de
 * cuantización, y 'suave' usa el heightfield-mesh + vegetación low-poly del
 * toolkit (árboles con ramas y césped por matas, 138A-2).
 * Solo presentación y métricas estructurales para que el usuario decida el
 * estilo con evidencia; desde 138A-3 ambos modos comparten el MISMO agua de
 * costa (espuma + niebla) que la isla curva, para comparar 1:1. */

import * as THREE from 'three';
import {
  buildHeightfieldMeshData,
  buildLowPolyVegetationMeshData,
  generateIslandHeightfield,
  placeVegetation,
  type IslandHeightfield,
} from '../../../game-core';
import {
  buildBlockPropsMeshData,
  buildBlockTerrainMeshData,
  placeBlockProps,
} from './game-block-mesher';
import { buildBlockHeightmapFromIsland } from './game-procedural-blocks';
import { mountCurvedWater, WATER_MESH_SCALE } from './game-curved-water';
import { toGeometry, toIndexedGeometry } from './game-procedural-geometry';
import { type WorldBend } from './game-world-bend';

/* Misma rejilla que la isla 128A-1 para que el comparador sea 1:1. */
const WIDTH = 48;
const DEPTH = 32;
const MAX_LEVEL = 4;
const WATER_Y = -0.12;
const PROP_COUNT = 60;

export type ProceduralTerrainMode = 'bloques' | 'suave';

export interface ProceduralTerrainStats {
  readonly mode: ProceduralTerrainMode;
  readonly vertices: number;
  readonly triangles: number;
  readonly propCount: number;
}

export interface TerrainPick {
  readonly i: number;
  readonly j: number;
  /** Nivel de bloque; null en modo suave (no hay bloques que mostrar). */
  readonly level: number | null;
  readonly worldX: number;
  readonly worldZ: number;
  readonly height: number;
}

export interface ProceduralComparator {
  readonly setMode: (mode: ProceduralTerrainMode) => void;
  readonly mode: () => ProceduralTerrainMode;
  readonly setVisible: (visible: boolean) => void;
  readonly regenerate: (seed: number) => void;
  readonly groundHeightAt: (x: number, z: number) => number;
  readonly raycastGroup: THREE.Object3D;
  readonly pickTerrain: (x: number, y: number, z: number) => TerrainPick | null;
  readonly setPropsVisible: (visible: boolean) => void;
  readonly terrainStats: () => ProceduralTerrainStats;
  readonly update: (timeSeconds: number, anchorX: number, anchorY: number, anchorZ: number) => void;
  readonly dispose: () => void;
}

interface BuiltMode {
  readonly group: THREE.Group;
  readonly stats: ProceduralTerrainStats;
}

export function mountProceduralComparator(
  scene: THREE.Scene,
  bend: WorldBend,
  toonRamp: THREE.Texture,
  seed = 1337,
  centerX = 0,
  centerZ = 0,
): ProceduralComparator {
  let currentSeed = seed;
  let currentHeightfield: IslandHeightfield;
  let currentBlockLevels: Int8Array;
  let mode: ProceduralTerrainMode = 'bloques';
  let propsVisible = true;

  const world = new THREE.Group();
  const material = bend.apply(new THREE.MeshToonMaterial({ gradientMap: toonRamp, vertexColors: true }));
  /* Agua compartida con la isla (costa/espuma/niebla). Se monta sin anexarla a
   * la escena (addToScene:false) porque vive re-parentada en `world` para que
   * siga el bend del mundo junto al terreno; dispose la retira de su padre. */
  const water = mountCurvedWater(scene, bend, {
    width: WIDTH,
    depth: DEPTH,
    segmentsX: 120,
    segmentsZ: 80,
    meshScale: WATER_MESH_SCALE,
    addToScene: false,
    waterY: WATER_Y,
    centerX,
    centerZ,
    seed,
  });
  water.mesh.position.set(0, WATER_Y, 0);
  world.add(water.mesh);

  let blocks: BuiltMode | null = null;
  let smooth: BuiltMode | null = null;
  let raycastGroup: THREE.Object3D = water.mesh;

  const buildBlocks = (): BuiltMode => {
    currentHeightfield = generateIslandHeightfield({
      seed: currentSeed,
      width: WIDTH,
      depth: DEPTH,
      maxHeight: MAX_LEVEL,
    });
    const blockH = buildBlockHeightmapFromIsland(currentHeightfield, MAX_LEVEL);
    currentBlockLevels = blockH.levels;
    const terrainData = buildBlockTerrainMeshData(blockH, currentSeed);
    const placements = placeBlockProps(blockH, currentSeed, PROP_COUNT);
    const propsData = buildBlockPropsMeshData(placements);
    const group = new THREE.Group();
    const terrain = new THREE.Mesh(toGeometry(terrainData), material);
    const props = new THREE.Mesh(toGeometry(propsData), material);
    props.visible = propsVisible;
    group.add(terrain, props);
    return {
      group,
      stats: {
        mode: 'bloques',
        vertices: terrainData.positions.length / 3,
        triangles: terrainData.positions.length / 9,
        propCount: placements.length,
      },
    };
  };

  const buildSmooth = (): BuiltMode => {
    currentHeightfield = generateIslandHeightfield({
      seed: currentSeed,
      width: WIDTH,
      depth: DEPTH,
      maxHeight: MAX_LEVEL,
    });
    const meshData = buildHeightfieldMeshData(currentHeightfield);
    const veg = placeVegetation(currentHeightfield, currentSeed);
    const propData = buildLowPolyVegetationMeshData(veg.placements);
    const group = new THREE.Group();
    const terrain = new THREE.Mesh(toIndexedGeometry(meshData), material);
    const props = new THREE.Mesh(toIndexedGeometry(propData), material);
    props.visible = propsVisible;
    group.add(terrain, props);
    return {
      group,
      stats: {
        mode: 'suave',
        vertices: meshData.vertexCount,
        triangles: meshData.triangleCount,
        propCount: veg.placements.length,
      },
    };
  };

  const rebuild = (): void => {
    disposeBuiltMode(blocks);
    disposeBuiltMode(smooth);
    blocks = buildBlocks();
    smooth = buildSmooth();
    water.setShore(Float32Array.from(
      currentHeightfield.heights,
      (h) => (h >= currentHeightfield.waterLevel ? 1 : 0),
    ));
    world.add(blocks.group, smooth.group);
    applyMode();
  };

  const applyMode = (): void => {
    if (!blocks || !smooth) return;
    blocks.group.visible = mode === 'bloques';
    smooth.group.visible = mode === 'suave';
    raycastGroup = (mode === 'bloques' ? blocks : smooth).group.children[0];
  };

  const cellAtWorld = (x: number, z: number): { i: number; j: number } | null => {
    const i = Math.floor(x - centerX + WIDTH / 2);
    const j = Math.floor(z - centerZ + DEPTH / 2);
    if (i < 0 || j < 0 || i >= WIDTH || j >= DEPTH) return null;
    return { i, j };
  };

  const cellHeight = (i: number, j: number): number =>
    currentHeightfield.heights[j * WIDTH + i];

  const groundHeightAt = (x: number, z: number): number => {
    const cell = cellAtWorld(x, z);
    if (!cell) return WATER_Y;
    if (mode === 'suave') {
      const y = cellHeight(cell.i, cell.j);
      return y < currentHeightfield.waterLevel ? WATER_Y : y;
    }
    const level = currentBlockLevels[cell.j * WIDTH + cell.i];
    return level < 0 ? WATER_Y : level;
  };

  const pickTerrain = (x: number, y: number, z: number): TerrainPick | null => {
    const cell = cellAtWorld(x, z);
    if (!cell) return null;
    if (mode === 'suave') {
      const height = cellHeight(cell.i, cell.j);
      if (height < currentHeightfield.waterLevel) return null;
      return {
        i: cell.i,
        j: cell.j,
        level: null,
        worldX: cell.i - WIDTH / 2 + 0.5 + centerX,
        worldZ: cell.j - DEPTH / 2 + 0.5 + centerZ,
        height,
      };
    }
    const level = currentBlockLevels[cell.j * WIDTH + cell.i];
    if (level < 0) return null;
    let layer = Math.floor(y + 0.001);
    if (y >= level - 0.001) layer = level - 1;
    layer = Math.max(-1, Math.min(level - 1, layer));
    return {
      i: cell.i,
      j: cell.j,
      level,
      worldX: cell.i - WIDTH / 2 + 0.5 + centerX,
      worldZ: cell.j - DEPTH / 2 + 0.5 + centerZ,
      height: layer + 0.5,
    };
  };

  const setPropsVisible = (visible: boolean): void => {
    propsVisible = visible;
    if (!blocks || !smooth) return;
    for (const built of [blocks, smooth]) {
      const props = built.group.children[1];
      props.visible = visible;
    }
  };

  rebuild();
  world.position.set(centerX, 0, centerZ);
  world.visible = false;
  scene.add(world);

  return {
    setMode: (nextMode) => {
      mode = nextMode;
      applyMode();
    },
    mode: () => mode,
    setVisible: (visible) => {
      world.visible = visible;
      if (visible) applyMode();
    },
    regenerate: (newSeed) => {
      currentSeed = newSeed;
      rebuild();
    },
    groundHeightAt,
    get raycastGroup() {
      return raycastGroup;
    },
    pickTerrain,
    setPropsVisible,
    terrainStats: () => (mode === 'bloques' ? blocks!.stats : smooth!.stats),
    update: (timeSeconds) => water.update(timeSeconds),
    dispose: () => {
      scene.remove(world);
      water.dispose();
      disposeBuiltMode(blocks);
      disposeBuiltMode(smooth);
      material.dispose();
      world.clear();
    },
  };
}

function disposeBuiltMode(built: BuiltMode | null): void {
  if (!built) return;
  built.group.traverse((object) => {
    if (object instanceof THREE.Mesh) object.geometry.dispose();
  });
  built.group.clear();
}
