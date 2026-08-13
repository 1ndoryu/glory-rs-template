/* GAME-01 — Heightfield de isla del toolkit procedural (138A-1).
 * Un solo generador de altura continua (superelipse + fbm + warp + banda
 * costera) del que se derivan AMBOS estilos del comparador: 'suave' usa la
 * altura tal cual y 'bloques' la cuantiza con relajación de caminabilidad.
 * Misma familia matemática que el experimento 128A-1 para que el comparador
 * compare estilos, no formas: el rect jugable queda siempre en tierra y el
 * océano rodea la isla. Datos puros, sin Three/DOM/red. */

import { fbm2 } from './noise';

export const ISLAND_HEIGHTFIELD_DEFAULTS = {
  maxHeight: 4,
  waterLevel: 0,
  coast: 0.16,
  warp: 0.2,
  octaves: 4,
} as const;

export interface IslandHeightfieldOptions {
  readonly seed: number;
  readonly width: number;
  readonly depth: number;
  readonly maxHeight?: number;
  readonly waterLevel?: number;
  /** Amplitud pico del warp costero (el fbm se desplaza ±warp/2). */
  readonly coast?: number;
  readonly warp?: number;
  readonly octaves?: number;
}

export interface IslandHeightfield {
  readonly width: number;
  readonly depth: number;
  /** Altura por celda `j * width + i` en unidades de mundo (agua = waterLevel). */
  readonly heights: Float32Array;
  readonly waterLevel: number;
  readonly maxHeight: number;
}

/* Exponente de superelipse: 4 redondea las esquinas del rect jugable. */
const ROUND_EXP = 4;
/* Banda de transición costa → interior (misma escala que el experimento). */
const COAST_BAND = 0.22;
/* Divisor del relieve crudo: normaliza picos a 1 sin aplanar el interior. */
const RELIEF_SCALE = 3.2;
/* Fracción de altura mínima del interior: evita que la isla quede plana. */
const LAND_FLOOR = 0.22;
/* Profundidad de fondo marino en el borde de la rejilla. */
const SEAFLOOR_DROP = 1.4;

export function generateIslandHeightfield(options: IslandHeightfieldOptions): IslandHeightfield {
  const { seed, width, depth } = options;
  const maxHeight = options.maxHeight ?? ISLAND_HEIGHTFIELD_DEFAULTS.maxHeight;
  const waterLevel = options.waterLevel ?? ISLAND_HEIGHTFIELD_DEFAULTS.waterLevel;
  const coast = options.coast ?? ISLAND_HEIGHTFIELD_DEFAULTS.coast;
  const warp = options.warp ?? ISLAND_HEIGHTFIELD_DEFAULTS.warp;
  const octaves = options.octaves ?? ISLAND_HEIGHTFIELD_DEFAULTS.octaves;
  if (!Number.isSafeInteger(width) || width < 2 || !Number.isSafeInteger(depth) || depth < 2) {
    throw new Error('dimensiones de heightfield inválidas');
  }
  if (!Number.isFinite(maxHeight) || maxHeight <= 0) throw new Error('maxHeight inválido');
  if (!Number.isFinite(coast) || coast <= 0 || coast >= 0.5) throw new Error('coast inválido');
  /* El desvío pico del fbm es warp/2: debe quedar bajo el umbral de costa para
   * que las esquinas de la rejilla sean siempre océano (isla rodeada de agua). */
  if (!Number.isFinite(warp) || warp < 0 || warp >= coast * 2) throw new Error('warp inválido');

  const heights = new Float32Array(width * depth);
  const cx = (width - 1) / 2;
  const cz = (depth - 1) / 2;
  for (let j = 0; j < depth; j += 1) {
    for (let i = 0; i < width; i += 1) {
      const nx = (i - cx) / (width * 0.5);
      const nz = (j - cz) / (depth * 0.5);
      const d = Math.pow(
        Math.pow(Math.abs(nx), ROUND_EXP) + Math.pow(Math.abs(nz), ROUND_EXP),
        1 / ROUND_EXP,
      );
      const warpV = (fbm2(i * 0.18, j * 0.18, seed, octaves) - 0.5) * warp;
      const mask = 1 - d + warpV;
      let h: number;
      if (mask < coast) {
        /* Fondo marino: se hunde suavemente hacia los bordes, siempre bajo el agua. */
        h = waterLevel - 0.25 - (coast - mask) * SEAFLOOR_DROP;
      } else {
        /* 0 en la costa → 1 en el interior; la banda costera queda de playa. */
        const land = Math.min(1, (mask - coast) / COAST_BAND);
        const e = fbm2(i * 0.17 + 40.2, j * 0.17 + 11.9, seed + 9137, 3);
        const e2 = fbm2(i * 0.42 + 71.2, j * 0.42 + 47.9, seed + 5511, 2);
        const raw = Math.max(0, (e - 0.34) * 5.6 + (e2 - 0.5) * 1.0);
        const n = Math.min(1, raw / RELIEF_SCALE);
        h = waterLevel + land * (LAND_FLOOR + (1 - LAND_FLOOR) * n) * maxHeight;
      }
      heights[j * width + i] = h;
    }
  }
  return { width, depth, heights, waterLevel, maxHeight };
}

