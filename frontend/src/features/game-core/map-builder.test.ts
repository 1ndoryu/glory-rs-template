import { describe, expect, it } from 'vitest';
import {
  assertValidMapVersion,
} from './map-version';
import {
  buildMapVersionFromOptions,
  mapBuilderStats,
  parseSerializedWorld,
  serializeWorld,
} from './map-builder';
import { TERRAIN_OPTIONS_DEFAULTS, terrainOptionsPreset } from './procedural/terrain-options';
import type { MapVersion } from './map-version';
import type { TerrainLayer } from './terrain-layers';

describe('buildMapVersionFromOptions (138A-4)', () => {
  it('construye un MapVersion válido con chunks, assets, instancias y spawn', () => {
    const map = buildMapVersionFromOptions(TERRAIN_OPTIONS_DEFAULTS);
    expect(() => assertValidMapVersion(map)).not.toThrow();
    expect(map.terrain.chunks.length).toBe(6); /* 48×32 → 3×2 chunks */
    expect(map.terrain.chunks.every(chunk => chunk.heights.length === 289 && chunk.surfaces.length === 256)).toBe(true);
    expect(map.terrain.bounds).toEqual({ minX: -24, maxX: 24, minZ: -16, maxZ: 16 });
    expect(map.spawnPoints.length).toBe(1);
    expect(map.instances.length).toBeGreaterThan(0);
    const stats = mapBuilderStats(map);
    expect(stats.chunks).toBe(6);
    expect(stats.instances).toBe(map.instances.length);
    expect(stats.trees + stats.rocks).toBe(stats.instances);
    expect(stats.vertices).toBe(6 * 289);
    expect(stats.triangles).toBe(6 * 16 * 16 * 2);
  });

  it('es determinista por seed y distinto entre seeds', () => {
    const a = buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, seed: 1234 });
    const b = buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, seed: 1234 });
    const c = buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, seed: 5678 });
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
    expect(JSON.stringify(a)).not.toBe(JSON.stringify(c));
  });

  it('todas las formas generan documentos válidos', () => {
    for (const shape of ['isla', 'continente', 'archipielago', 'valle'] as const) {
      const map = buildMapVersionFromOptions(terrainOptionsPreset(shape));
      expect(() => assertValidMapVersion(map)).not.toThrow();
      expect(mapBuilderStats(map).instances).toBeGreaterThanOrEqual(0);
    }
  });

  it('respeta tamaños, cellSize y densidades', () => {
    const big = buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, width: 64, depth: 64 });
    expect(big.terrain.chunks.length).toBe(16);
    expect(big.terrain.bounds.maxX).toBe(32);
    const scaled = buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, cellSize: 2 });
    expect(scaled.terrain.bounds.maxX).toBe(48);
    expect(() => assertValidMapVersion(scaled)).not.toThrow();
    const bare = buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, vegetationDensity: 0 });
    expect(bare.instances).toEqual([]);
    expect(bare.spawnPoints.length).toBe(1);
  });

  it('estilo suave no coloca árboles y conserva rocas (138A-6)', () => {
    const suave = buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, style: 'suave' });
    const stats = mapBuilderStats(suave);
    expect(stats.trees).toBe(0);
    expect(stats.instances).toBe(stats.rocks);
    expect(stats.rocks).toBeGreaterThan(0);
  });

  it('las instancias heredan la escala base menor del toolkit (138A-6)', () => {
    const map = buildMapVersionFromOptions(TERRAIN_OPTIONS_DEFAULTS);
    expect(map.instances.length).toBeGreaterThan(0);
    for (const instance of map.instances) {
      expect(instance.scale).toBeLessThanOrEqual(0.625 + 1e-9);
    }
  });

  it('falla cerrado con opciones inválidas', () => {
    expect(() => buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, width: 17 })).toThrow('width');
    expect(() => buildMapVersionFromOptions({ ...TERRAIN_OPTIONS_DEFAULTS, shape: 'luna' as never })).toThrow('shape');
  });

  it('el spawn cae sobre tierra dentro de bounds', () => {
    const map = buildMapVersionFromOptions(TERRAIN_OPTIONS_DEFAULTS);
    const spawn = map.spawnPoints[0];
    expect(spawn.position.x).toBeGreaterThan(map.terrain.bounds.minX);
    expect(spawn.position.x).toBeLessThan(map.terrain.bounds.maxX);
    expect(spawn.position.z).toBeGreaterThan(map.terrain.bounds.minZ);
    expect(spawn.position.z).toBeLessThan(map.terrain.bounds.maxZ);
  });

  it('aplica el stack de capas a alturas y superficies (138A-9)', () => {
    const options = { ...TERRAIN_OPTIONS_DEFAULTS, seed: 42, waterLevel: 0 };
    const layer: TerrainLayer = {
      id: 'colina', name: 'Colina', enabled: true, kind: 'elevation',
      shape: { kind: 'circle', cx: 0, cz: 0, radius: 4 },
      falloff: 'smooth', falloffRadius: 1, bias: 1, blend: 'set',
      height: 3, elevationMode: 'absolute',
    };
    const base = buildMapVersionFromOptions(options);
    const layered = buildMapVersionFromOptions(options, 'constructor-bosque', [layer]);
    expect(() => assertValidMapVersion(layered)).not.toThrow();
    /* El chunk central debe tener alguna altura mayor tras la capa. */
    const centerChunk = layered.terrain.chunks.find(chunk => chunk.x === 1 && chunk.z === 1)!;
    const baseCenter = base.terrain.chunks.find(chunk => chunk.x === 1 && chunk.z === 1)!;
    expect(Math.max(...centerChunk.heights)).toBeGreaterThan(Math.max(...baseCenter.heights));
  });

  it('pinta superficies en los chunks y filtra vegetación sobre no-hierba', () => {
    const options = { ...TERRAIN_OPTIONS_DEFAULTS, seed: 42, waterLevel: 0 };
    const layer: TerrainLayer = {
      id: 'lago', name: 'Lago', enabled: true, kind: 'water',
      shape: { kind: 'circle', cx: 0, cz: 0, radius: 6 },
      falloff: 'hard', falloffRadius: 0.5, bias: 1, blend: 'set', hardness: 0.5,
      lowerToWater: true,
    };
    const layered = buildMapVersionFromOptions(options, 'constructor-bosque', [layer]);
    const centerChunk = layered.terrain.chunks.find(chunk => chunk.x === 1 && chunk.z === 1)!;
    expect(centerChunk.surfaces).toContain(1);
    expect(() => assertValidMapVersion(layered)).not.toThrow();
  });
});

