/* GAME-01 — Renderer del fixture jugable.
 * Three.js vive detrás de este adaptador: recibe snapshots puros de game-core
 * y no decide movimiento, colisiones ni identidad. El contenido estático se
 * limita a la ventana visible mediante el cache lógico de chunks.
 */

import * as THREE from 'three';
import {
  buildMapVersionFromOptions,
  editMapVersionObjects,
  MapChunkCache,
  mapBuilderStats,
  normalizeWorldPalette,
  normalizeTerrainOptions,
  parseSerializedWorld,
  serializeWorld,
  terrainOptionsPreset,
  WORLD_PALETTE_DEFAULTS,
  type MapBuilderStats,
  type MapEditOp,
  type MapVersion,
  type RenderStyle,
  type TerrainOptions,
  type WorldPalette,
  type WorldMap,
  type WorldSnapshot,
} from '../../../game-core';
import {
  createCurvedFigure,
  createCurvedFigureMaterials,
  type ForestMaterials,
} from '../game-shared/forest-models';
import { createWorldBend } from './game-world-bend';
import { mountCurvedIsland, type BlockPick } from './game-curved-island';
import { mountCurvedIslandPanel } from './game-curved-island-panel';
import {
  CONSTRUCTOR_PANEL_DEFAULT_WIDTH,
  loadConstructorState,
  saveConstructorState,
  type ConstructorPanelState,
} from './game-constructor-persistence';
import {
  attachCameraModeShortcut,
  DEFAULT_CAMERA_MODE,
  type CameraMode,
} from './game-camera-modes';
import {
  mountProceduralComparator,
  type TerrainPick,
} from './game-procedural-comparator';
import { createDebouncedRegenerator } from './game-realtime-debounce';
import { FIXTURE_PROPS } from './game-fixture-map';
import { ASSET_DRAG_MIME } from './game-constructor-assets';
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
  /* [GAME-01-VIS] Azimuth orbital actual para que el runtime convierta el
   * input relativo a cámara en dirección de mundo (teclas tipo Genshin). */
  readonly getCameraAzimuth: () => number;
  /* [128A-1] Follow de cámara conmutable desde el panel temporal. */
  readonly setCameraFollow: (follow: boolean) => void;
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
/* [138A-7] Primera persona: altura de ojos, límites de inclinación y
 * despeje mínimo del suelo para la 3ª persona. */
const CAMERA_EYE_HEIGHT = 1.6;
const CAMERA_PITCH_MIN = -1.2;
const CAMERA_PITCH_MAX = 1.2;
const CAMERA_GROUND_CLEARANCE = 1.1;
/* [GAME-01-VIS] Firmeza del follow de cámara (1/s): la cámara se mantiene
 * pegada al personaje como en un mundo abierto, con suavizado exponencial
 * independiente del framerate (a 60 fps ≈ 18% por frame). */
