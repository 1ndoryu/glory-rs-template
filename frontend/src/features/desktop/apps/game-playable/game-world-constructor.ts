/* GAME-01 / 138A-5 — Toolkit de edición del Constructor de mundo.
 * Sustituye la sección única "Constructor" por un rail lateral de iconos
 * (tipo Blender): cada icono abre un subpanel pequeño con opciones
 * agrupadas y un solo subpanel activo a la vez. Cada mutación emite
 * `onChange` (tiempo real con debounce en la escena); Generar/Exportar/
 * Importar y las métricas quedan fijos bajo el rail. Solo DOM + contrato
 * puro de game-core: la generación y el 3D viven en la escena. */

import { createElement, Globe, Mountain, type IconNode } from 'lucide';
import { createEl } from '../../../../utils/dom';
import {
  SHAPE_PRESETS,
  TERRAIN_OPTIONS_DEFAULTS,
  TERRAIN_OPTIONS_LIMITS,
  normalizeTerrainOptions,
  type ShapePreset,
  type TerrainOptions,
} from '../../../game-core';
import {
  createRangeControl,
  createSelectControl,
  createSeedRow,
  createSegmentControl,
} from './game-constructor-controls';

export interface WorldConstructorControls {
  readonly onGenerate: (options: TerrainOptions) => void;
  readonly onExport: () => void;
  readonly onImport: (text: string) => void;
  /** [138A-5] Tiempo real: cada cambio de valor emite las opciones válidas. */
  readonly onChange?: (options: TerrainOptions) => void;
}

export interface WorldConstructorSection {
  readonly setStats: (text: string) => void;
  readonly applyOptions: (options: TerrainOptions) => void;
  readonly destroy: () => void;
}

const DIMENSION_OPTIONS: readonly number[] = [16, 32, 48, 64, 96, 128];
const CELL_SIZE_OPTIONS: readonly number[] = [0.5, 1, 1.5, 2];

interface SubpanelDefinition {
  readonly key: string;
  readonly label: string;
  readonly icon: IconNode;
  /** Construye el subpanel dentro de `container` y registra su sincronizador. */
  readonly build: (container: HTMLElement, ctx: ConstructorPanelContext) => void;
}

interface ConstructorPanelContext {
  /** Opciones actuales del constructor (mismo objeto hasta el próximo commit). */
  readonly state: TerrainOptions;
  /** Aplica una mutación sobre las opciones y emite tiempo real. */
  readonly commit: (next: TerrainOptions) => void;
  /** Registra un sincronizador que `applyOptions` ejecuta al restaurar. */
  readonly sync: (fn: () => void) => void;
}

