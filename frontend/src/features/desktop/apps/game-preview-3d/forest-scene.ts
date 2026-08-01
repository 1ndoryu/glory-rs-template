/* [GAME-01-VIS-3D] Escena Three.js aislada del shell.
 * Centraliza cámara, render loop y liberación GPU para que cerrar la app no
 * deje controles, observers ni contextos WebGL activos. */

import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { buildForestDiorama } from './forest-diorama';
import type { ForestMaterials } from './forest-models';

export interface ForestSceneHandle {
  resetCamera: () => void;
  destroy: () => void;
}

export interface ForestSceneOptions {
  readonly onContextLost?: () => void;
}

const CAMERA_POSITION = new THREE.Vector3(18, 17, 18);
const CAMERA_TARGET = new THREE.Vector3(0, 1.2, 0);

export function mountForestScene(host: HTMLElement, options: ForestSceneOptions = {}): ForestSceneHandle {
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0xeeeeea);
  scene.fog = new THREE.Fog(0xeeeeea, 25, 48);

  const camera = new THREE.OrthographicCamera(-12, 12, 9, -9, 0.1, 80);
  const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'low-power' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.domElement.setAttribute('aria-label', 'Diorama tridimensional del bosque');
  host.append(renderer.domElement);

  const controls = new OrbitControls(camera, renderer.domElement);
  let contextLost = false;
  const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
  const syncMotionPreference = (): void => {
    controls.enableDamping = !reducedMotionQuery.matches;
    controls.dampingFactor = reducedMotionQuery.matches ? 0 : 0.08;
  };
  const handleContextLost = (event: Event): void => {
    event.preventDefault();
    contextLost = true;
    renderer.setAnimationLoop(null);
    options.onContextLost?.();
  };
  const handleMotionPreference = (): void => syncMotionPreference();
  syncMotionPreference();
  reducedMotionQuery.addEventListener('change', handleMotionPreference);
  renderer.domElement.addEventListener('webglcontextlost', handleContextLost, false);
  controls.minZoom = 0.7;
  controls.maxZoom = 2.4;
  controls.minPolarAngle = Math.PI * 0.2;
  controls.maxPolarAngle = Math.PI * 0.47;
  controls.screenSpacePanning = true;

  const materials: ForestMaterials = {
    ink: new THREE.MeshToonMaterial({ color: 0x111111 }),
    paper: new THREE.MeshToonMaterial({ color: 0xf8f8f4 }),
    pale: new THREE.MeshToonMaterial({ color: 0xd7d7d1 }),
    middle: new THREE.MeshToonMaterial({ color: 0x8d8d88 }),
    water: new THREE.MeshToonMaterial({ color: 0x55555a }),
    lines: new THREE.LineBasicMaterial({ color: 0x050505 }),
  };

  buildForestDiorama(scene, materials);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x555555, 2.2));
  const sun = new THREE.DirectionalLight(0xffffff, 3.4);
  sun.position.set(-8, 18, 10);
  sun.castShadow = true;
  sun.shadow.mapSize.set(1024, 1024);
  scene.add(sun);

  const resetCamera = (): void => {
    camera.position.copy(CAMERA_POSITION);
    camera.zoom = 1;
    camera.updateProjectionMatrix();
    controls.target.copy(CAMERA_TARGET);
    controls.update();
  };
  resetCamera();

  let visible = true;
  let destroyed = false;
  const renderFrame = (): void => {
    if (destroyed || contextLost) return;
    controls.update();
    renderer.render(scene, camera);
  };
  const syncLoop = (): void => renderer.setAnimationLoop(!destroyed && !contextLost && visible && !document.hidden ? renderFrame : null);
  const resize = (): void => {
    if (destroyed || contextLost) return;
    const width = Math.max(host.clientWidth, 1);
    const height = Math.max(host.clientHeight, 1);
    const halfHeight = 10;
    const halfWidth = halfHeight * (width / height);
    Object.assign(camera, { left: -halfWidth, right: halfWidth, top: halfHeight, bottom: -halfHeight });
    camera.updateProjectionMatrix();
    renderer.setSize(width, height, false);
    renderFrame();
  };
  const resizeObserver = new ResizeObserver(resize);
  const visibilityObserver = new IntersectionObserver(([entry]) => {
    visible = entry?.isIntersecting ?? false;
    syncLoop();
  });
  const handleVisibility = (): void => syncLoop();
  resizeObserver.observe(host);
  visibilityObserver.observe(host);
  document.addEventListener('visibilitychange', handleVisibility);
  resize();
  syncLoop();

  return {
    resetCamera,
    destroy: () => {
      if (destroyed) return;
      destroyed = true;
      renderer.setAnimationLoop(null);
      document.removeEventListener('visibilitychange', handleVisibility);
      reducedMotionQuery.removeEventListener('change', handleMotionPreference);
      renderer.domElement.removeEventListener('webglcontextlost', handleContextLost);
      resizeObserver.disconnect();
      visibilityObserver.disconnect();
      controls.dispose();
      disposeScene(scene);
      renderer.dispose();
      renderer.forceContextLoss();
      renderer.domElement.remove();
    },
  };
}

function disposeScene(scene: THREE.Scene): void {
  const geometries = new Set<THREE.BufferGeometry>();
  const materials = new Set<THREE.Material>();
  scene.traverse((object) => {
    if (object instanceof THREE.Mesh || object instanceof THREE.LineSegments) {
      geometries.add(object.geometry);
      const assigned = Array.isArray(object.material) ? object.material : [object.material];
      assigned.forEach(material => materials.add(material));
    }
  });
  geometries.forEach(geometry => geometry.dispose());
  materials.forEach(material => material.dispose());
  scene.clear();
}
