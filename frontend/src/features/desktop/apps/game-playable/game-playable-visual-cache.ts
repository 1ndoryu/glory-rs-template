/* GAME-01 — Cache visual acotado del fixture.
 * El loader lógico decide qué está visible; este adaptador solo materializa
 * chunks y props en Three.js, compartiendo geometría mediante prototipos.
 */

import * as THREE from 'three';
import {
  buildTerrainMeshData,
  type MapVersion,
  type VisibleMapContent,
} from '../../../game-core';
import {
  createBroadleaf,
  createConifer,
  createPond,
  createRock,
  type ForestMaterials,
} from '../game-shared/forest-models';
import type { FixtureProp } from './game-fixture-map';

export interface GamePlayableVisualCacheOptions {
  readonly scene: THREE.Scene;
  readonly materials: ForestMaterials;
  readonly map: MapVersion;
  readonly props: ReadonlyMap<string, FixtureProp>;
}

export function createGamePlayableVisualCache(options: GamePlayableVisualCacheOptions): GamePlayableVisualCache {
  return new GamePlayableVisualCache(options);
}

export class GamePlayableVisualCache {
  private readonly terrainObjects = new Map<string, THREE.Mesh>();
  private readonly propObjects = new Map<string, THREE.Group>();
  private readonly prototypes = new Map<FixtureProp['kind'], THREE.Group>();
  private destroyed = false;

  public constructor(private readonly options: GamePlayableVisualCacheOptions) {}

  public sync(content: VisibleMapContent): void {
    if (this.destroyed) return;
    const activeChunks = new Set(content.chunkKeys);
    for (const chunk of content.chunks) {
      const key = `${chunk.x}:${chunk.z}`;
      const terrain = this.terrainObjects.get(key) ?? this.createTerrain(key, chunk);
      if (!terrain.parent) this.options.scene.add(terrain);
    }
    for (const [key, terrain] of this.terrainObjects) {
      if (activeChunks.has(key)) continue;
      this.options.scene.remove(terrain);
      terrain.geometry.dispose();
      this.terrainObjects.delete(key);
    }

    const activeProps = new Set<string>();
    for (const instance of content.instances) {
      const prop = this.options.props.get(instance.id);
      if (!prop) continue;
      const object = this.propObjects.get(prop.id) ?? this.createProp(prop);
      object.position.set(instance.position.x, prop.kind === 'pond' ? 0 : 0.15, instance.position.z);
      this.configurePropTransform(object, prop);
      activeProps.add(prop.id);
    }
    for (const [id, object] of this.propObjects) {
      if (activeProps.has(id)) continue;
      this.options.scene.remove(object);
      this.propObjects.delete(id);
    }
  }

  public destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    for (const object of this.propObjects.values()) this.options.scene.remove(object);
    for (const terrain of this.terrainObjects.values()) {
      this.options.scene.remove(terrain);
      terrain.geometry.dispose();
    }
    for (const prototype of this.prototypes.values()) {
      disposeObjectGeometries(prototype);
    }
    this.terrainObjects.clear();
    this.propObjects.clear();
    this.prototypes.clear();
  }

  private createTerrain(key: string, chunk: MapVersion['terrain']['chunks'][number]): THREE.Mesh {
    const data = buildTerrainMeshData(
      chunk,
      this.options.map.terrain.cellSize,
      this.options.map.terrain.bounds.minX,
      this.options.map.terrain.bounds.minZ,
    );
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(data.positions, 3));
    geometry.setIndex(new THREE.BufferAttribute(data.indices, 1));
    geometry.clearGroups();
    for (let cell = 0; cell < data.surfaces.length; cell += 1) {
      geometry.addGroup(cell * 6, 6, surfaceMaterialIndex(data.surfaces[cell]));
    }
    geometry.computeVertexNormals();
    const material = [this.options.materials.pale, this.options.materials.water, this.options.materials.middle];
    const terrain = new THREE.Mesh(geometry, material);
    terrain.receiveShadow = true;
    terrain.userData.chunkKey = key;
    this.terrainObjects.set(key, terrain);
    return terrain;
  }

  private createProp(prop: FixtureProp): THREE.Group {
    const prototype = this.getPrototype(prop.kind);
    const object = prototype.clone(true);
    object.userData.instanceId = prop.id;
    object.userData.assetVersionId = prop.assetVersionId;
    this.options.scene.add(object);
    this.propObjects.set(prop.id, object);
    return object;
  }

  private getPrototype(kind: FixtureProp['kind']): THREE.Group {
    const cached = this.prototypes.get(kind);
    if (cached) return cached;
    const { materials } = this.options;
    const prototype = kind === 'conifer'
      ? createConifer(materials)
      : kind === 'broadleaf'
        ? createBroadleaf(materials)
        : kind === 'rock'
          ? createRock(materials)
          : createPond(materials, 1, 1);
    this.prototypes.set(kind, prototype);
    return prototype;
  }

  private configurePropTransform(object: THREE.Group, prop: FixtureProp): void {
    if (prop.kind === 'pond') {
      object.scale.set(prop.width ?? 1, prop.depth ?? 1, 1);
      return;
    }
    object.scale.setScalar(prop.scale);
    if (prop.kind === 'rock') object.rotation.y = prop.scale * 0.8;
  }
}

function surfaceMaterialIndex(surface: number): number {
  return surface === 1 ? 1 : surface === 2 ? 2 : 0;
}

function disposeObjectGeometries(object: THREE.Object3D): void {
  object.traverse((child) => {
    if (child instanceof THREE.Mesh || child instanceof THREE.LineSegments) {
      child.geometry.dispose();
    }
  });
}
