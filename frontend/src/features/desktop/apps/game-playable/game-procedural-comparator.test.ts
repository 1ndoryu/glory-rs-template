import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import { terrainOptionsPreset } from '../../../game-core';

/* Spy sobre la fábrica real: el comparador debe crear el material del agua
 * UNA vez (al montar) y solo regenerar geometría después. Cada llamada extra
 * de `buildToonWaterPlane` filtra un MeshToonMaterial sin liberar (138A-4,
 * hallazgo IMPORTANTE del supervisor_reviewer). */
vi.mock('./game-toon-water', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./game-toon-water')>();
  return { ...actual, buildToonWaterPlane: vi.fn(actual.buildToonWaterPlane) };
});

import { createWorldBend } from './game-world-bend';
import { mountProceduralComparator, type ProceduralComparator } from './game-procedural-comparator';
import * as waterModule from './game-toon-water';

describe('comparador procedural — ciclo de vida del agua', () => {
  const scene = new THREE.Scene();
  const ramp = new THREE.Texture();
  const bend = createWorldBend();

  it('no crea materiales de agua nuevos al regenerar (sin fugas GPU)', () => {
    const comparator: ProceduralComparator = mountProceduralComparator(scene, bend, ramp);
    expect(waterModule.buildToonWaterPlane).toHaveBeenCalledTimes(1);

    comparator.regenerate(4242);
    comparator.regenerateFromOptions({ ...terrainOptionsPreset('continente'), seed: 7 });
    comparator.regenerateFromOptions({ ...terrainOptionsPreset('valle'), seed: 99 });
    expect(waterModule.buildToonWaterPlane).toHaveBeenCalledTimes(1);

    expect(() => comparator.dispose()).not.toThrow();
  });
});
