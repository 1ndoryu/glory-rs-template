import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import {
  FIXTURE_MAP_VERSION,
  FIXTURE_PROPS,
} from './game-fixture-map';
import { createGamePlayableVisualCache } from './game-playable-visual-cache';
import type { VisibleMapContent } from '../../../game-core';

function createMaterials(): {
  ink: THREE.Material;
  paper: THREE.Material;
  pale: THREE.Material;
  middle: THREE.Material;
  water: THREE.Material;
  lines: THREE.LineBasicMaterial;
} {
  return {
    ink: new THREE.MeshBasicMaterial({ color: 0x111111 }),
    paper: new THREE.MeshBasicMaterial({ color: 0xf8f8f4 }),
    pale: new THREE.MeshBasicMaterial({ color: 0xd7d7d1 }),
    middle: new THREE.MeshBasicMaterial({ color: 0x8d8d88 }),
    water: new THREE.MeshBasicMaterial({ color: 0x55555a }),
    lines: new THREE.LineBasicMaterial({ color: 0x050505 }),
  };
}

describe('GamePlayableVisualCache', () => {
  it('evicts old non-active terrain after the bounded visual cache is full', () => {
    const scene = new THREE.Scene();
    const materials = createMaterials();
    const cache = createGamePlayableVisualCache({
      scene,
      materials,
      map: FIXTURE_MAP_VERSION,
      props: new Map(FIXTURE_PROPS.map(prop => [prop.id, prop])),
    });
    const sourceChunk = FIXTURE_MAP_VERSION.terrain.chunks[0];
    const chunks = Array.from({ length: 13 }, (_, x) => ({ ...sourceChunk, x }));
    const content = (chunk: typeof sourceChunk): VisibleMapContent => ({
      chunkKeys: [`${chunk.x}:0`],
      chunks: [chunk],
      instances: [],
      assets: [],
      assetVersionIds: [],
      evictedChunkKeys: [],
      cacheSize: 1,
    });

    chunks.forEach(chunk => cache.sync(content(chunk)));

    expect(scene.children.some(
      child => child instanceof THREE.Mesh && child.userData.chunkKey === '0:0',
    )).toBe(false);
    expect(scene.children.some(
      child => child instanceof THREE.Mesh && child.userData.chunkKey === '12:0',
    )).toBe(true);

    cache.destroy();
    Object.values(materials).forEach(material => material.dispose());
  });

  it('reuses terrain geometry after a temporary visibility eviction', () => {
    const scene = new THREE.Scene();
    const materials = createMaterials();
    const cache = createGamePlayableVisualCache({
      scene,
      materials,
      map: FIXTURE_MAP_VERSION,
      props: new Map(FIXTURE_PROPS.map(prop => [prop.id, prop])),
    });
    const chunk = FIXTURE_MAP_VERSION.terrain.chunks[0];
    const content: VisibleMapContent = {
      chunkKeys: ['0:0'],
      chunks: [chunk],
      instances: [],
      assets: [],
      assetVersionIds: [],
      evictedChunkKeys: [],
      cacheSize: 1,
    };

    cache.sync(content);
    const terrain = scene.children.find(
      child => child instanceof THREE.Mesh && child.userData.chunkKey === '0:0',
    ) as THREE.Mesh;
    expect(terrain).toBeDefined();
    const geometry = terrain.geometry;

    cache.sync({ ...content, chunkKeys: [], chunks: [] });
    expect(terrain.parent).toBeNull();
    cache.sync(content);

    expect(scene.children.find(
      child => child instanceof THREE.Mesh && child.userData.chunkKey === '0:0',
    )).toBe(terrain);
    expect(terrain.geometry).toBe(geometry);
    expect(terrain.parent).toBe(scene);

    cache.destroy();
    Object.values(materials).forEach(material => material.dispose());
  });

  it('uses AssetInstance transform instead of fixture display coordinates', () => {
    const scene = new THREE.Scene();
    const materials = createMaterials();
    const cache = createGamePlayableVisualCache({
      scene,
      materials,
      map: FIXTURE_MAP_VERSION,
      props: new Map(FIXTURE_PROPS.map(prop => [prop.id, prop])),
    });
    const prop = FIXTURE_PROPS.find(candidate => candidate.id === 'rock-north')!;
    const sourceInstance = FIXTURE_MAP_VERSION.instances.find(instance => instance.id === prop.id)!;
    const content: VisibleMapContent = {
      chunkKeys: ['0:0'],
      chunks: [FIXTURE_MAP_VERSION.terrain.chunks[0]],
      instances: [{
        ...sourceInstance,
        position: { x: 12.5, z: -1.25 },
        rotationY: 0.4,
        scale: 1.7,
      }],
      assets: [FIXTURE_MAP_VERSION.assetManifest[sourceInstance.assetVersionId]],
      assetVersionIds: [sourceInstance.assetVersionId],
      evictedChunkKeys: [],
      cacheSize: 1,
    };

    cache.sync(content);

    const instanceMesh = scene.children.find(
      child => child instanceof THREE.InstancedMesh && child.count === 1,
    );
    expect(instanceMesh).toBeInstanceOf(THREE.InstancedMesh);
    expect((instanceMesh as THREE.InstancedMesh).frustumCulled).toBe(true);
    expect((instanceMesh as THREE.InstancedMesh).boundingSphere).not.toBeNull();
    const matrix = new THREE.Matrix4();
    (instanceMesh as THREE.InstancedMesh).getMatrixAt(0, matrix);
    const position = new THREE.Vector3();
    position.setFromMatrixPosition(matrix);
    expect(position.x).toBeCloseTo(12.5);
    expect(position.z).toBeCloseTo(-1.25);
    const scale = new THREE.Vector3();
    matrix.decompose(new THREE.Vector3(), new THREE.Quaternion(), scale);
    expect(scale.x).toBeCloseTo(2.125);
    expect(scale.y).toBeCloseTo(1.105);
    expect(scale.z).toBeCloseTo(1.7);

    const outline = scene.children.find(
      child => child instanceof THREE.Group && child.userData.instanceId === prop.id,
    );
    expect(outline).toBeInstanceOf(THREE.Group);
    expect((outline as THREE.Group).position.x).toBeCloseTo(12.5);
    expect((outline as THREE.Group).position.z).toBeCloseTo(-1.25);

    const movedContent: VisibleMapContent = {
      ...content,
      instances: [{ ...content.instances[0], position: { x: 4, z: 3 }, scale: 1.1 }],
    };
    cache.sync(movedContent);
    expect((outline as THREE.Group).position.x).toBeCloseTo(4);
    expect((outline as THREE.Group).position.z).toBeCloseTo(3);
    expect((outline as THREE.Group).scale.x).toBeCloseTo(1.1);

    cache.destroy();
    cache.destroy();
    materials.ink.dispose();
    materials.paper.dispose();
    materials.pale.dispose();
    materials.middle.dispose();
    materials.water.dispose();
    materials.lines.dispose();
  });
});