export function mountWorldConstructor(
  host: HTMLElement,
  controls: WorldConstructorControls,
): WorldConstructorSection {
  const defaults = { ...TERRAIN_OPTIONS_DEFAULTS };
  let state: TerrainOptions = normalizeTerrainOptions(defaults);
  const syncers: Array<() => void> = [];
  const sync = (fn: () => void): void => { syncers.push(fn); };
  const commit = (next: TerrainOptions): void => {
    state = next;
    emitChange();
  };
  const emitChange = (): void => {
    controls.onChange?.(normalizeTerrainOptions(state));
  };
  const ctx: ConstructorPanelContext = {
    get state() { return state; },
    commit,
    sync,
  };

  const root = createEl('div', { className: 'juegoConstructor' });
  const cuerpo = createEl('div', { className: 'juegoConstructor__cuerpo' });
  const rail = createEl('nav', {
    className: 'juegoConstructor__rail',
    ariaLabel: 'Herramientas del constructor',
  });
  const lienzo = createEl('div', { className: 'juegoConstructor__lienzo' });
  cuerpo.append(rail, lienzo);

  /* --- acciones fijas: generar / exportar / importar + métricas --- */
  const statsEl = createEl('p', { className: 'juegoPanelTerreno__statsLine', textContent: '' });
  const generateButton = createEl('button', {
    className: 'juegoPanelTerreno__boton',
    type: 'button',
    textContent: 'Generar mundo',
  });
  generateButton.addEventListener('click', () => {
    try {
      controls.onGenerate(normalizeTerrainOptions(state));
    } catch (error) {
      statsEl.textContent = error instanceof Error ? `error: ${error.message}` : 'opciones inválidas';
    }
  });

  const exportButton = createEl('button', {
    className: 'juegoPanelTerreno__boton',
    type: 'button',
    textContent: 'Exportar JSON',
  });
  exportButton.addEventListener('click', () => controls.onExport());

  const fileInput = createEl('input', {
    className: 'juegoPanelTerreno__entrada',
    type: 'file',
    accept: '.json,application/json',
  });
  fileInput.hidden = true;
  const importButton = createEl('button', {
    className: 'juegoPanelTerreno__boton',
    type: 'button',
    textContent: 'Importar JSON',
  });
  importButton.addEventListener('click', () => fileInput.click());
  fileInput.addEventListener('change', () => {
    const file = fileInput.files?.[0];
    fileInput.value = '';
    if (file) {
      file.text().then(controls.onImport, () => {
        statsEl.textContent = 'error: no se pudo leer el archivo';
      });
    }
  });

  const acciones = createEl('div', { className: 'juegoConstructor__acciones' });
  acciones.append(fileInput, generateButton, exportButton, importButton, statsEl);

  /* --- registro de subpaneles (OCP: 138A-6/7/8 añaden Cámara, Objetos,
   * Color, Textura y Assets sin tocar el rail). --- */
  const panels: readonly SubpanelDefinition[] = [
    { key: 'terreno', label: 'Terreno', icon: Mountain, build: buildTerrenoPanel },
    { key: 'mundo', label: 'Mundo/Estilo', icon: Globe, build: buildMundoPanel },
  ];

  const railButtons = new Map<string, HTMLButtonElement>();
  let activePanel: SubpanelDefinition | null = null;
  let subpanelEl: HTMLElement | null = null;

  const openPanel = (panel: SubpanelDefinition): void => {
    if (activePanel?.key === panel.key) {
      activePanel = null;
      subpanelEl?.remove();
      subpanelEl = null;
      syncers.length = 0;
      for (const button of railButtons.values()) button.setAttribute('aria-pressed', 'false');
      return;
    }
    activePanel = panel;
    subpanelEl?.remove();
    syncers.length = 0;
    subpanelEl = createEl('div', {
      className: 'juegoConstructor__subpanel',
      role: 'region',
      ariaLabel: panel.label,
    });
    subpanelEl.appendChild(createEl('p', {
      className: 'juegoPanelTerreno__tituloGrupo',
      textContent: panel.label,
    }));
    panel.build(subpanelEl, ctx);
    lienzo.appendChild(subpanelEl);
    for (const [key, button] of railButtons) {
      button.setAttribute('aria-pressed', String(key === panel.key));
    }
  };

  for (const panel of panels) {
    const button = createEl('button', {
      className: 'juegoConstructor__icono',
      type: 'button',
      title: panel.label,
      'aria-label': panel.label,
      'aria-pressed': 'false',
    });
    button.appendChild(createEl('span', { ariaHidden: 'true' }, createElement(panel.icon)));
    button.addEventListener('click', () => {
      openPanel(panel);
      button.classList.toggle('juegoConstructor__icono--activo', activePanel?.key === panel.key);
    });
    railButtons.set(panel.key, button);
    rail.appendChild(button);
  }

  root.append(cuerpo, acciones);
  host.appendChild(root);
  openPanel(panels[0]);
  railButtons.get(panels[0].key)?.classList.add('juegoConstructor__icono--activo');

  return {
    setStats: (text) => { statsEl.textContent = text; },
    applyOptions: (options) => {
      state = normalizeTerrainOptions(options);
      for (const syncer of syncers) syncer();
    },
    destroy: () => { root.remove(); },
  };
}

