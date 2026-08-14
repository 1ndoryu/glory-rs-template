/* GAME-01 — Agua de la isla curva (138A-3). Adaptador Three delgado sobre
 * buildWaterMeshData del toolkit procedural; extraído de game-curved-island.ts
 * (deuda declarada en el cierre 128A-1): shore mask tierra/océano desenfocada,
 * shader toon de costa con espuma y niebla, y mesh indexado con phase de onda.
 * Solo presentación; no alimenta colisión ni simulación. */

import * as THREE from 'three';
import { buildWaterMeshData } from '../../../game-core';
import { WORLD_BEND_PARS, type WorldBend } from './game-world-bend';
import { BLOCK_COLORS } from './game-block-palette';

/* El plano de agua se extiende más allá del mapa (océano visible como límite);
 * la shore mask conserva el tamaño del mapa (uMapSize) en ambos adaptadores. */
export const WATER_MESH_SCALE = 2.4;

export interface CurvedWaterOptions {
  /** Tamaño del mapa que la shore mask cubre (uMapSize del shader). */
  readonly width: number;
  readonly depth: number;
  /** Divisiones del plano de agua (120×80 en la isla actual). */
  readonly segmentsX?: number;
  readonly segmentsZ?: number;
  /** Escala del mesh respecto al mapa (1 = borde a borde; 2.4 = océano visible). */
  readonly meshScale?: number;
  /** Agrega el mesh a la escena al montar (default true); falso si el
   * consumidor lo re-parenta a otro grupo (comparador: `world`). */
  readonly addToScene?: boolean;
  readonly waterY: number;
  readonly centerX: number;
  readonly centerZ: number;
  readonly seed?: number;
}

export interface CurvedWater {
  readonly mesh: THREE.Mesh;
  /** Regenera la textura de shore: 1 = tierra, 0 = océano, por celda. */
  readonly setShore: (landMask: ArrayLike<number>) => void;
  readonly update: (timeSeconds: number) => void;
  readonly setVisible: (visible: boolean) => void;
  readonly dispose: () => void;
}

/* Shore mask: 1 = tierra, 0 = océano; desenfocada para el banding de espuma.
 * (document.createElement para canvas: excepción aceptada en 128A-1). */
function buildShoreTexture(landMask: ArrayLike<number>, width: number, depth: number): THREE.CanvasTexture {
  let f = new Float32Array(width * depth);
  for (let k = 0; k < width * depth; k += 1) f[k] = landMask[k] > 0 ? 1 : 0;
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

export function mountCurvedWater(
  scene: THREE.Scene,
  bend: WorldBend,
  options: CurvedWaterOptions,
): CurvedWater {
  const { width, depth, waterY, centerX, centerZ } = options;
  const segmentsX = options.segmentsX ?? 120;
  const segmentsZ = options.segmentsZ ?? 80;
  const meshScale = options.meshScale ?? 1;
  const addToScene = options.addToScene ?? true;
  if (!Number.isFinite(meshScale) || meshScale <= 0) {
    throw new Error('escala de agua inválida');
  }
  const meshData = buildWaterMeshData({
    width: width * meshScale,
    depth: depth * meshScale,
    segmentsX,
    segmentsZ,
    seed: options.seed ?? 1337,
  });
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(meshData.positions, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute(meshData.uvs, 2));
  /* El shader actual desplaza por posición de mundo + tiempo; aWavePhase queda
   * disponible para variantes de onda por vértice sin tocar la geometría. */
  geometry.setAttribute('aWavePhase', new THREE.Float32BufferAttribute(meshData.wavePhase, 1));
  geometry.setIndex(new THREE.BufferAttribute(meshData.indices, 1));

  const uniforms = {
    uTime: { value: 0 },
    uShore: { value: buildShoreTexture(new Float32Array(width * depth), width, depth) },
    uMapOrigin: { value: new THREE.Vector2(centerX - width / 2, centerZ - depth / 2) },
    uMapSize: { value: new THREE.Vector2(width, depth) },
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
  const material = new THREE.ShaderMaterial({
    uniforms,
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
  const mesh = new THREE.Mesh(geometry, material);
  mesh.position.set(centerX, waterY, centerZ);
  if (addToScene) scene.add(mesh);

  let shoreTexture: THREE.CanvasTexture | null = uniforms.uShore.value as THREE.CanvasTexture;

  return {
    mesh,
    setShore: (landMask) => {
      const next = buildShoreTexture(landMask, width, depth);
      uniforms.uShore.value = next;
      if (shoreTexture) shoreTexture.dispose();
      shoreTexture = next;
    },
    update: (timeSeconds) => {
      uniforms.uTime.value = timeSeconds;
      const fog = scene.fog as THREE.Fog | null;
      if (fog) {
        uniforms.uFogColor.value.copy(fog.color);
        uniforms.uFogNear.value = fog.near;
        uniforms.uFogFar.value = fog.far;
      }
    },
    setVisible: (visible) => {
      mesh.visible = visible;
    },
    dispose: () => {
      if (mesh.parent) mesh.parent.remove(mesh);
      geometry.dispose();
      material.dispose();
      if (shoreTexture) shoreTexture.dispose();
      shoreTexture = null;
    },
  };
}
