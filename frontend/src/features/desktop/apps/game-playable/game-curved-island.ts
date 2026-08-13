/* GAME-01 — Isla del Bosque por bloques (Minecraft), override visual temporal.
 * Adaptador Three delgado sobre los módulos puros (heightmap + mesher): monta
 * las geometrías, el agua toon con shore mask, la lluvia, el highlight de
 * bloque seleccionable y expone controles (lluvia, props, regenerar) para el
 * panel temporal. No alimenta colisión ni simulación: solo presentación. */

import * as THREE from 'three';
import { WORLD_BEND_PARS, type WorldBend } from './game-world-bend';
import {
  cellAt,
  cellCenterX,
  cellCenterZ,
  generateBlockHeightmap,
  levelAt,
  type BlockHeightmap,
} from './game-block-heightmap';
import {
  buildBlockPropsMeshData,
  buildBlockTerrainMeshData,
  placeBlockProps,
  type BlockMeshData,
} from './game-block-mesher';
import {
  BLOCK_COLORS,
} from './game-block-palette';

/* 1 bloque = 1 unidad de mundo. La rejilla es 2:1 para seguir el rect jugable
 * (32×16) y dejar océano visible como límite alrededor de la isla. */
const WIDTH = 48;
const DEPTH = 32;
const MAX_LEVEL = 4;
const WATER_Y = -0.12;
const PROP_COUNT = 60;
const RAIN_MAX = 1100;
const RAIN_AREA = 26;

export interface BlockPick {
  readonly i: number;
  readonly j: number;
  readonly level: number;
  readonly worldX: number;
  readonly worldZ: number;
  readonly blockCenterY: number;
}

export interface CurvedIsland {
  readonly update: (timeSeconds: number, anchorX: number, anchorY: number, anchorZ: number) => void;
  /** Altura de piso (bloques) en un punto del mundo; agua → WATER_Y. */
  readonly groundHeightAt: (x: number, z: number) => number;
  /** Terreno raycastable (los props toon no se seleccionan por bloques). */
  readonly raycastGroup: THREE.Object3D;
  readonly pickBlock: (x: number, y: number, z: number) => BlockPick | null;
  /** [138A-1] Oculta/muestra toda la isla (terreno, agua, lluvia, highlight). */
  readonly setVisible: (visible: boolean) => void;
  readonly setHighlight: (pick: BlockPick | null) => void;
  readonly setRain: (amount: number) => void;
  readonly setPropsVisible: (visible: boolean) => void;
  readonly regenerate: (seed: number) => void;
  readonly dispose: () => void;
}

function toGeometry(data: BlockMeshData): THREE.BufferGeometry {
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(data.positions, 3));
  g.setAttribute('normal', new THREE.Float32BufferAttribute(data.normals, 3));
  g.setAttribute('uv', new THREE.Float32BufferAttribute(data.uvs, 2));
  g.setAttribute('color', new THREE.Float32BufferAttribute(data.colors, 3));
  return g;
}

