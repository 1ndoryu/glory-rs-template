/* GAME-01 — Primitivas visuales compartidas por previews y fixture.
 * No contienen estado, input ni lógica de juego; solo construyen geometría
 * temporal de la dirección artística aprobada. */

import * as THREE from 'three';

export interface ForestMaterials {
  readonly ink: THREE.Material;
  readonly paper: THREE.Material;
  readonly pale: THREE.Material;
  readonly middle: THREE.Material;
  readonly water: THREE.Material;
  readonly lines: THREE.LineBasicMaterial;
}

function outlined(geometry: THREE.BufferGeometry, material: THREE.Material, lines: THREE.LineBasicMaterial): THREE.Group {
  const group = new THREE.Group();
  const mesh = new THREE.Mesh(geometry, material);
  mesh.castShadow = true;
  mesh.receiveShadow = true;
  group.add(mesh, new THREE.LineSegments(new THREE.EdgesGeometry(geometry, 18), lines));
  return group;
}

export function createConifer(materials: ForestMaterials, scale = 1): THREE.Group {
  const tree = new THREE.Group();
  const trunk = outlined(new THREE.CylinderGeometry(0.16, 0.24, 2.1, 5), materials.ink, materials.lines);
  trunk.position.y = 1.05;
  const lower = outlined(new THREE.ConeGeometry(1.25, 2.8, 7), materials.middle, materials.lines);
  lower.position.y = 2.4;
  const upper = outlined(new THREE.ConeGeometry(0.9, 2.4, 7), materials.paper, materials.lines);
  upper.position.y = 3.55;
  tree.add(trunk, lower, upper);
  tree.scale.setScalar(scale);
  return tree;
}

export function createBroadleaf(materials: ForestMaterials, scale = 1): THREE.Group {
  const tree = new THREE.Group();
  const trunk = outlined(new THREE.CylinderGeometry(0.2, 0.3, 2.3, 6), materials.ink, materials.lines);
  trunk.position.y = 1.15;
  const crown = outlined(new THREE.IcosahedronGeometry(1.25, 1), materials.paper, materials.lines);
  crown.position.y = 2.85;
  crown.scale.set(1, 1.25, 1);
  tree.add(trunk, crown);
  tree.scale.setScalar(scale);
  return tree;
}

export function createRock(materials: ForestMaterials, scale = 1): THREE.Group {
  const rock = outlined(new THREE.IcosahedronGeometry(0.65, 0), materials.middle, materials.lines);
  rock.scale.set(1.25 * scale, 0.65 * scale, scale);
  rock.position.y = 0.42 * scale;
  rock.rotation.y = scale * 0.8;
  return rock;
}

export function createFigure(materials: ForestMaterials, remote = false): THREE.Group {
  const figure = new THREE.Group();
  const material = remote ? materials.middle : materials.ink;
  const body = outlined(new THREE.CylinderGeometry(0.28, 0.38, 1.2, 6), material, materials.lines);
  body.position.y = 0.9;
  const head = outlined(new THREE.IcosahedronGeometry(0.34, 1), material, materials.lines);
  head.position.y = 1.72;
  figure.add(body, head);
  return figure;
}

export function createPond(materials: ForestMaterials, width: number, depth: number): THREE.Group {
  const pond = outlined(new THREE.CircleGeometry(1, 18), materials.water, materials.lines);
  pond.rotation.x = -Math.PI / 2;
  pond.scale.set(width, depth, 1);
  pond.position.y = 0.34;
  return pond;
}
