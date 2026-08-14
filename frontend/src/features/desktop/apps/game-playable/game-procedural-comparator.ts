/* GAME-01 — Comparador visual del toolkit procedural (138A-1/138A-2, 138A-4).
 * Monta la misma base de altura en dos estilos ('bloques' reutiliza el mesher
 * 128A-1 vía adaptador; 'suave' usa heightfield-mesh + vegetación low-poly).
 * Desde 138A-4 acepta `TerrainOptions` completas para comparar estilos sobre
 * el MISMO mundo; sin opciones mantiene la isla clásica 48×32. Solo
 * presentación y métricas: el agua es un plano toon simple. */

import * as THREE from 'three';
import {
  buildHeightfieldMeshData,
  buildLowPolyVegetationMeshData,
  generateTerrainHeightfield,
  normalizeTerrainOptions,
  placeVegetation,
  terrainOptionsPreset,
  type IslandHeightfield,
  type RenderStyle,
  type TerrainOptions,
} from '../../../game-core';
import {
  buildBlockPropsMeshData,
  buildBlockTerrainMeshData,
  placeBlockProps,
} from './game-block-mesher';
import { buildBlockHeightmapFromIsland } from './game-procedural-blocks';
import { toGeometry, toIndexedGeometry } from './game-procedural-geometry';
import { buildToonWaterPlane, buildToonWaterPlaneGeometry } from './game-toon-water';
import { type WorldBend } from './game-world-bend';

const WATER_Y = -0.12;
const PROP_COUNT = 60;

