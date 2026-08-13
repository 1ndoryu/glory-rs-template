import { describe, expect, it } from 'vitest';
import { buildHeightfieldMeshData } from './heightfield-mesh';
import { generateIslandHeightfield } from './heightmap';

const WIDTH = 48;
const DEPTH = 32;

describe('heightfield-mesh (138A-1)', () => {
  const h = generateIslandHeightfield({ seed: 1337, width: WIDTH, depth: DEPTH, maxHeight: 4 });

  it('emite un vértice por celda y triángulos indexados del grid', () => {
    const m = buildHeightfieldMeshData(h);
    expect(m.vertexCount).toBe(WIDTH * DEPTH);
    expect(m.triangleCount).toBe((WIDTH - 1) * (DEPTH - 1) * 2);
    expect(m.positions.length).toBe(WIDTH * DEPTH * 3);
    expect(m.normals.length).toBe(WIDTH * DEPTH * 3);
    expect(m.uvs.length).toBe(WIDTH * DEPTH * 2);
    expect(m.colors.length).toBe(WIDTH * DEPTH * 3);
    expect(m.indices.length).toBe((WIDTH - 1) * (DEPTH - 1) * 6);
    for (const idx of m.indices) {
      expect(idx).toBeLessThan(m.vertexCount);
    }
  });

  it('las posiciones copian la altura del heightfield y las normales son unitarias', () => {
    const m = buildHeightfieldMeshData(h);
    for (let k = 0; k < WIDTH * DEPTH; k += 1) {
      expect(m.positions[k * 3 + 1]).toBe(h.heights[k]);
      const n = Math.hypot(m.normals[k * 3], m.normals[k * 3 + 1], m.normals[k * 3 + 2]);
      expect(n).toBeCloseTo(1, 6);
    }
  });

  it('es determinista y valida opciones', () => {
    expect(Array.from(buildHeightfieldMeshData(h).positions))
      .toEqual(Array.from(buildHeightfieldMeshData(h).positions));
    expect(() => buildHeightfieldMeshData(h, { cellSize: 0 })).toThrow('cellSize');
    expect(() => buildHeightfieldMeshData(h, { uvScale: -1 })).toThrow('uvScale');
    expect(() => buildHeightfieldMeshData(h, { colorRamp: [[0, 0, 0], [1, 1, 1]] })).toThrow('colorRamp');
  });
});