/* Shore mask: 1 = tierra, 0 = océano; desenfocada para el banding de espuma. */
function buildShoreTexture(levels: Int8Array, width: number, depth: number): THREE.CanvasTexture {
  let f = new Float32Array(width * depth);
  for (let k = 0; k < width * depth; k += 1) f[k] = levels[k] >= 0 ? 1 : 0;
  for (let pass = 0; pass < 3; pass += 1) {
    const g = new Float32Array(width * depth);
    for (let j = 0; j < depth; j += 1) {
      for (let i = 0; i < width; i += 1) {
        let s = 0, n = 0;
        for (let dj = -1; dj <= 1; dj += 1) {
          for (let di = -1; di <= 1; di += 1) {
            const a = i + di, b = j + dj;
            if (a < 0 || b < 0 || a >= width || b >= depth) { n += 1; continue; }
            s += f[b * width + a];
            n += 1;
          }
        }
        g[j * width + i] = s / n;
      }
    }
    f = g;
  }
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = depth;
  const ctx = canvas.getContext('2d')!;
  const img = ctx.createImageData(width, depth);
  for (let j = 0; j < depth; j += 1) {
    for (let i = 0; i < width; i += 1) {
      const v = Math.round(Math.min(1, f[j * width + i]) * 255);
      const o = ((depth - 1 - j) * width + i) * 4;
      img.data[o] = img.data[o + 1] = img.data[o + 2] = v;
      img.data[o + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
  const t = new THREE.CanvasTexture(canvas);
  t.wrapS = t.wrapT = THREE.ClampToEdgeWrapping;
  t.minFilter = t.magFilter = THREE.LinearFilter;
  return t;
}

/* ---------------- lluvia (sigue al jugador) ---------------- */
interface RainRig {
  readonly mesh: THREE.LineSegments;
  readonly material: THREE.ShaderMaterial;
  readonly setAnchor: (x: number, y: number, z: number) => void;
  readonly setTime: (t: number) => void;
}

function buildRain(bend: WorldBend): RainRig {
  const uniforms = {
    uTime: { value: 0 },
    uAnchor: { value: new THREE.Vector3() },
    uArea: { value: RAIN_AREA },
    uTop: { value: 15 },
    uSpan: { value: 21 },
    uBendOrigin: bend.uniforms.uBendOrigin,
    uBendDown: bend.uniforms.uBendDown,
    uBendPull: bend.uniforms.uBendPull,
    uBendClamp: bend.uniforms.uBendClamp,
  };
  const geometry = new THREE.BufferGeometry();
  const position = new Float32Array(RAIN_MAX * 2 * 3);
  const rand = new Float32Array(RAIN_MAX * 2 * 3);
  for (let i = 0; i < RAIN_MAX; i += 1) {
    const a = Math.random() * Math.PI * 2;
    const r = Math.sqrt(Math.random()) * RAIN_AREA;
    const phase = Math.random() * 21;
    for (let k = 0; k < 2; k += 1) {
      const o = (i * 2 + k) * 3;
      position[o] = 0;
      position[o + 1] = k === 0 ? 0 : -0.55;
      position[o + 2] = 0;
      rand[o] = Math.cos(a) * r;
      rand[o + 1] = phase;
      rand[o + 2] = Math.sin(a) * r;
    }
  }
  geometry.setAttribute('position', new THREE.BufferAttribute(position, 3));
  geometry.setAttribute('aRand', new THREE.BufferAttribute(rand, 3));
  const material = new THREE.ShaderMaterial({
    uniforms,
    transparent: true,
    depthWrite: false,
    vertexShader: WORLD_BEND_PARS + `
      attribute vec3 aRand;
      uniform float uTime, uArea, uTop, uSpan;
      uniform vec3 uAnchor;
      varying float vA;
      void main(){
        float y = uAnchor.y + uTop - mod(aRand.y + uTime * 15.0, uSpan);
        vec3 wp = vec3(uAnchor.x + aRand.x, y + position.y, uAnchor.z + aRand.z);
        vA = 1.0 - smoothstep(0.55, 1.0, length(aRand.xz) / uArea);
        vec4 mv = viewMatrix * vec4(applyWorldBend(wp), 1.0);
        gl_Position = projectionMatrix * mv;
      }
    `,
    fragmentShader: `
      precision mediump float;
      varying float vA;
      void main(){ gl_FragColor = vec4(0.93, 0.98, 1.0, vA * 0.34); }
    `,
  });
  const mesh = new THREE.LineSegments(geometry, material);
  mesh.frustumCulled = false;
  return {
    mesh,
    material,
    setAnchor: (x, y, z) => uniforms.uAnchor.value.set(x, y, z),
    setTime: (t) => { uniforms.uTime.value = t; },
  };
}

export function mountCurvedIsland(
  scene: THREE.Scene,
  bend: WorldBend,
  toonRamp: THREE.Texture,
  seed = 1337,
  centerX = 0,
  centerZ = 0,
): CurvedIsland {
  let heightmap: BlockHeightmap = generateBlockHeightmap(seed, WIDTH, DEPTH, MAX_LEVEL);
  const island = new THREE.Group();

  const blockMat = bend.apply(new THREE.MeshToonMaterial({ gradientMap: toonRamp, vertexColors: true }));
  const terrainMesh = new THREE.Mesh(toGeometry(buildBlockTerrainMeshData(heightmap, seed)), blockMat);
  const propsMesh = new THREE.Mesh(toGeometry(buildBlockPropsMeshData(placeBlockProps(heightmap, seed, PROP_COUNT))), blockMat);
  island.add(terrainMesh, propsMesh);
  island.position.set(centerX, 0, centerZ);
  scene.add(island);

  /* Agua: plano rectangular que se extiende más allá de la isla (el océano
   * es el límite); el shore mask produce el banding de espuma en la costa. */
  const waterUniforms = {
    uTime: { value: 0 },
    uShore: { value: buildShoreTexture(heightmap.levels, WIDTH, DEPTH) },
    uMapOrigin: { value: new THREE.Vector2(centerX - WIDTH / 2, centerZ - DEPTH / 2) },
    uMapSize: { value: new THREE.Vector2(WIDTH, DEPTH) },
    uDeep: { value: new THREE.Color(BLOCK_COLORS.waterDeep) },
    uShallow: { value: new THREE.Color(BLOCK_COLORS.waterShallow) },
    uFoam: { value: new THREE.Color(BLOCK_COLORS.foam) },
    uFogColor: { value: new THREE.Color(BLOCK_COLORS.sky) },
    uFogNear: { value: 20 },
    uFogFar: { value: 60 },
    uBendOrigin: bend.uniforms.uBendOrigin,
    uBendDown: bend.uniforms.uBendDown,
    uBendPull: bend.uniforms.uBendPull,
    uBendClamp: bend.uniforms.uBendClamp,
  };
  const waterMat = new THREE.ShaderMaterial({
    uniforms: waterUniforms,
    vertexShader: WORLD_BEND_PARS + `
      uniform float uTime;
      varying vec3 vWorld;
      varying float vFogDepth;
      void main(){
        vec3 wp = (modelMatrix * vec4(position, 1.0)).xyz;
        wp.y += sin(wp.x * 0.42 + uTime * 1.15) * 0.035
              + sin(wp.z * 0.57 - uTime * 0.85) * 0.035;
        vWorld = wp;
        vec4 mv = viewMatrix * vec4(applyWorldBend(wp), 1.0);
        vFogDepth = -mv.z;
        gl_Position = projectionMatrix * mv;
      }
    `,
    fragmentShader: `
      precision highp float;
      uniform sampler2D uShore;
      uniform vec2 uMapOrigin, uMapSize;
      uniform float uTime;
      uniform vec3 uDeep, uShallow, uFoam;
      uniform vec3 uFogColor;
      uniform float uFogNear, uFogFar;
      varying vec3 vWorld;
      varying float vFogDepth;
      void main(){
        vec2 uv = (vWorld.xz - uMapOrigin) / uMapSize;
        float shore = texture2D(uShore, clamp(uv, 0.002, 0.998)).r;
        float wob = sin(vWorld.x * 1.15 + vWorld.z * 0.9 + uTime * 1.30) * 0.030
                  + sin(vWorld.x * 2.70 - vWorld.z * 2.2 - uTime * 0.95) * 0.018;
        float s = shore + wob;
        vec3 col = uDeep;
        col = mix(col, mix(uDeep, uShallow, 0.55), step(0.06, s));
        col = mix(col, uShallow, step(0.20, s));
        float foam = step(0.34, s);
        col = mix(col, uFoam, foam * 0.9);
        float sp = step(0.90, sin(vWorld.x * 0.75 + vWorld.z * 1.6 + uTime * 0.45)) * (1.0 - step(0.10, s));
        col = mix(col, uFoam, sp * 0.30);
        gl_FragColor = vec4(col, 1.0);
        float fogFactor = smoothstep(uFogNear, uFogFar, vFogDepth);
        gl_FragColor.rgb = mix(gl_FragColor.rgb, uFogColor, fogFactor);
      }
    `,
  });
  const water = new THREE.Mesh(
    new THREE.PlaneGeometry(WIDTH * 2.4, DEPTH * 2.4, 120, 80),
    waterMat,
  );
  water.rotation.x = -Math.PI / 2;
  water.position.set(centerX, WATER_Y, centerZ);
  scene.add(water);

  const rain = buildRain(bend);
  scene.add(rain.mesh);
  let rainAmount = 0.6;
  rain.mesh.visible = rainAmount > 0.001;

  /* Highlight del bloque apuntado: relleno translúcido + contorno 1×1×1. */
  const highlightBox = new THREE.BoxGeometry(1.03, 1.03, 1.03);
  const highlight = new THREE.Group();
  highlight.add(
    new THREE.Mesh(highlightBox, new THREE.MeshBasicMaterial({
      color: 0xffffff,
      transparent: true,
      opacity: 0.16,
      depthWrite: false,
    })),
    new THREE.LineSegments(
      new THREE.EdgesGeometry(highlightBox),
      new THREE.LineBasicMaterial({ color: 0x14332a }),
    ),
  );
  highlight.visible = false;
  scene.add(highlight);
  let islandVisible = true;
  let highlightShown = false;

  const groundHeightAt = (x: number, z: number): number => {
    const lvl = levelAt(heightmap, Math.floor(x - centerX + WIDTH / 2), Math.floor(z - centerZ + DEPTH / 2));
    return lvl < 0 ? WATER_Y : lvl;
  };

  const pickBlock = (x: number, y: number, z: number): BlockPick | null => {
    const cell = cellAt(heightmap, x - centerX, z - centerZ);
    if (!cell || cell.level < 0) return null;
    /* Bloque real bajo el cursor: la cara superior cae en [level-1, level];
     * una cara lateral en [floor(y), floor(y)+1]. Nunca un bloque vacío. */
    let layer = Math.floor(y + 0.001);
    if (y >= cell.level - 0.001) layer = cell.level - 1;
    layer = Math.max(-1, Math.min(cell.level - 1, layer));
    return {
      i: cell.i,
      j: cell.j,
      level: cell.level,
      worldX: cellCenterX(heightmap, cell.i) + centerX,
      worldZ: cellCenterZ(heightmap, cell.j) + centerZ,
      blockCenterY: layer + 0.5,
    };
  };

  const setHighlight = (pick: BlockPick | null): void => {
    highlightShown = pick !== null;
    if (!pick) {
      highlight.visible = islandVisible && highlightShown;
      return;
    }
    highlight.visible = islandVisible && highlightShown;
    highlight.position.set(pick.worldX, pick.blockCenterY, pick.worldZ);
  };

  /* [138A-1] El comparador alterna entre esta isla y su vista suave: ocultar
   * TODO el conjunto (incluida agua/lluvia) en vez de solo el grupo de tierra. */
  const setVisible = (visible: boolean): void => {
    islandVisible = visible;
    island.visible = visible;
    water.visible = visible;
    rain.mesh.visible = visible && rainAmount > 0.001;
    highlight.visible = visible && highlightShown;
  };

  const setRain = (amount: number): void => {
    rainAmount = Math.max(0, Math.min(1, amount));
    rain.mesh.visible = rainAmount > 0.001;
  };

  const setPropsVisible = (visible: boolean): void => {
    propsMesh.visible = visible;
  };

  const regenerate = (newSeed: number): void => {
    heightmap = generateBlockHeightmap(newSeed, WIDTH, DEPTH, MAX_LEVEL);
    terrainMesh.geometry.dispose();
    terrainMesh.geometry = toGeometry(buildBlockTerrainMeshData(heightmap, newSeed));
    propsMesh.geometry.dispose();
    propsMesh.geometry = toGeometry(buildBlockPropsMeshData(placeBlockProps(heightmap, newSeed, PROP_COUNT)));
    (waterUniforms.uShore.value as THREE.Texture).dispose();
    waterUniforms.uShore.value = buildShoreTexture(heightmap.levels, WIDTH, DEPTH);
    setHighlight(null);
  };

  return {
    update: (timeSeconds, anchorX, anchorY, anchorZ) => {
      waterUniforms.uTime.value = timeSeconds;
      const fog = scene.fog as THREE.Fog | null;
      if (fog) {
        waterUniforms.uFogColor.value.copy(fog.color);
        waterUniforms.uFogNear.value = fog.near;
        waterUniforms.uFogFar.value = fog.far;
      }
      rain.setTime(timeSeconds);
      rain.setAnchor(anchorX, anchorY, anchorZ);
    },
    groundHeightAt,
    raycastGroup: terrainMesh,
    pickBlock,
    setVisible,
    setHighlight,
    setRain,
    setPropsVisible,
    regenerate,
    dispose: () => {
      scene.remove(island, water, rain.mesh, highlight);
      terrainMesh.geometry.dispose();
      propsMesh.geometry.dispose();
      blockMat.dispose();
      waterMat.dispose();
      (waterUniforms.uShore.value as THREE.Texture).dispose();
      water.geometry.dispose();
      rain.mesh.geometry.dispose();
      rain.material.dispose();
      highlightBox.dispose();
      highlight.traverse((child) => {
        if (child instanceof THREE.LineSegments) child.geometry.dispose();
      });
      island.clear();
    },
  };
}
