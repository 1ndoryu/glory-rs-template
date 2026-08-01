/* [GAME-01-VIS-3D] Composición espacial del segundo boceto.
 * Separarla del renderer permite cambiar densidad y encuadre durante el
 * feedback visual sin tocar lifecycle, cámara ni liberación GPU. */

import * as THREE from 'three';
import { createBroadleaf, createConifer, createFigure, createPond, createRock, type ForestMaterials } from './forest-models';

export function buildForestDiorama(scene: THREE.Scene, materials: ForestMaterials): void {
  const base = new THREE.Mesh(new THREE.BoxGeometry(26, 0.7, 20), materials.pale);
  base.position.y = -0.05;
  base.receiveShadow = true;
  scene.add(base, new THREE.GridHelper(24, 16, 0x777777, 0xc8c8c2));

  const trees: Array<[number, number, number, boolean]> = [
    [-9, -6, 1.1, true], [-7, -5, 0.9, true], [-10, -2, 1, true], [-7.5, -1, 1.2, true],
    [-10, 3, 0.85, true], [-6.8, 4.5, 1, true], [-3.8, -6, 0.9, false], [-2, -4.5, 1.15, false],
    [-4, -1.5, 0.8, false], [-2.2, 1.5, 1, false], [0.2, 3.8, 1.1, false], [3.2, 5.5, 0.85, false],
    [7.2, 5, 1.05, true], [9.5, 3, 0.9, true], [8.8, -2.5, 1.2, true], [6.5, -5.5, 0.8, true],
  ];
  trees.forEach(([x, z, scale, conifer]) => {
    const tree = conifer ? createConifer(materials, scale) : createBroadleaf(materials, scale);
    tree.position.set(x, 0.3, z);
    scene.add(tree);
  });

  const largePond = createPond(materials, 3.3, 2.1);
  largePond.position.set(4.4, 0.34, -1.2);
  const smallPond = createPond(materials, 1.6, 1.05);
  smallPond.position.set(-3.8, 0.34, 5.8);
  scene.add(largePond, smallPond);

  [[-1, -0.5], [1.2, -2], [3, -4], [0.4, 1.6]].forEach(([x, z], index) => {
    const step = new THREE.Mesh(new THREE.BoxGeometry(3.8, 0.08, 1.25), materials.paper);
    step.position.set(x, 0.34, z);
    step.rotation.y = -0.3 + index * 0.08;
    scene.add(step);
  });
  [[-5.3, 5], [5.8, 3.6], [9.2, -6.2], [-6.8, 0.7]].forEach(([x, z], index) => {
    const rock = createRock(materials, 0.7 + index * 0.08);
    rock.position.x = x;
    rock.position.z = z;
    scene.add(rock);
  });
  const local = createFigure(materials);
  local.position.set(0.2, 0.35, -0.5);
  const remote = createFigure(materials, true);
  remote.position.set(4.8, 0.35, 4.2);
  scene.add(local, remote);
}
