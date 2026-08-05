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
import {
  createGpuFrameProbe,
  estimateGpuMemory,
  readGpuIdentity,
  type GpuFrameProbe,
  type GpuIdentity,
  type GpuMemoryEstimate,
} from './game-gpu-probe';

export interface GamePlayableStreamingStats {
  readonly cacheSize: number;
  readonly visibleChunks: number;
  readonly visibleInstances: number;
  readonly visibleAssets: number;
}

export interface GamePlayableBatchStats {
  readonly drawCalls: number;
  readonly sourceMeshes: number;
}

export interface GamePlayableSceneHandle {
  readonly canvas: HTMLCanvasElement;
  readonly update: (snapshot: WorldSnapshot, localEntityId?: string) => void;
  readonly resize: () => void;
  readonly render: () => void;
  readonly streamingStats: () => GamePlayableStreamingStats;
  readonly rendererMetrics: () => GameRendererMetrics;
  readonly batchStats: () => GamePlayableBatchStats;
  readonly gpuIdentity: () => GpuIdentity | null;
  readonly gpuFrameMs: () => number | null;
  readonly gpuMemoryEstimate: () => GpuMemoryEstimate;
  readonly destroy: () => void;
}

/* Cámara orbital (Genshin): distancia y ángulos controlables. */
const CAMERA_DISTANCE = 16;
const CAMERA_MIN_DISTANCE = 7;
const CAMERA_MAX_DISTANCE = 30;
const CAMERA_MIN_POLAR = 0.35;
const CAMERA_MAX_POLAR = 1.15;
const STREAM_HALF_WIDTH = 4;
const STREAM_HALF_DEPTH = 4;
/* Culling avanzado: radio circular de visibilidad (unidades de mundo) que
 * recorta chunks/instancias en las esquinas de la ventana rectangular. */
const STREAM_MAX_DISTANCE = 26;
/* [GAME-01-VIS] Con cámara orbital libre el borde lejano del frustum cae más
 * allá del jugador cuanto más zoom out; el radio de streaming crece con la
 * distancia de cámara (+18 cubre el extremo horizontal del frustum a FOV 50°)
 * para evitar pop-in al alejar. */
const STREAM_MARGIN_BEYOND_CAMERA = 18;
const FOG_NEAR_MARGIN = 8;
const FOG_FAR_OFFSET = 34;