/**
 * Cuantiza el heightfield a niveles de bloque: -1 océano, 0 playa, 1..maxLevel
 * hierba. Es la misma base continua → ambos estilos comparten forma y seed.
 */
export function quantizeBlockLevels(h: IslandHeightfield, maxLevel: number): Int8Array {
  if (!Number.isSafeInteger(maxLevel) || maxLevel < 1 || maxLevel > 16) {
    throw new Error('maxLevel fuera de rango');
  }
  const levels = new Int8Array(h.width * h.depth);
  for (let k = 0; k < h.width * h.depth; k += 1) {
    const y = h.heights[k];
    if (y < h.waterLevel) {
      levels[k] = -1;
    } else {
      const t = Math.min(1, Math.max(0, (y - h.waterLevel) / h.maxHeight));
      levels[k] = Math.min(maxLevel, Math.round(t * maxLevel));
    }
  }
  return levels;
}

const NEIGHBORS: readonly (readonly [number, number])[] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

/** Ningún vecino difiere en más de un bloque: la isla queda transitable. */
export function relaxBlockWalkability(
  levels: Int8Array,
  width: number,
  depth: number,
  passes = 8,
): void {
  for (let pass = 0; pass < passes; pass += 1) {
    let changed = false;
    for (let j = 0; j < depth; j += 1) {
      for (let i = 0; i < width; i += 1) {
        const id = j * width + i;
        const h = levels[id];
        if (h < 0) continue;
        let lowest = 99;
        for (const [di, dj] of NEIGHBORS) {
          const ni = i + di;
          const nj = j + dj;
          if (ni < 0 || nj < 0 || ni >= width || nj >= depth) continue;
          const nh = levels[nj * width + ni];
          if (nh < lowest) lowest = nh;
        }
        if (lowest < 99 && h > lowest + 1) {
          levels[id] = lowest + 1;
          changed = true;
        }
      }
    }
    if (!changed) break;
  }
}

/** Elimina islotes de 1-2 celdas aislados en el mar (ruido de costa). */
export function trimLonelyIslands(levels: Int8Array, width: number, depth: number): void {
  for (let j = 0; j < depth; j += 1) {
    for (let i = 0; i < width; i += 1) {
      const id = j * width + i;
      if (levels[id] < 0) continue;
      let land = 0;
      for (const [di, dj] of NEIGHBORS) {
        const ni = i + di;
        const nj = j + dj;
        if (ni < 0 || nj < 0 || ni >= width || nj >= depth) continue;
        if (levels[nj * width + ni] >= 0) land += 1;
      }
      if (land <= 1) levels[id] = -1;
    }
  }
}