const CAMERA_FOLLOW_RATE = 12;
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
  /* [138A-8] Fondo reutilizable: la paleta del mundo puede teñir cielo y
   * niebla en tiempo real sin recrear colores en cada cambio. */
  const backgroundColor = new THREE.Color(0xaecfc4);
  scene.background = backgroundColor;
  const fog = new THREE.Fog(0xaecfc4, CAMERA_DISTANCE + FOG_NEAR_MARGIN, CAMERA_DISTANCE + FOG_FAR_OFFSET);
  scene.fog = fog;

  /* [GAME-01-VIS] Cámara libre orbital tipo Genshin: el jugador arrastra
   * para orbitar y usa la rueda/pellizco para acercar; el punto focal sigue
   * al personaje. Sustituye la isométrica fija del boceto. */
  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 120);
  const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'low-power' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
  renderer.domElement.setAttribute('aria-label', 'Bosque jugable offline');
  host.appendChild(renderer.domElement);

  /* Paleta "Curved Island" (referencia visual): arena, roca hueso, agua teal,
   * cielo overcast y toon ramp de 4 bandas. El bending se aplica a todos los
   * materiales para la curva de mundo. */
  const bend = createWorldBend();
  let toonRamp: THREE.Texture = createToonRamp();
  const curved = (color: number): THREE.MeshToonMaterial => bend.apply(
    new THREE.MeshToonMaterial({ color, gradientMap: toonRamp }),
  );
  const materials: ForestMaterials = {
    ink: curved(0xcb9a63),    /* troncos */
    paper: curved(0x93d268),  /* follaje */
    pale: curved(0xf7b845),   /* arena / suelo */
    middle: curved(0xdccfba), /* roca / camino */
    water: curved(0x36a79e),  /* agua profunda */
    lines: new THREE.LineBasicMaterial({ color: 0x2f5d43 }),
  };

  const figureMaterials = createCurvedFigureMaterials();
  for (const material of Object.values(figureMaterials)) {
    bend.apply(material);
    if (material instanceof THREE.MeshToonMaterial) material.gradientMap = toonRamp;
  }

  scene.add(new THREE.HemisphereLight(0xdcefe8, 0xffcf8a, 1.0));
  const sun = new THREE.DirectionalLight(0xfff6e6, 1.2);
  sun.position.set(6, 10, 4);
  scene.add(sun);
  const rim = new THREE.DirectionalLight(0xcfe6ff, 0.4);
  rim.position.set(-6, 4, -5);
  scene.add(rim);

  /* [CURVED-ISLAND] Override temporal del terreno: la isla de la referencia
   * sustituye visualmente los chunks del fixture (sin tocar colisión). Se
   * centra en el punto medio de los bounds del mapa para que la zona jugable
   * quede siempre sobre tierra y no sobre el agua. */
  const islandCenterX = (map.bounds.minX + map.bounds.maxX) / 2;
  const islandCenterZ = (map.bounds.minZ + map.bounds.maxZ) / 2;
  const curvedIsland = mountCurvedIsland(scene, bend, toonRamp, 1337, islandCenterX, islandCenterZ);

  /* [138A-1] Comparador visual del toolkit procedural: montado oculto; el
   * panel lo activa para probar el mismo seed en bloques vs suave. */
  const proceduralComparator = mountProceduralComparator(scene, bend, toonRamp, 1337, islandCenterX, islandCenterZ);
  proceduralComparator.setVisible(false);
  let comparatorVisible = false;
  let comparatorMode: RenderStyle = 'bloques';

  /* [138A-8] Rampa toon global conmutable: reemplaza la textura en todos los
   * materiales toon (isla curva, figura y comparador) sin regenerar mallas.
   * La textura anterior se libera; la nueva la aporta el panel de Textura. */
  const applyToonRamp = (next: THREE.Texture): void => {
    toonRamp.dispose();
    toonRamp = next;
    scene.traverse((object) => {
      if (object instanceof THREE.Mesh && object.material instanceof THREE.MeshToonMaterial) {
        object.material.gradientMap = next;
      }
    });
    for (const material of Object.values(materials)) {
      if (material instanceof THREE.MeshToonMaterial) material.gradientMap = next;
    }
    for (const material of Object.values(figureMaterials)) {
      if (material instanceof THREE.MeshToonMaterial) material.gradientMap = next;
    }
    proceduralComparator.setToonRamp(next);
  };

  /* [128A-1] Follow de cámara conmutable desde el panel temporal. */
  let followPlayer = true;
  /* [138A-4] Estado del constructor: últimas opciones y documento generado. */
  let constructorOptions: TerrainOptions = terrainOptionsPreset('isla');
  /* [138A-8] Documento inicial con el mismo pipeline que el comparador, para
   * que el panel de Assets y el drop tengan instancias desde el primer frame
   * (el comparador lo consume oculto; su generación propia ya coincide). */
  let constructorMap: MapVersion | null = buildMapVersionFromOptions(constructorOptions);
  proceduralComparator.setDocument(constructorMap);
  /* [138A-8] Paleta del mundo y estado de ventana del Constructor (se
   * restauran desde storage en el bloque de restore, más abajo). */
  let constructorPalette: WorldPalette = { ...WORLD_PALETTE_DEFAULTS };
  let constructorPanelState: ConstructorPanelState = {
    collapsed: false,
    side: 'right',
    width: CONSTRUCTOR_PANEL_DEFAULT_WIDTH,
  };

  const formatConstructorStats = (stats: MapBuilderStats): string =>
    `mundo · chunks ${stats.chunks} · instancias ${stats.instances}`
    + ` · árboles ${stats.trees} · rocas ${stats.rocks}`
    + ` · tris ${stats.triangles} · vértices ${stats.vertices}`;

  const downloadWorldJson = (): void => {
    const map = constructorMap ?? buildMapVersionFromOptions(constructorOptions);
    const json = serializeWorld(constructorOptions, map);
    const url = URL.createObjectURL(new Blob([json], { type: 'application/json' }));
    const link = document.createElement('a');
    link.href = url;
    link.download = `bosque-${constructorOptions.shape}-${constructorOptions.seed}.json`;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  };

  /* [138A-6] Solo quedan dos estilos (bloques/suave): seleccionar uno muestra
   * el comparador del constructor; la isla curva queda como referencia
   * histórica inicial, sin selector propio. */
  const applyTerrainMode = (terrainMode: RenderStyle): void => {
    comparatorMode = terrainMode;
    comparatorVisible = true;
    curvedIsland.setVisible(false);
    proceduralComparator.setVisible(true);
    proceduralComparator.setMode(comparatorMode);
    panel.setTerrainMode(terrainMode);
    applyPick(null);
    /* [138A-5][138A-7] El estilo y la cámara se persisten con las opciones. */
    persistConstructorState(terrainMode);
  };

  /* [138A-4] Genera el documento con el pipeline puro y muestra el resultado
   * en el comparador (misma base de opciones para bloques/suave).
   * [138A-5] Al regenerar en tiempo real se conserva el modo visible del
   * comparador en vez de volver a 'bloques' en cada cambio de valor. */
  const showConstructorWorld = (options: TerrainOptions): void => {
    constructorOptions = normalizeTerrainOptions(options);
    constructorMap = buildMapVersionFromOptions(constructorOptions);
    /* [138A-8] El documento es la fuente del comparador: los assets pintados
     * sobreviven a la regeneración (rebuildDocumentProps usa el actual). */
    proceduralComparator.setDocument(constructorMap);
    proceduralComparator.regenerateFromOptions(constructorOptions);
    panel.setConstructorOptions(constructorOptions);
    panel.setConstructorStats(formatConstructorStats(mapBuilderStats(constructorMap)));
    applyTerrainMode(comparatorVisible ? comparatorMode : 'bloques');
  };

  /* [138A-5] Regeneración en vivo: los cambios de controles se agrupan ~200 ms
   * y la última opción gana; se cancela en destroy. */
  const regenerateDebounced = createDebouncedRegenerator<TerrainOptions>(200, (options) => {
    showConstructorWorld(options);
  });
  /* [138A-8] La paleta se aplica con el mismo debounce de 200 ms para no
   * reconstruir mallas ni persistir en cada evento `input` del picker. */
  const paletteDebounced = createDebouncedRegenerator<WorldPalette>(200, (palette) => {
    const next = normalizeWorldPalette(palette);
    constructorPalette = next;
    proceduralComparator.setPalette(next);
    backgroundColor.setHex(next.sky);
    fog.color.copy(backgroundColor);
    persistConstructorState(comparatorMode);
  });

  /* [138A-7] Persiste opciones + estilo + cámara en una sola llamada. */
  const persistConstructorState = (mode: RenderStyle): void => {
    saveConstructorState({
      version: 1,
      options: constructorOptions,
      mode,
      camera: cameraMode,
      palette: constructorPalette,
      panel: constructorPanelState,
    });
  };

  /* [138A-7] Cambio de modo de cámara (panel, atajo C y restauración). Al
   * entrar en primera persona la mirada parte del azimuth orbital para evitar
   * saltos; el panel se sincroniza vía `syncCameraSegment` (asignado tras
   * montarlo, patrón 138A-5) y el modo se persiste con el constructor. */
  let syncCameraSegment: ((mode: CameraMode) => void) | null = null;
  const setCameraMode = (mode: CameraMode): void => {
    cameraMode = mode;
    if (mode === 'primera') {
      look.yaw = orbit.azimuth;
      look.pitch = 0;
    }
    syncCameraSegment?.(mode);
    persistConstructorState(comparatorMode);
  };

  /* [138A-8] Ediciones de objetos (Quitar/Limpiar del panel Assets y drop de
   * instancias): fail-closed con las cuotas y bounds del MapVersion. El panel
   * sincroniza su inventario vía applyMap y el comparador repinta los props. */
  const applyConstructorObjectEdits = (ops: readonly MapEditOp[]): void => {
    try {
      constructorMap = editMapVersionObjects(
        constructorMap ?? buildMapVersionFromOptions(constructorOptions),
        ops,
      );
      proceduralComparator.setDocument(constructorMap);
      panel.setConstructorMap(constructorMap);
      panel.setConstructorStats(formatConstructorStats(mapBuilderStats(constructorMap)));
      persistConstructorState(comparatorMode);
    } catch (error) {
      panel.setConstructorStats(error instanceof Error ? `error: ${error.message}` : 'edición inválida');
    }
  };

  const panel = mountCurvedIslandPanel(host, {
    setCurvature: (down, pull) => bend.setCurvature(down, pull),
    setRain: (amount) => curvedIsland.setRain(amount),
    setPropsVisible: (visible) => {
      curvedIsland.setPropsVisible(visible);
      proceduralComparator.setPropsVisible(visible);
    },
    setCameraFollow: (follow) => { followPlayer = follow; },
    regenerate: () => {
      const newSeed = Math.floor(Math.random() * 99999);
      curvedIsland.regenerate(newSeed);
      proceduralComparator.regenerate(newSeed);
    },
    setTerrainMode: applyTerrainMode,
    setCameraMode,
    worldConstructor: {
      onGenerate: (options) => {
        /* [138A-5] Generar de forma explícita cancela el debounce pendiente
         * para no regenerar dos veces seguidas. */
        regenerateDebounced.cancel();
        showConstructorWorld(options);
      },
      onChange: (options) => regenerateDebounced.schedule(options),
      onExport: () => downloadWorldJson(),
      onImport: (text) => {
        try {
          const world = parseSerializedWorld(text);
          constructorOptions = world.options;
          constructorMap = world.map;
          proceduralComparator.setDocument(constructorMap);
          proceduralComparator.regenerateFromOptions(world.options);
          panel.setConstructorOptions(world.options);
          panel.setConstructorStats(formatConstructorStats(mapBuilderStats(world.map)));
          applyTerrainMode('bloques');
        } catch (error) {
          panel.setConstructorStats(error instanceof Error ? `error: ${error.message}` : 'mundo inválido');
        }
      },
      onPaletteChange: (palette) => {
        paletteDebounced.schedule(palette);
      },
      onEditObjects: applyConstructorObjectEdits,
      onToonRampChange: (dataUrl) => {
        if (dataUrl === null) {
          applyToonRamp(createToonRamp());
          panel.setConstructorStats('rampa restaurada');
          return;
        }
        const image = new Image();
        image.crossOrigin = 'anonymous';
        image.onload = () => {
          try {
            const size = 8;
            const canvas = document.createElement('canvas');
            canvas.width = size;
            canvas.height = 1;
            const context = canvas.getContext('2d');
            if (!context) throw new Error('canvas 2d no disponible');
            /* Muestrea la fila central de la imagen como gradiente toon. */
            const sourceY = Math.floor(image.height / 2);
            context.drawImage(image, 0, sourceY, image.width, 1, 0, 0, size, 1);
            const pixels = context.getImageData(0, 0, size, 1).data;
            const data = new Uint8Array(size * 4);
            for (let i = 0; i < size; i += 1) {
              data[i * 4] = pixels[i * 4];
              data[i * 4 + 1] = pixels[i * 4 + 1];
              data[i * 4 + 2] = pixels[i * 4 + 2];
              data[i * 4 + 3] = 255;
            }
            const ramp = new THREE.DataTexture(data, size, 1, THREE.RGBAFormat);
            ramp.minFilter = THREE.NearestFilter;
            ramp.magFilter = THREE.NearestFilter;
            ramp.generateMipmaps = false;
            ramp.colorSpace = THREE.NoColorSpace;
            ramp.needsUpdate = true;
            applyToonRamp(ramp);
            panel.setConstructorStats('rampa aplicada');
          } catch (error) {
            panel.setConstructorStats(error instanceof DOMException && error.name === 'SecurityError'
              ? 'error: imagen cross-origin sin CORS (usa data: o mismo origen)'
              : error instanceof Error ? `error: ${error.message}` : 'rampa inválida');
          }
        };
        image.onerror = () => panel.setConstructorStats('error: imagen no cargable (revisa CORS o usa data:)');
        image.src = dataUrl;
      },
    },
    initialPalette: constructorPalette,
    initialMap: constructorMap,
    constructorPanelState,
    onConstructorPanelStateChange: (state) => {
      constructorPanelState = state;
      persistConstructorState(comparatorMode);
    },
  });
  syncCameraSegment = (mode) => panel.setCameraMode(mode);

  const chunkCache = new MapChunkCache(mapVersion);
  const visualCache = createGamePlayableVisualCache({
    scene,
    materials,
    map: mapVersion,
    props: new Map(FIXTURE_PROPS.map(prop => [prop.id, prop])),
    hideTerrain: true,
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
  let currentPlayerY = 0;
  let cameraTarget = new THREE.Vector3(currentPlayer.x, 0, currentPlayer.z);
  let lastCameraTime = performance.now();
  let lastRenderTime = performance.now();
  let waterTimeSeconds = 0;
  /* [GAME-01-VIS] Estado orbital: distancia y ángulos que el jugador controla
   * con arrastre (azimuth/polar) y rueda o pellizco (distancia). */
  let orbit = { distance: CAMERA_DISTANCE, azimuth: Math.PI / 4, polar: 0.85 };
  /* [138A-7] Modo de cámara activo y mirada de primera persona (yaw/pitch).
   * `look.yaw` comparte la convención de `rotateInputToWorld`: la cámara
   * mira hacia (-sin(yaw), -cos(yaw)) en X/Z para que W aleje de la cámara. */
  let cameraMode: CameraMode = DEFAULT_CAMERA_MODE;
  let look = { yaw: Math.PI / 4, pitch: 0 };
  let dragging = false;
  let lastPointer: { x: number; y: number } | null = null;
  let destroyed = false;

  const clampTarget = (target: THREE.Vector3): THREE.Vector3 => {
    const margin = 4;
    return new THREE.Vector3(
      THREE.MathUtils.clamp(target.x, map.bounds.minX + margin, map.bounds.maxX - margin),
      target.y,
      THREE.MathUtils.clamp(target.z, map.bounds.minZ + margin, map.bounds.maxZ - margin),
    );
  };

  const updateCamera = (): void => {
    /* [GAME-01-VIS] Suavizado exponencial con delta real: el follow de cámara
     * se siente igual a 30, 60 o 120 fps y nunca se despega del personaje. */
    const now = performance.now();
    const dt = Math.min(Math.max((now - lastCameraTime) / 1000, 0), 0.1);
    lastCameraTime = now;
    if (followPlayer) {
      const desired = clampTarget(new THREE.Vector3(currentPlayer.x, currentPlayerY + 0.8, currentPlayer.z));
      cameraTarget.lerp(desired, 1 - Math.exp(-CAMERA_FOLLOW_RATE * dt));
    }
    /* [138A-7] Primera persona: la cámara está en los ojos del personaje y
     * el arrastre mueve la mirada (look.yaw/pitch). Sin zoom: niebla fija en
     * la distancia orbital por defecto. */
    if (cameraMode === 'primera') {
      const eye = new THREE.Vector3(currentPlayer.x, currentPlayerY + CAMERA_EYE_HEIGHT, currentPlayer.z);
      const dir = new THREE.Vector3(
        -Math.sin(look.yaw) * Math.cos(look.pitch),
        Math.sin(look.pitch),
        -Math.cos(look.yaw) * Math.cos(look.pitch),
      );
      camera.position.copy(eye);
      camera.lookAt(eye.add(dir));
      fog.near = CAMERA_DISTANCE + FOG_NEAR_MARGIN;
      fog.far = CAMERA_DISTANCE + FOG_FAR_OFFSET;
      return;
    }
    const sinPolar = Math.sin(orbit.polar);
    const offset = new THREE.Vector3(
      orbit.distance * sinPolar * Math.sin(orbit.azimuth),
      orbit.distance * Math.cos(orbit.polar),
      orbit.distance * sinPolar * Math.cos(orbit.azimuth),
    );
    camera.position.copy(cameraTarget).add(offset);
    /* [138A-7] 3ª persona: la órbita sigue al personaje y no se hunde en el
     * terreno (colisión básica con la altura del suelo). */
    if (cameraMode === 'tercera') {
      camera.position.y = Math.max(
        camera.position.y,
        groundHeightAt(camera.position.x, camera.position.z) + CAMERA_GROUND_CLEARANCE,
      );
    }
    camera.lookAt(cameraTarget);
    /* Niebla adaptativa: cerca y lejos escalan con el zoom para que la escena
     * nunca se lave a distancia máxima ni se pierda el horizonte a mínimo. */
    fog.near = orbit.distance + FOG_NEAR_MARGIN;
    fog.far = orbit.distance + FOG_FAR_OFFSET;
  };

  /* Arrastre para orbitar: un solo puntero gira; dos dedos (móvil) hacen
   * pinch para zoom. La cámara nunca decide estado de juego. */
  /* [128A-1] Bloque seleccionable: raycast al terreno/props en hover (sin
   * interferir con el arrastre de órbita) y highlight del bloque apuntado. */
  const raycaster = new THREE.Raycaster();
  const pointerNdc = new THREE.Vector2();
  /* [138A-1] Normaliza el pick de la isla (BlockPick) y del comparador
   * (TerrainPick) a un mismo contrato de panel; el highlight de bloques solo
   * aplica a la isla 128A-1 visible. */
  const applyPick = (pick: TerrainPick | BlockPick | null): void => {
    if (!pick) {
      curvedIsland.setHighlight(null);
      panel.setPick(null);
      return;
    }
    if (!comparatorVisible && pick.level !== null) {
      curvedIsland.setHighlight(pick as BlockPick);
    } else {
      curvedIsland.setHighlight(null);
    }
    panel.setPick({ i: pick.i, j: pick.j, level: pick.level });
  };
  const updatePick = (clientX: number, clientY: number): void => {
    const rect = host.getBoundingClientRect();
    pointerNdc.set(
      ((clientX - rect.left) / Math.max(rect.width, 1)) * 2 - 1,
      -((clientY - rect.top) / Math.max(rect.height, 1)) * 2 + 1,
    );
    raycaster.setFromCamera(pointerNdc, camera);
    const activeGroup = comparatorVisible ? proceduralComparator.raycastGroup : curvedIsland.raycastGroup;
    const hits = raycaster.intersectObject(activeGroup, true);
    const hit = hits[0];
    applyPick(hit
      ? comparatorVisible
        ? proceduralComparator.pickTerrain(hit.point.x, hit.point.y, hit.point.z)
        : curvedIsland.pickBlock(hit.point.x, hit.point.y, hit.point.z)
      : null);
  };

  /* [138A-5] Restaura las últimas opciones y modo al recargar (fail-closed).
   * Debe correr después de `applyPick` (la generación lo invoca al aplicar
   * el modo) y antes de conectar el input de órbita. */
  const restored = loadConstructorState();
  if (restored) {
    cameraMode = restored.camera;
    if (restored.palette) constructorPalette = normalizeWorldPalette(restored.palette);
    if (restored.panel) constructorPanelState = { ...restored.panel };
    showConstructorWorld(restored.options);
    if (restored.mode !== 'bloques') applyTerrainMode(restored.mode);
    /* [138A-8] Restaura documento, paleta y ventana en los subpaneles del
     * Constructor y reaplica los colores de escena (cielo/niebla). */
    panel.setConstructorMap(constructorMap);
    panel.setConstructorPalette(constructorPalette);
    panel.setConstructorPanelState(constructorPanelState);
    proceduralComparator.setPalette(constructorPalette);
    backgroundColor.setHex(constructorPalette.sky);
    fog.color.copy(backgroundColor);
  }
  /* [138A-7] Sincroniza el segmento de cámara del panel y la mirada de
   * primera persona con el modo restaurado (o el default `libre`). */
  setCameraMode(cameraMode);

  const groundHeightAt = (x: number, z: number): number =>
    comparatorVisible
      ? proceduralComparator.groundHeightAt(x, z)
      : curvedIsland.groundHeightAt(x, z);

  const onOrbitStart = (event: PointerEvent): void => {
    if (event.pointerType === 'mouse' && event.button !== 0) return;
    dragging = true;
    lastPointer = { x: event.clientX, y: event.clientY };
    host.setPointerCapture?.(event.pointerId);
  };
  const onOrbitMove = (event: PointerEvent): void => {
    if (dragging && lastPointer) {
      const dx = event.clientX - lastPointer.x;
      const dy = event.clientY - lastPointer.y;
      lastPointer = { x: event.clientX, y: event.clientY };
      /* [138A-7] En primera persona el arrastre gira la mirada; en libre y
       * 3ª persona orbita la cámara alrededor del personaje. */
      if (cameraMode === 'primera') {
        look.yaw -= dx * 0.008;
        look.pitch = THREE.MathUtils.clamp(look.pitch + dy * 0.008, CAMERA_PITCH_MIN, CAMERA_PITCH_MAX);
        return;
      }
      orbit.azimuth -= dx * 0.008;
      orbit.polar = THREE.MathUtils.clamp(orbit.polar + dy * 0.008, CAMERA_MIN_POLAR, CAMERA_MAX_POLAR);
      return;
    }
    updatePick(event.clientX, event.clientY);
  };
  const onOrbitEnd = (): void => {
    dragging = false;
    lastPointer = null;
  };
  const onPointerLeave = (): void => {
    applyPick(null);
  };
  const onWheel = (event: WheelEvent): void => {
    event.preventDefault();
    /* [138A-7] En primera persona no hay zoom orbital; la rueda no cambia
     * distancia (solo se consume para no hacer scroll de la página). */
    if (cameraMode === 'primera') return;
    orbit.distance = THREE.MathUtils.clamp(
      orbit.distance * (event.deltaY > 0 ? 1.08 : 0.92),
      CAMERA_MIN_DISTANCE,
      CAMERA_MAX_DISTANCE,
    );
  };
  /* [138A-7] Atajo C para alternar libre → primera → 3ª persona; se
   * desmonta en destroy junto con el resto de listeners. */
  const stopCameraShortcut = attachCameraModeShortcut(() => cameraMode, setCameraMode);
  host.addEventListener('pointerdown', onOrbitStart);
  host.addEventListener('pointermove', onOrbitMove);
  host.addEventListener('pointerup', onOrbitEnd);
  host.addEventListener('pointercancel', onOrbitEnd);
  host.addEventListener('pointerleave', onPointerLeave);
  host.addEventListener('wheel', onWheel, { passive: false });

  /* [138A-8] Drop de assets del panel Assets al mundo: el drag viaja con el
   * asset id y el drop resuelve la celda por raycast sobre el terreno visible
   * (comparador o isla curva) antes de colocar la instancia en el documento. */
  const onDragOver = (event: DragEvent): void => {
    if (event.dataTransfer?.types.includes(ASSET_DRAG_MIME)) {
      event.preventDefault();
      event.dataTransfer.dropEffect = 'copy';
    }
  };
  const onDropAsset = (event: DragEvent): void => {
    const assetVersionId = event.dataTransfer?.getData(ASSET_DRAG_MIME);
    if (!assetVersionId) return;
    event.preventDefault();
    const rect = host.getBoundingClientRect();
    pointerNdc.set(
      ((event.clientX - rect.left) / Math.max(rect.width, 1)) * 2 - 1,
      -((event.clientY - rect.top) / Math.max(rect.height, 1)) * 2 + 1,
    );
    raycaster.setFromCamera(pointerNdc, camera);
    const activeGroup = comparatorVisible ? proceduralComparator.raycastGroup : curvedIsland.raycastGroup;
    const hit = raycaster.intersectObject(activeGroup, true)[0];
    if (!hit) return;
    const pick = comparatorVisible
      ? proceduralComparator.pickTerrain(hit.point.x, hit.point.y, hit.point.z)
      : curvedIsland.pickBlock(hit.point.x, hit.point.y, hit.point.z);
    if (!pick) return;
    /* El documento vive en el frame local (bounds ±w/2·cellSize); el pick
     * entrega coordenadas de escena, así que se restan los centros. */
    applyConstructorObjectEdits([{
      kind: 'add',
      assetVersionId,
      position: { x: pick.worldX - islandCenterX, z: pick.worldZ - islandCenterZ },
    }]);
  };
  host.addEventListener('dragover', onDragOver);
  host.addEventListener('drop', onDropAsset);

  const createEntity = (id: string, characterId: string, localEntityId = 'local'): THREE.Group => {
    const remote = id !== localEntityId;
    /* [297A-77] Cada entidad lleva su personaje del catálogo: el tono se
     * aplica en la figura (material compartido) para que los remotos se vean
     * distintos y el local refleje su elección. */
    const figure = createCurvedFigure(figureMaterials, remote, characterId);
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
      const groundY = groundHeightAt(entity.position.x, entity.position.z);
      object.position.set(entity.position.x, groundY + 0.2, entity.position.z);
      /* [CURVED-ISLAND] El personaje mira hacia su dirección de movimiento
       * (el runtime ya la expresa en espacio mundo, relativa a cámara). */
      if (Math.hypot(entity.velocity.x, entity.velocity.z) > 0.001) {
        const target = Math.atan2(entity.velocity.x, entity.velocity.z);
        const current = typeof object.userData.yaw === 'number' ? object.userData.yaw : target;
        let diff = target - current;
        while (diff > Math.PI) diff -= Math.PI * 2;
        while (diff < -Math.PI) diff += Math.PI * 2;
        object.userData.yaw = current + diff * 0.6;
        object.rotation.y = object.userData.yaw;
      }
      activeIds.add(entity.id);
      if (entity.id === localEntityId) {
        currentPlayer = entity.position;
        currentPlayerY = groundY;
      }
    }
    for (const [id, object] of entities) {
      if (activeIds.has(id)) continue;
      scene.remove(object);
      disposeObjectGeometries(object);
      entities.delete(id);
    }
    streamProps(currentPlayer);
    bend.setOrigin(currentPlayer.x, currentPlayerY, currentPlayer.z);
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
    const now = performance.now();
    const frameDt = Math.min((now - lastRenderTime) / 1000, 0.1);
    lastRenderTime = now;
    waterTimeSeconds += frameDt;
    curvedIsland.update(waterTimeSeconds, currentPlayer.x, currentPlayerY, currentPlayer.z);
    proceduralComparator.update(waterTimeSeconds, currentPlayer.x, currentPlayerY, currentPlayer.z);
    bend.setOrigin(currentPlayer.x, currentPlayerY, currentPlayer.z);
    gpuFrameProbe.beginFrame();
    renderer.render(scene, camera);
    gpuFrameProbe.endFrame();
    const frameMs = gpuFrameProbe.readFrameMs();
    if (frameMs !== null) lastGpuFrameMs = frameMs;
    currentRendererMetrics = readRendererMetrics(renderer.info, readAvailableHeapMemory());
    if (comparatorVisible) {
      const stats = proceduralComparator.terrainStats();
      const frameText = lastGpuFrameMs !== null ? `${lastGpuFrameMs.toFixed(1)}ms` : '—';
      panel.setTerrainMetrics(
        `${stats.mode} · tris ${stats.triangles} · vértices ${stats.vertices} · props ${stats.propCount}`
        + ` · draw calls ${currentRendererMetrics.drawCalls} · frame ${frameText}`,
      );
    }
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
    getCameraAzimuth: () => cameraMode === 'primera' ? look.yaw : orbit.azimuth,
    setCameraFollow: (follow: boolean): void => { followPlayer = follow; },
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
      host.removeEventListener('pointerleave', onPointerLeave);
      host.removeEventListener('wheel', onWheel);
      host.removeEventListener('dragover', onDragOver);
      host.removeEventListener('drop', onDropAsset);
      stopCameraShortcut();
      regenerateDebounced.dispose();
      paletteDebounced.dispose();
      panel.destroy();
      gpuFrameProbe.dispose();
      visualCache.destroy();
      proceduralComparator.dispose();
      curvedIsland.dispose();
      disposeScene(scene, materials, Object.values(figureMaterials));
      toonRamp.dispose();
      renderer.dispose();
      renderer.forceContextLoss();
      renderer.domElement.remove();
      entities.clear();
    },
  };
}

