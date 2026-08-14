import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import * as gameCore from '../../../game-core';
import { buildMapVersionFromOptions, terrainOptionsPreset, WORLD_PALETTE_DEFAULTS } from '../../../game-core';

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

describe('comparador procedural — paleta, rampa y documento (138A-8)', () => {
  it('setPalette recolorea agua, bloques y suave sin tocar opciones', () => {
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
    const world = scene.children[0] as THREE.Group;
    const water = world.children[0] as THREE.Mesh;
    const waterMaterial = water.material as THREE.MeshToonMaterial;
    const blocksTerrain = comparator.raycastGroup as THREE.Mesh;
    const blocksColors = () => Array.from(
      (blocksTerrain.geometry.getAttribute('color') as THREE.BufferAttribute).array,
    );
    const before = blocksColors();

    const custom = { ...WORLD_PALETTE_DEFAULTS, grass: 0x112233, waterShallow: 0xabcdef };
    comparator.setPalette(custom);

    expect(waterMaterial.color.getHex()).toBe(0xabcdef);
    /* setPalette reconstruye los meshes: el raycastGroup apunta al nuevo. */
    const afterTerrain = comparator.raycastGroup as THREE.Mesh;
    const after = Array.from(
      (afterTerrain.geometry.getAttribute('color') as THREE.BufferAttribute).array,
    );
    expect(after).not.toEqual(before);
    /* La cara superior de hierba se tiñe con jitter ±0.05: tolerancia amplia. */
    expect(after.some(value => Math.abs(value - 0x11 / 255) < 0.05)).toBe(true);

    comparator.setMode('suave');
    const smoothTerrain = comparator.raycastGroup as THREE.Mesh;
    const smoothColors = Array.from(
      (smoothTerrain.geometry.getAttribute('color') as THREE.BufferAttribute).array,
    );
    expect(smoothColors.some(value => Math.abs(value - 0x11 / 255) < 0.05)).toBe(true);

    comparator.dispose();
  });

  it('setToonRamp actualiza el gradientMap del material compartido', () => {
    const scene = new THREE.Scene();
    const comparator: ProceduralComparator = mountProceduralComparator(
      scene,
      createWorldBend(),
      new THREE.Texture(),
    );
    const nextRamp = new THREE.Texture();
    comparator.setToonRamp(nextRamp);
    const world = scene.children[0] as THREE.Group;
    const blocksTerrain = (world.children[1] as THREE.Group).children[0] as THREE.Mesh;
    expect((blocksTerrain.material as THREE.MeshToonMaterial).gradientMap).toBe(nextRamp);
    comparator.dispose();
  });

  it('setDocument muestra los props del documento y setDocument(null) los restaura', () => {
    const scene = new THREE.Scene();
    const comparator: ProceduralComparator = mountProceduralComparator(
      scene,
      createWorldBend(),
      new THREE.Texture(),
      1337,
      0,
      0,
      { ...terrainOptionsPreset('isla'), style: 'bloques', seed: 7 },
    );
    const map = buildMapVersionFromOptions({ ...terrainOptionsPreset('isla'), style: 'bloques', seed: 7 });

    const world = scene.children[0] as THREE.Group;
    const blocksGroup = world.children[1] as THREE.Group;
    const generatedProps = blocksGroup.children[1] as THREE.Mesh;
    expect(generatedProps.visible).toBe(true);
    expect(comparator.terrainStats().propCount).toBeGreaterThan(0);

    comparator.setDocument(map);
    expect(comparator.terrainStats().propCount).toBe(map.instances.length);
    expect(generatedProps.visible).toBe(false);
    /* El grupo de documento se añade tras los dos modos (índice 3). */
    const docGroup = world.children[3] as THREE.Group;
    expect(docGroup.children.length).toBe(1);
    expect(docGroup.visible).toBe(true);

    comparator.setDocument(null);
    expect(generatedProps.visible).toBe(true);
    expect(comparator.terrainStats().propCount).toBeGreaterThan(0);
    expect(world.children).not.toContain(docGroup);

    comparator.dispose();
  });
});
