import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import * as gameCore from '../../../game-core';
import { terrainOptionsPreset } from '../../../game-core';

/* Spy sobre la fábrica real: el comparador debe crear el material del agua
 * UNA vez (al montar) y solo regenerar geometría después. Cada llamada extra
 * de `buildToonWaterPlane` filtra un MeshToonMaterial sin liberar (138A-4,
 * hallazgo IMPORTANTE del supervisor_reviewer). */
vi.mock('./game-toon-water', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./game-toon-water')>();
  return { ...actual, buildToonWaterPlane: vi.fn(actual.buildToonWaterPlane) };
});

/* [138A-6] Spy sobre el presupuesto de vegetación: el modo suave debe llamar
 * `placeVegetation` con maxTrees=0 conservando césped y rocas. */
vi.mock('../../../game-core', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../game-core')>();
  return { ...actual, placeVegetation: vi.fn(actual.placeVegetation) };
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

describe('comparador procedural — cellSize real y estilos (138A-6)', () => {
  it('el agua, el preview y el pick escalan con cellSize', () => {
    const scene = new THREE.Scene();
    const comparator: ProceduralComparator = mountProceduralComparator(
      scene,
      createWorldBend(),
      new THREE.Texture(),
      1337,
      0,
      0,
      { ...terrainOptionsPreset('isla'), cellSize: 2 },
    );

    const world = scene.children[0] as THREE.Group;
    const water = world.children[0] as THREE.Mesh;
    const waterPlane = water.geometry as THREE.PlaneGeometry;
    expect(waterPlane.parameters.width).toBeCloseTo(48 * 2 * 2.4);
    expect(waterPlane.parameters.height).toBeCloseTo(32 * 2 * 2.4);

    /* Modo bloques: la huella x/z del grupo escala por cellSize (la altura
     * no: maxHeight es un control independiente del contrato). */
    comparator.setMode('bloques');
    const blocksGroup = comparator.raycastGroup.parent as THREE.Group;
    expect(blocksGroup.scale.x).toBe(2);
    expect(blocksGroup.scale.y).toBe(1);
    expect(blocksGroup.scale.z).toBe(2);

    /* Pick en el centro de la isla: la celda devuelta reporta coordenadas de
     * mundo escaladas por cellSize (paridad con el documento). */
    const pick = comparator.pickTerrain(1.5, 0, 0.5);
    expect(pick?.i).toBe(24);
    expect(pick?.j).toBe(16);
    expect(pick?.worldX).toBe(1);
    expect(pick?.worldZ).toBe(1);
    expect(pick?.level).not.toBeNull();

    comparator.setMode('suave');
    const smoothPick = comparator.pickTerrain(1.5, 0, 0.5);
    expect(smoothPick?.i).toBe(24);
    expect(smoothPick?.j).toBe(16);
    expect(smoothPick?.worldX).toBe(1);
    expect(smoothPick?.worldZ).toBe(1);
    expect(smoothPick?.level).toBeNull();

    /* Regenerar con otra celda actualiza el agua en tiempo real. */
    comparator.regenerateFromOptions({ ...terrainOptionsPreset('isla'), cellSize: 1 });
    const water1 = (scene.children[0] as THREE.Group).children[0] as THREE.Mesh;
    expect((water1.geometry as THREE.PlaneGeometry).parameters.width).toBeCloseTo(48 * 1 * 2.4);

    comparator.dispose();
  });

  it('suave no coloca árboles: placeVegetation recibe maxTrees=0 y conserva el resto', () => {
    const scene = new THREE.Scene();
    const comparator: ProceduralComparator = mountProceduralComparator(
      scene,
      createWorldBend(),
      new THREE.Texture(),
      1337,
      0,
      0,
      terrainOptionsPreset('isla'),
    );

    comparator.setMode('suave');
    expect(gameCore.placeVegetation).toHaveBeenCalledWith(
      expect.anything(),
      expect.anything(),
      expect.objectContaining({
        maxTrees: 0,
        maxGrass: expect.any(Number),
        maxRocks: expect.any(Number),
      }),
    );
    expect(comparator.terrainStats().propCount).toBeGreaterThan(0);

    comparator.dispose();
  });
});
