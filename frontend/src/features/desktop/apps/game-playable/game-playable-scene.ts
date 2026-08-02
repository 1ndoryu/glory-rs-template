/* GAME-01 — Renderer del fixture jugable.
 * Three.js vive detrás de este adaptador: recibe snapshots puros de game-core
 * y no decide movimiento, colisiones ni identidad. El contenido estático se
 * limita a la ventana visible mediante el cache lógico de chunks.
 */

import * as THREE from 'three';
import {
  MapChunkCache,
  type MapVersion,
  type WorldMap,
  type WorldSnapshot,
} from '../../../game-core';
import { createFigure, type ForestMaterials } from '../game-shared/forest-models';
import { FIXTURE_PROPS } from './game-fixture-map';
import { createGamePlayableVisualCache } from './game-playable-visual-cache';
import {
  readAvailableHeapMemory,
  readRendererMetrics,
  type GameRendererMetrics,
} from './game-renderer-metrics';

export interface GamePlayableStreamingStats {
  readonly cacheSize: number;
  readonly visibleChunks: number;
  readonly visibleInstances: number;
  readonly visibleAssets: number;
}

export interface GamePlayableSceneHandle {
  readonly update: (snapshot: WorldSnapshot) => void;
  readonly resize: () => void;
  readonly render: () => void;
  readonly streamingStats: () => GamePlayableStreamingStats;
  readonly rendererMetrics: () => GameRendererMetrics;
  readonly destroy: () => void;
}

const CAMERA_HEIGHT = 15;
const CAMERA_DISTANCE = 13;
const STREAM_HALF_WIDTH = 4;
const STREAM_HALF_DEPTH = 4;

