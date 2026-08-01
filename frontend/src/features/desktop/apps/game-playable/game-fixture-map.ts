/* GAME-01 — Fixture jugable offline: mapa publicado pequeño y determinista.
 * El documento usa el contrato de MapVersion para probar el mismo boundary que
 * consumirá el futuro backend; todavía no hay red, persistencia ni editor. */

import {
  mapVersionToWorldMap,
  type AssetInstance,
  type GameAssetVersion,
  type MapVersion,
  type StaticCollider,
  type WorldMap,
} from '../../../game-core';

export interface FixtureProp {
  readonly kind: 'conifer' | 'broadleaf' | 'rock' | 'pond';
  readonly x: number;
  readonly z: number;
  readonly scale: number;
  readonly width?: number;
  readonly depth?: number;
}

const FIXTURE_ASSETS: readonly GameAssetVersion[] = [
  { id: 'asset-conifer', category: 'tree', contentHash: 'fixture-conifer-v1' },
  { id: 'asset-broadleaf', category: 'tree', contentHash: 'fixture-broadleaf-v1' },
  { id: 'asset-rock', category: 'rock', contentHash: 'fixture-rock-v1', collisionProxy: { kind: 'circle', radius: 0.8 } },
  { id: 'asset-pond', category: 'water', contentHash: 'fixture-pond-v1', collisionProxy: { kind: 'aabb', halfWidth: 2.6, halfDepth: 1.7 } },
  { id: 'asset-tree-collider', category: 'tree', contentHash: 'fixture-tree-v1', collisionProxy: { kind: 'circle', radius: 0.7 } },
];

const FIXTURE_INSTANCES: readonly AssetInstance[] = [
  { id: 'pond-east', assetVersionId: 'asset-pond', position: { x: 5.2, z: -1.4 }, rotationY: 0, scale: 1, terrainAnchor: 'surface' },
  { id: 'pond-west', assetVersionId: 'asset-pond', position: { x: -4.4, z: 5.3 }, rotationY: 0, scale: 0.58, terrainAnchor: 'surface' },
  { id: 'rock-north', assetVersionId: 'asset-rock', position: { x: -5.2, z: 4.8 }, rotationY: 0, scale: 1, terrainAnchor: 'surface' },
  { id: 'tree-north-east', assetVersionId: 'asset-tree-collider', position: { x: 7.4, z: 4.3 }, rotationY: 0, scale: 1, terrainAnchor: 'surface' },
  { id: 'tree-south-east', assetVersionId: 'asset-tree-collider', position: { x: 7.8, z: -5.1 }, rotationY: 0, scale: 1, terrainAnchor: 'surface' },
];

const terrainHeights = Array.from({ length: 17 * 17 }, (_, index) => {
  const x = index % 17;
  const z = Math.floor(index / 17);
  return Number((Math.sin(x * 0.3) * Math.cos(z * 0.2) * 0.15).toFixed(3));
});

const terrainSurfaces = Array.from({ length: 16 * 16 }, (_, index) => index % 11 === 0 ? 1 : 0);

export const FIXTURE_MAP_VERSION: MapVersion = {
  schemaVersion: 1,
  id: 'fixture-bosque-v1',
  terrain: {
    schemaVersion: 1,
    bounds: { minX: -10, maxX: 10, minZ: -8, maxZ: 8 },
    cellSize: 1,
    chunkSize: 16,
    chunks: [{ x: 0, z: 0, heights: terrainHeights, surfaces: terrainSurfaces }],
  },
  assetManifest: Object.fromEntries(FIXTURE_ASSETS.map(asset => [asset.id, asset])),
  instances: FIXTURE_INSTANCES,
  spawnPoints: [{ id: 'spawn-centre', position: { x: 0, z: -0.5 }, radius: 0.38 }],
};

export const FIXTURE_MAP: WorldMap = mapVersionToWorldMap(FIXTURE_MAP_VERSION);

export const FIXTURE_PROPS: readonly FixtureProp[] = [
  { kind: 'conifer', x: -8.2, z: -5.3, scale: 0.9 },
  { kind: 'conifer', x: -7.2, z: -2.8, scale: 1.1 },
  { kind: 'conifer', x: -8.6, z: 1.4, scale: 0.85 },
  { kind: 'conifer', x: -6.6, z: 6.1, scale: 0.95 },
  { kind: 'broadleaf', x: -3.2, z: -6.1, scale: 0.95 },
  { kind: 'broadleaf', x: -1.4, z: 6.4, scale: 0.85 },
  { kind: 'broadleaf', x: 2.4, z: 5.8, scale: 0.9 },
  { kind: 'broadleaf', x: 8.0, z: 5.4, scale: 0.95 },
  { kind: 'conifer', x: 8.4, z: -6.0, scale: 1.05 },
  { kind: 'rock', x: -5.2, z: 4.8, scale: 0.8 },
  { kind: 'rock', x: 3.4, z: -5.3, scale: 0.65 },
  { kind: 'rock', x: 5.9, z: 3.8, scale: 0.55 },
  { kind: 'pond', x: 5.2, z: -1.4, scale: 1, width: 2.6, depth: 1.7 },
  { kind: 'pond', x: -4.4, z: 5.3, scale: 1, width: 1.5, depth: 0.9 },
];

export const FIXTURE_COLLIDERS: readonly StaticCollider[] = FIXTURE_MAP.colliders;