/* Rampa toon de 4 bandas compartida por todos los materiales lit (arena,
 * roca, agua, follaje y figura). El dato vive en espacio lineal: no debe
 * pasar por gestión de color. */
function createToonRamp(): THREE.DataTexture {
  const steps = [0.58, 0.75, 0.89, 1.0];
  const data = new Uint8Array(steps.length * 4);
  steps.forEach((value, index) => {
    const band = Math.round(value * 255);
    data[index * 4] = band;
    data[index * 4 + 1] = band;
    data[index * 4 + 2] = band;
    data[index * 4 + 3] = 255;
  });
  const texture = new THREE.DataTexture(data, steps.length, 1, THREE.RGBAFormat);
  texture.minFilter = THREE.NearestFilter;
  texture.magFilter = THREE.NearestFilter;
  texture.generateMipmaps = false;
  texture.colorSpace = THREE.NoColorSpace;
  texture.needsUpdate = true;
  return texture;
}

function disposeObjectGeometries(object: THREE.Object3D): void {
  object.traverse((child) => {
    if (child instanceof THREE.Mesh || child instanceof THREE.LineSegments) {
      child.geometry.dispose();
    }
  });
}

function disposeScene(
  scene: THREE.Scene,
  sharedMaterials: ForestMaterials,
  extraMaterials: readonly THREE.Material[] = [],
): void {
  const geometries = new Set<THREE.BufferGeometry>();
  const materials = new Set<THREE.Material>([...Object.values(sharedMaterials), ...extraMaterials]);
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