export function mountGamePlayableScene(
  host: HTMLElement,
  map: WorldMap,
  mapVersion: MapVersion,
): GamePlayableSceneHandle {
  /* [GAME-01-VIS] Dirección aprobada 05-ago: low poly verde stylized con
   * cielo despejado (referencia de estilo tipo Genshin). El contrato de mapa
   * no cambia; solo renderer, paleta y cámara. */
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x87ceeb);
  const fog = new THREE.Fog(0x87ceeb, CAMERA_DISTANCE + FOG_NEAR_MARGIN, CAMERA_DISTANCE + FOG_FAR_OFFSET);
  scene.fog = fog;

  /* [GAME-01-VIS] Cámara libre orbital tipo Genshin: el jugador arrastra
   * para orbitar y usa la rueda/pellizco para acercar; el punto focal sigue
   * al personaje. Sustituye la isométrica fija del boceto. */
  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 120);
  const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'low-power' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
  renderer.domElement.setAttribute('aria-label', 'Bosque jugable offline');
  host.appendChild(renderer.domElement);

  /* Paleta verde stylized: ink = verde profundo (troncos/contorno), middle y
   * paper = follaje en dos verdes, pale = verde claro, water = azul stylized.
   * El contorno se mantiene verde oscuro para no romper la lectura low poly. */
  const materials: ForestMaterials = {
    ink: new THREE.MeshToonMaterial({ color: 0x2f6b2f }),
    paper: new THREE.MeshToonMaterial({ color: 0x7fbf4f }),
    pale: new THREE.MeshToonMaterial({ color: 0xa8d98a }),
    middle: new THREE.MeshToonMaterial({ color: 0x5a9e4b }),
    water: new THREE.MeshToonMaterial({ color: 0x3d8bcd }),
    lines: new THREE.LineBasicMaterial({ color: 0x1e4620 }),
  };

  scene.add(new THREE.GridHelper(20, 20, 0x6f9e3f, 0xb9d99a));

  scene.add(new THREE.HemisphereLight(0xfff7e0, 0x3a6b35, 1.6));
  const sun = new THREE.DirectionalLight(0xfff2c8, 2.4);
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

  /* Probe físico de GPU: identidad, tiempo de frame y memoria estimada. El
   * contexto WebGL real viene del renderer; el probe es opcional y nunca
   * rompe el fixture si la extensión no existe. */
  const gl = renderer.getContext() as unknown as Parameters<typeof createGpuFrameProbe>[0] | null;
  const gpuFrameProbe: GpuFrameProbe = gl
    ? createGpuFrameProbe(gl as Parameters<typeof createGpuFrameProbe>[0])
    : { available: false, beginFrame() {}, endFrame() {}, readFrameMs: () => null, dispose() {} };
  const gpuIdentity: GpuIdentity | null = gl ? readGpuIdentity(gl as Parameters<typeof readGpuIdentity>[0]) : null;
  let lastGpuFrameMs: number | null = null;
  let currentStreamingStats: GamePlayableStreamingStats = {
    cacheSize: 0,
    visibleChunks: 0,
    visibleInstances: 0,
    visibleAssets: 0,
  };
  let currentRendererMetrics: GameRendererMetrics = readRendererMetrics({});

  const streamProps = (center: { x: number; z: number }): void => {
    /* [GAME-01-VIS] Radio adaptativo: nunca por debajo del mínimo y crece con
     * el zoom de la cámara orbital para cubrir el borde lejano del frustum. */
    const maxDistance = Math.max(STREAM_MAX_DISTANCE, orbit.distance + STREAM_MARGIN_BEYOND_CAMERA);
    const visible = chunkCache.select({
      center,
      halfWidth: STREAM_HALF_WIDTH,
      halfDepth: STREAM_HALF_DEPTH,
      marginCells: 0,
      maxDistance,
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
  /* [GAME-01-VIS] Estado orbital: distancia y ángulos que el jugador controla
   * con arrastre (azimuth/polar) y rueda o pellizco (distancia). */
  let orbit = { distance: CAMERA_DISTANCE, azimuth: Math.PI / 4, polar: 0.85 };
  let dragging = false;
  let lastPointer: { x: number; y: number } | null = null;
  let destroyed = false;

  const clampTarget = (target: THREE.Vector3): THREE.Vector3 => {
    const margin = 4;
    return new THREE.Vector3(
      THREE.MathUtils.clamp(target.x, map.bounds.minX + margin, map.bounds.maxX - margin),
      0,
      THREE.MathUtils.clamp(target.z, map.bounds.minZ + margin, map.bounds.maxZ - margin),
    );
  };

  const updateCamera = (): void => {
    const desired = clampTarget(new THREE.Vector3(currentPlayer.x, 0, currentPlayer.z));
    cameraTarget.lerp(desired, 0.14);
    const sinPolar = Math.sin(orbit.polar);
    const offset = new THREE.Vector3(
      orbit.distance * sinPolar * Math.sin(orbit.azimuth),
      orbit.distance * Math.cos(orbit.polar),
      orbit.distance * sinPolar * Math.cos(orbit.azimuth),
    );
    camera.position.copy(cameraTarget).add(offset);
    camera.lookAt(cameraTarget);
    /* Niebla adaptativa: cerca y lejos escalan con el zoom para que la escena
     * nunca se lave a distancia máxima ni se pierda el horizonte a mínimo. */
    fog.near = orbit.distance + FOG_NEAR_MARGIN;
    fog.far = orbit.distance + FOG_FAR_OFFSET;
  };

  /* Arrastre para orbitar: un solo puntero gira; dos dedos (móvil) hacen
   * pinch para zoom. La cámara nunca decide estado de juego. */
  const onOrbitStart = (event: PointerEvent): void => {
    if (event.pointerType === 'mouse' && event.button !== 0) return;
    dragging = true;
    lastPointer = { x: event.clientX, y: event.clientY };
    host.setPointerCapture?.(event.pointerId);
  };
  const onOrbitMove = (event: PointerEvent): void => {
    if (!dragging || !lastPointer) return;
    const dx = event.clientX - lastPointer.x;
    const dy = event.clientY - lastPointer.y;
    lastPointer = { x: event.clientX, y: event.clientY };
    orbit.azimuth -= dx * 0.008;
    orbit.polar = THREE.MathUtils.clamp(orbit.polar + dy * 0.008, CAMERA_MIN_POLAR, CAMERA_MAX_POLAR);
  };
  const onOrbitEnd = (): void => {
    dragging = false;
    lastPointer = null;
  };
  const onWheel = (event: WheelEvent): void => {
    event.preventDefault();
    orbit.distance = THREE.MathUtils.clamp(
      orbit.distance * (event.deltaY > 0 ? 1.08 : 0.92),
      CAMERA_MIN_DISTANCE,
      CAMERA_MAX_DISTANCE,
    );
  };
  host.addEventListener('pointerdown', onOrbitStart);
  host.addEventListener('pointermove', onOrbitMove);
  host.addEventListener('pointerup', onOrbitEnd);
  host.addEventListener('pointercancel', onOrbitEnd);
  host.addEventListener('wheel', onWheel, { passive: false });

  const createEntity = (id: string, characterId: string, localEntityId = 'local'): THREE.Group => {
    const remote = id !== localEntityId;
    /* [297A-77] Cada entidad lleva su personaje del catálogo: el tono se
     * aplica en la figura (material compartido) para que los remotos se vean
     * distintos y el local refleje su elección. */
    const figure = createFigure(materials, remote, characterId);
    figure.userData.entityId = id;
    figure.userData.characterId = characterId;
    scene.add(figure);
    entities.set(id, figure);
    return figure;
  };

  const update = (snapshot: WorldSnapshot, localEntityId = 'local'): void => {
    if (destroyed) return;
    const activeIds = new Set<string>();
    for (const entity of snapshot.entities) {
      const existing = entities.get(entity.id);
      /* Si el personaje cambió (reconexión con otro perfil), recrear la
       * figura para aplicar el tono nuevo. */
      const object = existing && existing.userData.characterId === entity.characterId
        ? existing
        : recreateEntity(entity.id, entity.characterId, existing, localEntityId);
      object.position.set(entity.position.x, 0.2, entity.position.z);
      activeIds.add(entity.id);
      if (entity.id === localEntityId) currentPlayer = entity.position;
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

  const recreateEntity = (
    id: string,
    characterId: string,
    previous: THREE.Group | undefined,
    localEntityId: string,
  ): THREE.Group => {
    if (previous) {
      scene.remove(previous);
      disposeObjectGeometries(previous);
      entities.delete(id);
    }
    return createEntity(id, characterId, localEntityId);
  };

  const resize = (): void => {
    if (destroyed) return;
    const width = Math.max(host.clientWidth, 1);
    const height = Math.max(host.clientHeight, 1);
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height, false);
    updateCamera();
  };

  const render = (): void => {
    if (destroyed) return;
    gpuFrameProbe.beginFrame();
    renderer.render(scene, camera);
    gpuFrameProbe.endFrame();
    const frameMs = gpuFrameProbe.readFrameMs();
    if (frameMs !== null) lastGpuFrameMs = frameMs;
    currentRendererMetrics = readRendererMetrics(renderer.info, readAvailableHeapMemory());
  };

  const estimateGpuSceneMemory = (): GpuMemoryEstimate => {
    const textures: Parameters<typeof estimateGpuMemory>[0] extends readonly (infer T)[] ? T[] : never[] = [];
    const geometries: Parameters<typeof estimateGpuMemory>[1] extends readonly (infer T)[] ? T[] : never[] = [];
    const seenTextures = new Set<THREE.Texture>();
    const seenGeometries = new Set<THREE.BufferGeometry>();
    scene.traverse((object) => {
      if (object instanceof THREE.Mesh || object instanceof THREE.LineSegments) {
        if (!seenGeometries.has(object.geometry)) {
          seenGeometries.add(object.geometry);
          const position = object.geometry.getAttribute('position');
          const vertexCount = position ? position.count : 0;
          const indexCount = object.geometry.index ? object.geometry.index.count : 0;
          geometries.push({ vertexCount: vertexCount + indexCount, bytesPerVertex: 12 });
        }
      }
      if (object instanceof THREE.Mesh || object instanceof THREE.LineSegments
        || object instanceof THREE.InstancedMesh) {
        const assigned = Array.isArray(object.material) ? object.material : [object.material];
        for (const material of assigned) {
          const candidate = (material as THREE.MeshBasicMaterial & { map?: THREE.Texture }).map;
          if (candidate && !seenTextures.has(candidate)) {
            seenTextures.add(candidate);
            const image = candidate.image as { width?: number; height?: number } | undefined;
            textures.push({
              width: image?.width ?? 0,
              height: image?.height ?? 0,
              bytesPerPixel: 4,
            });
          }
        }
      }
    });
    return estimateGpuMemory(textures, geometries);
  };

  resize();

  return {
    canvas: renderer.domElement,
    update,
    resize,
    render,
    streamingStats: () => currentStreamingStats,
    rendererMetrics: () => currentRendererMetrics,
    batchStats: () => ({
      drawCalls: visualCache.batchDrawCallCount(),
      sourceMeshes: visualCache.batchSourceMeshCount(),
    }),
    gpuIdentity: () => gpuIdentity,
    gpuFrameMs: () => lastGpuFrameMs,
    gpuMemoryEstimate: estimateGpuSceneMemory,
    destroy: () => {
      if (destroyed) return;
      destroyed = true;
      host.removeEventListener('pointerdown', onOrbitStart);
      host.removeEventListener('pointermove', onOrbitMove);
      host.removeEventListener('pointerup', onOrbitEnd);
      host.removeEventListener('pointercancel', onOrbitEnd);
      host.removeEventListener('wheel', onWheel);
      gpuFrameProbe.dispose();
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