export interface ProceduralTerrainStats {
  readonly mode: RenderStyle;
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
  readonly setMode: (mode: RenderStyle) => void;
  readonly mode: () => RenderStyle;
  readonly setVisible: (visible: boolean) => void;
  readonly regenerate: (seed: number) => void;
  /** [138A-4] Regenera con opciones completas del constructor. */
  readonly regenerateFromOptions: (options: TerrainOptions) => void;
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
  options?: TerrainOptions,
): ProceduralComparator {
  let currentOptions = options === undefined
    ? { ...terrainOptionsPreset('isla'), seed }
    : normalizeTerrainOptions({ ...options, seed: options.seed });
  let currentWidth = currentOptions.width;
  let currentDepth = currentOptions.depth;
  let currentHeightfield: IslandHeightfield;
  let currentBlockLevels: Int8Array;
  let mode: RenderStyle = 'bloques';
  let propsVisible = true;

  const world = new THREE.Group();
  const material = bend.apply(new THREE.MeshToonMaterial({ gradientMap: toonRamp, vertexColors: true }));
  /* El material del agua se crea UNA vez aquí (geometría placeholder) y cada
   * rebuild solo regenera la geometría; crear un material por montaje o por
   * regeneración filtraría recursos GPU sin liberar. */
  const initialWater = buildToonWaterPlane(bend, 1, 1, toonRamp);
  let waterGeometry = initialWater.geometry;
  const waterMaterial = initialWater.material;
  const water = new THREE.Mesh(waterGeometry, waterMaterial);
  water.position.y = WATER_Y;
  /* El agua queda por encima del fondo marino sin pelear en z en el borde. */
  water.renderOrder = 1;
  world.add(water);

  let blocks: BuiltMode | null = null;
  let smooth: BuiltMode | null = null;
  let raycastGroup: THREE.Object3D = water;

  const rebuildWater = (): void => {
    waterGeometry?.dispose();
    /* [138A-6] El agua cubre el rect del mundo escalado por cellSize. */
    const cellSize = currentOptions.cellSize;
    waterGeometry = buildToonWaterPlaneGeometry(currentWidth * cellSize * 2.4, currentDepth * cellSize * 2.4);
    water.geometry = waterGeometry;
  };

  const blockMaxLevel = (): number =>
    Math.min(16, Math.max(1, Math.round(currentOptions.maxHeight)));

  const buildBlocks = (heightfield: IslandHeightfield): BuiltMode => {
    const blockH = buildBlockHeightmapFromIsland(heightfield, blockMaxLevel());
    currentBlockLevels = blockH.levels;
    const terrainData = buildBlockTerrainMeshData(blockH, currentOptions.seed);
    const placements = placeBlockProps(blockH, currentOptions.seed, PROP_COUNT);
    const propsData = buildBlockPropsMeshData(placements);
    const group = new THREE.Group();
    const terrain = new THREE.Mesh(toGeometry(terrainData), material);
    const props = new THREE.Mesh(toGeometry(propsData), material);
    props.visible = propsVisible;
    /* [138A-6] El tamaño de bloque real: el mesher emite celdas de 1 unidad y
     * el grupo escala la huella x/z por cellSize (la altura no se escala:
     * maxHeight es un control independiente en el contrato). */
    group.scale.set(currentOptions.cellSize, 1, currentOptions.cellSize);
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

  const buildSmooth = (heightfield: IslandHeightfield): BuiltMode => {
    const cellSize = currentOptions.cellSize;
    const meshData = buildHeightfieldMeshData(heightfield, { cellSize });
    const density = currentOptions.vegetationDensity;
    const veg = placeVegetation(heightfield, currentOptions.seed, {
      maxGrass: Math.round(420 * density),
      /* [138A-6] Sin árboles en suave: conserva césped y rocas. */
      maxTrees: 0,
      maxRocks: Math.round(26 * density),
    });
    /* [138A-6] Las posiciones del toolkit están en celdas; el preview suave
     * las traduce al mundo escalado por cellSize igual que el documento. */
    const scaledPlacements = veg.placements.map(placement => ({
      ...placement,
      x: placement.x * cellSize,
      z: placement.z * cellSize,
    }));
    const propData = buildLowPolyVegetationMeshData(scaledPlacements);
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
    /* Un único heightfield por rebuild: bloques y suave comparten la MISMA
     * base exacta y la generación no se ejecuta dos veces por clic. */
    currentHeightfield = generateTerrainHeightfield(currentOptions);
    blocks = buildBlocks(currentHeightfield);
    smooth = buildSmooth(currentHeightfield);
    world.add(blocks.group, smooth.group);
    applyMode();
  };

  const setOptions = (next: TerrainOptions): void => {
    currentOptions = normalizeTerrainOptions(next);
    currentWidth = currentOptions.width;
    currentDepth = currentOptions.depth;
    rebuildWater();
    rebuild();
  };

  const applyMode = (): void => {
    if (!blocks || !smooth) return;
    blocks.group.visible = mode === 'bloques';
    smooth.group.visible = mode === 'suave';
    raycastGroup = (mode === 'bloques' ? blocks : smooth).group.children[0];
  };

  const cellAtWorld = (x: number, z: number): { i: number; j: number } | null => {
    /* [138A-6] El mundo del comparador escala por cellSize (bloques via scale
     * del grupo, suave via posiciones del mesh); el pick divide por cellSize. */
    const i = Math.floor((x - centerX) / currentOptions.cellSize + currentWidth / 2);
    const j = Math.floor((z - centerZ) / currentOptions.cellSize + currentDepth / 2);
    if (i < 0 || j < 0 || i >= currentWidth || j >= currentDepth) return null;
    return { i, j };
  };

  const cellHeight = (i: number, j: number): number =>
    currentHeightfield.heights[j * currentWidth + i];

  const groundHeightAt = (x: number, z: number): number => {
    const cell = cellAtWorld(x, z);
    if (!cell) return WATER_Y;
    if (mode === 'suave') {
      const y = cellHeight(cell.i, cell.j);
      return y < currentHeightfield.waterLevel ? WATER_Y : y;
    }
    const level = currentBlockLevels[cell.j * currentWidth + cell.i];
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
        worldX: (cell.i - currentWidth / 2 + 0.5) * currentOptions.cellSize + centerX,
        worldZ: (cell.j - currentDepth / 2 + 0.5) * currentOptions.cellSize + centerZ,
        height,
      };
    }
    const level = currentBlockLevels[cell.j * currentWidth + cell.i];
    if (level < 0) return null;
    let layer = Math.floor(y + 0.001);
    if (y >= level - 0.001) layer = level - 1;
    layer = Math.max(-1, Math.min(level - 1, layer));
    return {
      i: cell.i,
      j: cell.j,
      level,
      worldX: (cell.i - currentWidth / 2 + 0.5) * currentOptions.cellSize + centerX,
      worldZ: (cell.j - currentDepth / 2 + 0.5) * currentOptions.cellSize + centerZ,
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

  setOptions(currentOptions);
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
      setOptions({ ...currentOptions, seed: newSeed });
    },
    regenerateFromOptions: (next) => {
      setOptions(next);
    },
    groundHeightAt,
    get raycastGroup() {
      return raycastGroup;
    },
    pickTerrain,
    setPropsVisible,
    terrainStats: () => (mode === 'bloques' ? blocks!.stats : smooth!.stats),
    /* Agua estática: el update existe solo por el contrato común con la isla. */
    update: () => {},
    dispose: () => {
      scene.remove(world);
      disposeBuiltMode(blocks);
      disposeBuiltMode(smooth);
      material.dispose();
      waterMaterial.dispose();
      waterGeometry.dispose();
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