/* --- subpanel Terreno: forma, seed y rangos del relieve --- */
function buildTerrenoPanel(
  container: HTMLElement,
  ctx: ConstructorPanelContext,
): void {
  const { commit, sync } = ctx;

  /* forma: segmentos de presets */
  const shapeSegments = createSegmentControl<ShapePreset>(
    SHAPE_PRESETS,
    ctx.state.shape,
    (shape) => commit({ ...ctx.state, shape }),
  );
  sync(() => shapeSegments.setActive(ctx.state.shape));
  container.appendChild(shapeSegments.container);

  const seed = createSeedRow(
    TERRAIN_OPTIONS_LIMITS.minSeed,
    TERRAIN_OPTIONS_LIMITS.maxSeed,
    ctx.state.seed,
    (seedValue) => commit({ ...ctx.state, seed: seedValue }),
  );
  sync(() => seed.setValue(ctx.state.seed));
  container.appendChild(seed.row);

  /* rangos continuos del relieve */
  const height = createRangeControl(
    'Altura máx', TERRAIN_OPTIONS_LIMITS.minMaxHeight, TERRAIN_OPTIONS_LIMITS.maxMaxHeight,
    0.5, ctx.state.maxHeight, v => v.toFixed(1),
    (maxHeight) => commit({ ...ctx.state, maxHeight }),
  );
  const water = createRangeControl(
    'Nivel agua', -2, 4, 0.1, ctx.state.waterLevel, v => v.toFixed(1),
    (waterLevel) => commit({ ...ctx.state, waterLevel }),
  );
  const coast = createRangeControl(
    'Costa', TERRAIN_OPTIONS_LIMITS.minCoast, TERRAIN_OPTIONS_LIMITS.maxCoast - 0.04,
    0.01, ctx.state.coast, v => v.toFixed(2),
    (coast) => commit({ ...ctx.state, coast }),
  );
  const warp = createRangeControl(
    'Warp', TERRAIN_OPTIONS_LIMITS.minWarp, TERRAIN_OPTIONS_LIMITS.maxWarp - 0.04,
    0.01, ctx.state.warp, v => v.toFixed(2),
    (warp) => commit({ ...ctx.state, warp }),
  );
  const octaves = createRangeControl(
    'Octaves', TERRAIN_OPTIONS_LIMITS.minOctaves, TERRAIN_OPTIONS_LIMITS.maxOctaves,
    1, ctx.state.octaves, v => String(v),
    (octaves) => commit({ ...ctx.state, octaves }),
  );
  for (const control of [height, water, coast, warp, octaves]) {
    sync(() => control.setValue(
      control === height ? ctx.state.maxHeight
        : control === water ? ctx.state.waterLevel
        : control === coast ? ctx.state.coast
        : control === warp ? ctx.state.warp
        : ctx.state.octaves,
    ));
    container.appendChild(control.row);
  }
}

/* --- subpanel Mundo/Estilo: dimensiones, celda y vegetación --- */
function buildMundoPanel(
  container: HTMLElement,
  ctx: ConstructorPanelContext,
): void {
  const { commit, sync } = ctx;

  const widthSel = createSelectControl('Ancho', DIMENSION_OPTIONS, ctx.state.width,
    (width) => commit({ ...ctx.state, width }));
  const depthSel = createSelectControl('Profundo', DIMENSION_OPTIONS, ctx.state.depth,
    (depth) => commit({ ...ctx.state, depth }));
  const dimsRow = createEl('div', { className: 'juegoPanelTerreno__doble' });
  dimsRow.append(widthSel.row, depthSel.row);
  container.appendChild(dimsRow);

  const cellSel = createSelectControl('Celda', CELL_SIZE_OPTIONS, ctx.state.cellSize,
    (cellSize) => commit({ ...ctx.state, cellSize }));
  container.appendChild(cellSel.row);

  const density = createRangeControl(
    'Vegetación', 0, 100, 1, ctx.state.vegetationDensity * 100, v => `${Math.round(v)}%`,
    (v) => commit({ ...ctx.state, vegetationDensity: v / 100 }),
  );
  container.appendChild(density.row);

  sync(() => widthSel.setValue(ctx.state.width));
  sync(() => depthSel.setValue(ctx.state.depth));
  sync(() => cellSel.setValue(ctx.state.cellSize));
  sync(() => density.setValue(ctx.state.vegetationDensity * 100));
}
