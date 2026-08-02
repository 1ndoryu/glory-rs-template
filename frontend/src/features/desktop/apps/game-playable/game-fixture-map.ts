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
  readonly id: string;
  readonly assetVersionId: string;
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

export const FIXTURE_PROPS: readonly FixtureProp[] = [
  { id: 'conifer-west-1', assetVersionId: 'asset-conifer', kind: 'conifer', x: -8.2, z: -5.3, scale: 0.9 },
  { id: 'conifer-west-2', assetVersionId: 'asset-conifer', kind: 'conifer', x: -7.2, z: -2.8, scale: 1.1 },
  { id: 'conifer-west-3', assetVersionId: 'asset-conifer', kind: 'conifer', x: -8.6, z: 1.4, scale: 0.85 },
  { id: 'conifer-west-4', assetVersionId: 'asset-conifer', kind: 'conifer', x: -6.6, z: 6.1, scale: 0.95 },
  { id: 'broadleaf-north-1', assetVersionId: 'asset-broadleaf', kind: 'broadleaf', x: -3.2, z: -6.1, scale: 0.95 },
  { id: 'broadleaf-north-2', assetVersionId: 'asset-broadleaf', kind: 'broadleaf', x: -1.4, z: 6.4, scale: 0.85 },
  { id: 'broadleaf-east-1', assetVersionId: 'asset-broadleaf', kind: 'broadleaf', x: 2.4, z: 5.8, scale: 0.9 },
  { id: 'broadleaf-east-2', assetVersionId: 'asset-broadleaf', kind: 'broadleaf', x: 8.0, z: 5.4, scale: 0.95 },
  { id: 'conifer-east-1', assetVersionId: 'asset-conifer', kind: 'conifer', x: 8.4, z: -6.0, scale: 1.05 },
  { id: 'rock-north', assetVersionId: 'asset-rock', kind: 'rock', x: -5.2, z: 4.8, scale: 0.8 },
  { id: 'rock-south', assetVersionId: 'asset-rock', kind: 'rock', x: 3.4, z: -5.3, scale: 0.65 },
  { id: 'rock-east', assetVersionId: 'asset-rock', kind: 'rock', x: 5.9, z: 3.8, scale: 0.55 },
  { id: 'pond-east', assetVersionId: 'asset-pond', kind: 'pond', x: 5.2, z: -1.4, scale: 1, width: 2.6, depth: 1.7 },
  { id: 'pond-west', assetVersionId: 'asset-pond', kind: 'pond', x: -4.4, z: 5.3, scale: 1, width: 1.5, depth: 0.9 },
];

const terrainHeights = Array.from({ length: 17 * 17 }, (_, index) => {
  const x = index % 17;
  const z = Math.floor(index / 17);
  return Number((Math.sin(x * 0.3) * Math.cos(z * 0.2) * 0.15).toFixed(3));
});

const terrainSurfaces = Array.from({ length: 16 * 16 }, (_, index) => index % 11 === 0 ? 1 : 0);
const FIXTURE_CHUNK = { x: 0, z: 0, heights: terrainHeights, surfaces: terrainSurfaces } as const;
const FIXTURE_EAST_CHUNK = { x: 1, z: 0, heights: terrainHeights, surfaces: terrainSurfaces } as const;

const FIXTURE_INSTANCES: readonly AssetInstance[] = FIXTURE_PROPS.map((prop) => ({
  id: prop.id,
  assetVersionId: prop.assetVersionId,
  position: { x: prop.x, z: prop.z },
  rotationY: 0,
  scale: prop.scale,
  terrainAnchor: 'surface',
}));

export const FIXTURE_MAP_VERSION: MapVersion = {
  schemaVersion: 1,
  id: 'fixture-bosque-v1',
  terrain: {
    schemaVersion: 1,
    bounds: { minX: -10, maxX: 22, minZ: -8, maxZ: 8 },
    cellSize: 1,
    chunkSize: 16,
    chunks: [FIXTURE_CHUNK, FIXTURE_EAST_CHUNK],
  },
  assetManifest: Object.fromEntries(FIXTURE_ASSETS.map(asset => [asset.id, asset])),
  instances: FIXTURE_INSTANCES,
  spawnPoints: [{ id: 'spawn-centre', position: { x: 0, z: -0.5 }, radius: 0.38 }],
};

export const FIXTURE_MAP: WorldMap = mapVersionToWorldMap(FIXTURE_MAP_VERSION);

export const FIXTURE_COLLIDERS: readonly StaticCollider[] = FIXTURE_MAP.colliders;