describe('serializeWorld/parseSerializedWorld (138A-4)', () => {
  it('round-trip de mundo + opciones', () => {
    const options = { ...TERRAIN_OPTIONS_DEFAULTS, seed: 777, shape: 'archipielago' as const };
    const map = buildMapVersionFromOptions(options);
    const parsed = parseSerializedWorld(serializeWorld(options, map));
    expect(parsed.options).toEqual(options);
    expect(JSON.stringify(parsed.map)).toBe(JSON.stringify(map));
  });

  it('rechaza formatos, versiones y documentos inválidos', () => {
    const options = TERRAIN_OPTIONS_DEFAULTS;
    const map = buildMapVersionFromOptions(options);
    const envelope = JSON.parse(serializeWorld(options, map)) as { format: string; version: number; options: unknown; map: MapVersion };
    expect(() => parseSerializedWorld(JSON.stringify({ ...envelope, format: 'otro' }))).toThrow('formato');
    expect(() => parseSerializedWorld(JSON.stringify({ ...envelope, version: 99 }))).toThrow('formato');
    expect(() => parseSerializedWorld(JSON.stringify({ ...envelope, options: { ...options, width: 3 } }))).toThrow('opciones');
    expect(() => parseSerializedWorld(JSON.stringify({ ...envelope, map: { ...map, spawnPoints: [] } }))).toThrow('MapVersion');
    expect(() => parseSerializedWorld('{no json')).toThrow('JSON');
  });

  it('round-trip con stack de capas y fail-closed ante capas inválidas', () => {
    const options = { ...TERRAIN_OPTIONS_DEFAULTS, seed: 9 };
    const layer: TerrainLayer = {
      id: 'arena', name: 'Arena', enabled: true, kind: 'sand',
      shape: { kind: 'painted', cells: [[4, 4], [4, 5]] },
      falloff: 'hard', falloffRadius: 0.25, bias: 1, blend: 'set', hardness: 0.5,
    };
    const map = buildMapVersionFromOptions(options, 'constructor-bosque', [layer]);
    const text = serializeWorld(options, map, [layer]);
    const parsed = parseSerializedWorld(text);
    expect(parsed.layers).toHaveLength(1);
    expect(parsed.layers![0].id).toBe('arena');
    expect(JSON.stringify(parsed.map)).toBe(JSON.stringify(map));

    const envelope = JSON.parse(text) as { layers: readonly TerrainLayer[] };
    expect(() => parseSerializedWorld(JSON.stringify({ ...envelope, layers: [{ id: 'x' }] })))
      .toThrow(/capas del mundo inválidas/);
  });

  it('exports previos a 138A-9 sin capas siguen parseando', () => {
    const options = TERRAIN_OPTIONS_DEFAULTS;
    const map = buildMapVersionFromOptions(options);
    const parsed = parseSerializedWorld(serializeWorld(options, map));
    expect(parsed.layers).toBeUndefined();
  });
});
