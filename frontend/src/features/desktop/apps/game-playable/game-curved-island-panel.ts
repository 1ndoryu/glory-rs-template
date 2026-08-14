/* GAME-01 — Panel temporal de configuración del terreno (como el #panel de la
 * referencia). Vive dentro de la ventana del Bosque, usa los tokens B&W del OS
 * y se destruye junto al runtime. Permite ajustar la curva del mundo, la
 * lluvia, los props, el follow de cámara y regenerar la isla. Cuando existe el
 * Constructor de mundo, el panel exterior es el rail de iconos y los grupos de
 * la isla son secciones suyas; sin constructor conserva el panel clásico. */

import { Camera, Layers, Waves } from 'lucide';
import { createEl } from '../../../../utils/dom';
import type { RenderStyle, TerrainOptions } from '../../../game-core';
import { DEFAULT_CAMERA_MODE, type CameraMode } from './game-camera-modes';
import {
  mountWorldConstructor,
  type WorldConstructorSection,
  type WorldConstructorSubpanel,
} from './game-world-constructor';
import {
  buildCamaraGroup,
  buildEstilosGroup,
  buildIslaGroup,
  type CurvedIslandPanelControls,
} from './game-curved-island-controls';

export interface CurvedIslandPanel {
  readonly setPick: (pick: { i: number; j: number; level: number | null } | null) => void;
  /** [138A-1] Línea de métricas del comparador (vacío la oculta). */
  readonly setTerrainMetrics: (text: string) => void;
  /** [138A-4] Línea de métricas del constructor (vacío la oculta). */
  readonly setConstructorStats: (text: string) => void;
  /** [138A-4] Sincroniza los controles del constructor con unas opciones. */
  readonly setConstructorOptions: (options: TerrainOptions) => void;
  /** [138A-4] Marca el segmento de estilo activo sin disparar el control. */
  readonly setTerrainMode: (mode: RenderStyle) => void;
  /** [138A-7] Marca el segmento de cámara activo sin disparar el control. */
  readonly setCameraMode: (mode: CameraMode) => void;
  readonly destroy: () => void;
}

export function mountCurvedIslandPanel(
  host: HTMLElement,
  controls: CurvedIslandPanelControls,
): CurvedIslandPanel {
  const stats = createEl('p', { className: 'juegoPanelTerreno__stats', textContent: '' });
  let metricsEl: HTMLParagraphElement | null = null;
  let currentMode: RenderStyle = 'bloques';
  let currentCameraMode: CameraMode = DEFAULT_CAMERA_MODE;
  let estilosSetActive: ((mode: RenderStyle) => void) | null = null;
  let camaraSetActive: ((mode: CameraMode) => void) | null = null;
  let constructorSection: WorldConstructorSection | null = null;
  let legacyPanel: HTMLElement | null = null;

  const mountEstilos = (container: HTMLElement): void => {
    const grupo = buildEstilosGroup(container, controls, currentMode);
    metricsEl = grupo.metricsEl;
    estilosSetActive = grupo.setActive;
  };

  const mountCamara = (container: HTMLElement): void => {
    const grupo = buildCamaraGroup(container, controls, currentCameraMode);
    camaraSetActive = grupo.setActive;
  };

  if (controls.worldConstructor) {
    /* [138A-5] El rail de iconos es el panel exterior; "Isla" y "Estilos"
     * son secciones suyas, no al revés. */
    const extraPanels: WorldConstructorSubpanel[] = [
      { key: 'isla', label: 'Isla', icon: Waves, build: (c) => buildIslaGroup(c, controls) },
    ];
    if (controls.setTerrainMode) {
      extraPanels.push({ key: 'estilos', label: 'Estilos', icon: Layers, build: mountEstilos });
    }
    if (controls.setCameraMode) {
      extraPanels.push({ key: 'camara', label: 'Cámara', icon: Camera, build: mountCamara });
    }
    constructorSection = mountWorldConstructor(host, controls.worldConstructor, {
      title: 'Constructor',
      extraPanels,
    });
  } else {
    /* Legacy sin constructor: se conserva el panel clásico del terreno. */
    const panel = createEl('section', {
      className: 'juegoPanelTerreno',
      ariaLabel: 'Configuración temporal del terreno',
    });
    const header = createEl('header', { className: 'juegoPanelTerreno__cabecera' });
    header.appendChild(createEl('span', { className: 'juegoPanelTerreno__titulo', textContent: 'Terreno' }));
    const body = createEl('div', { className: 'juegoPanelTerreno__cuerpo' });
    panel.append(header, body);
    header.addEventListener('click', () => {
      panel.classList.toggle('juegoPanelTerreno--cerrado');
    });
    for (const type of ['pointerdown', 'pointermove', 'pointerup', 'pointercancel', 'wheel'] as const) {
      panel.addEventListener(type, (event) => event.stopPropagation());
    }
    buildIslaGroup(body, controls);
    if (controls.setTerrainMode) mountEstilos(body);
    host.appendChild(panel);
    legacyPanel = panel;
  }
  host.appendChild(stats);

  return {
    setPick: (pick) => {
      stats.textContent = pick
        ? pick.level === null
          ? `terreno suave · ${pick.i},${pick.j}`
          : `bloque ${pick.i},${pick.j} · nivel ${pick.level}`
        : '';
    },
    setTerrainMetrics: (text) => {
      if (metricsEl?.isConnected) metricsEl.textContent = text;
    },
    setConstructorStats: (text) => {
      constructorSection?.setStats(text);
    },
    setConstructorOptions: (options) => {
      constructorSection?.applyOptions(options);
    },
    setTerrainMode: (mode) => {
      currentMode = mode;
      estilosSetActive?.(mode);
    },
    setCameraMode: (mode) => {
      currentCameraMode = mode;
      camaraSetActive?.(mode);
    },
    destroy: () => {
      legacyPanel?.remove();
      stats.remove();
      constructorSection?.destroy();
      metricsEl = null;
      estilosSetActive = null;
      camaraSetActive = null;
      currentMode = 'bloques';
      currentCameraMode = DEFAULT_CAMERA_MODE;
    },
  };
}