export function mountGamePlayableScene(
  host: HTMLElement,
  map: WorldMap,
  mapVersion: MapVersion,
): GamePlayableSceneHandle {
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0xeeeeea);
  scene.fog = new THREE.Fog(0xeeeeea, 22, 42);

  const camera = new THREE.OrthographicCamera(-10, 10, 8, -8, 0.1, 80);
  const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'low-power' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
  renderer.domElement.setAttribute('aria-label', 'Bosque jugable offline');
  host.appendChild(renderer.domElement);

  const materials: ForestMaterials = {
    ink: new THREE.MeshToonMaterial({ color: 0x111111 }),
    paper: new THREE.MeshToonMaterial({ color: 0xf8f8f4 }),
    pale: new THREE.MeshToonMaterial({ color: 0xd7d7d1 }),
    middle: new THREE.MeshToonMaterial({ color: 0x8d8d88 }),
    water: new THREE.MeshToonMaterial({ color: 0x55555a }),
    lines: new THREE.LineBasicMaterial({ color: 0x050505 }),
  };

  scene.add(new THREE.GridHelper(20, 20, 0x777777, 0xc8c8c2));

  scene.add(new THREE.HemisphereLight(0xffffff, 0x555555, 2.2));
  const sun = new THREE.DirectionalLight(0xffffff, 3.2);
  sun.position.set(-8, 18, 10);
  sun.castShadow = true;
  scene.add(sun);

  const chunkCache = new MapChunkCache(mapVersion);
  const visualCache = createGamePlayableVisualCache({
    scene,
    materials,
    map: mapVersion,
    props: new Map(FIXTURE_PROPS.map(prop => [prop.id, prop])),
  });
  let currentStreamingStats: GamePlayableStreamingStats = {
    cacheSize: 0,
    visibleChunks: 0,
    visibleInstances: 0,
    visibleAssets: 0,
  };
  let currentRendererMetrics: GameRendererMetrics = readRendererMetrics({});

  const streamProps = (center: { x: number; z: number }): void => {
    const visible = chunkCache.select({
      center,
      halfWidth: STREAM_HALF_WIDTH,
      halfDepth: STREAM_HALF_DEPTH,
      marginCells: 0,
    });
    visualCache.sync(visible);
    currentStreamingStats = {
      cacheSize: visible.cacheSize,
      visibleChunks: visible.chunks.length,
      visibleInstances: visible.instances.length,
      visibleAssets: visible.assets.length,
    };
  };

  const entities = new Map<string, THREE.Group>();
  let currentPlayer = { x: 0, z: -0.5 };
  let cameraTarget = new THREE.Vector3(currentPlayer.x, 0, currentPlayer.z);
  let destroyed = false;

  const clampTarget = (target: THREE.Vector3): THREE.Vector3 => {
    const width = Math.max(host.clientWidth, 1);
    const height = Math.max(host.clientHeight, 1);
    const halfHeight = 7.5;
    const halfWidth = halfHeight * width / height;
    return new THREE.Vector3(
      THREE.MathUtils.clamp(target.x, map.bounds.minX + Math.min(halfWidth, 4), map.bounds.maxX - Math.min(halfWidth, 4)),
      0,
      THREE.MathUtils.clamp(target.z, map.bounds.minZ + Math.min(halfHeight, 4), map.bounds.maxZ - Math.min(halfHeight, 4)),
    );
  };

  const updateCamera = (): void => {
    const desired = clampTarget(new THREE.Vector3(currentPlayer.x, 0, currentPlayer.z));
    cameraTarget.lerp(desired, 0.14);
    camera.position.set(
      cameraTarget.x + CAMERA_DISTANCE,
      CAMERA_HEIGHT,
      cameraTarget.z + CAMERA_DISTANCE,
    );
    camera.lookAt(cameraTarget.x, 0, cameraTarget.z);
  };

  const createEntity = (id: string): THREE.Group => {
    const remote = id !== 'local';
    const figure = createFigure(materials, remote);
    figure.userData.entityId = id;
    scene.add(figure);
    entities.set(id, figure);
    return figure;
  };

  const update = (snapshot: WorldSnapshot): void => {
    if (destroyed) return;
    const activeIds = new Set<string>();
    for (const entity of snapshot.entities) {
      const object = entities.get(entity.id) ?? createEntity(entity.id);
      object.position.set(entity.position.x, 0.2, entity.position.z);
      activeIds.add(entity.id);
      if (entity.id === 'local') currentPlayer = entity.position;
    }
    for (const [id, object] of entities) {
      if (activeIds.has(id)) continue;
      scene.remove(object);
      disposeObjectGeometries(object);
      entities.delete(id);
    }
    streamProps(currentPlayer);
    updateCamera();
  };

  const resize = (): void => {
    if (destroyed) return;
    const width = Math.max(host.clientWidth, 1);
    const height = Math.max(host.clientHeight, 1);
    const halfHeight = 8;
    const halfWidth = halfHeight * width / height;
    camera.left = -halfWidth;
    camera.right = halfWidth;
    camera.top = halfHeight;
    camera.bottom = -halfHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height, false);
    updateCamera();
  };

  const render = (): void => {
    if (destroyed) return;
    renderer.render(scene, camera);
    currentRendererMetrics = readRendererMetrics(renderer.info, readAvailableHeapMemory());
  };

  resize();

  return {
    update,
    resize,
    render,
    streamingStats: () => currentStreamingStats,
    rendererMetrics: () => currentRendererMetrics,
    destroy: () => {
      if (destroyed) return;
      destroyed = true;
      visualCache.destroy();
      disposeScene(scene, materials);
      renderer.dispose();
      renderer.forceContextLoss();
      renderer.domElement.remove();
      entities.clear();
    },
  };
}

function disposeObjectGeometries(object: THREE.Object3D): void {
  object.traverse((child) => {
    if (child instanceof THREE.Mesh || child instanceof THREE.LineSegments) {
      child.geometry.dispose();
    }
  });
}

function disposeScene(scene: THREE.Scene, sharedMaterials: ForestMaterials): void {
  const geometries = new Set<THREE.BufferGeometry>();
  const materials = new Set<THREE.Material>(Object.values(sharedMaterials));
  scene.traverse((object) => {
    if (object instanceof THREE.Mesh || object instanceof THREE.LineSegments) {
      geometries.add(object.geometry);
      const assigned = Array.isArray(object.material) ? object.material : [object.material];
      assigned.forEach(material => materials.add(material));
    }
  });
  geometries.forEach(geometry => geometry.dispose());
  materials.forEach(material => material.dispose());
  scene.clear();
}
