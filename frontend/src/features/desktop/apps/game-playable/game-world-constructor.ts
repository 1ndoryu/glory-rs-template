/* GAME-01 / 138A-4 — Sección Constructor de mundo del panel del Bosque.
 * Edita TerrainOptions (forma, seed, tamaño, altura, agua, costa, warp,
 * octaves, celda y densidad de vegetación), genera/exporta/importa el mundo
 * y muestra métricas de presupuesto. Solo DOM + contrato puro de game-core:
 * la generación y el 3D viven en la escena. */

import { createEl } from '../../../../utils/dom';
import {
  SHAPE_PRESETS,
  TERRAIN_OPTIONS_DEFAULTS,
  TERRAIN_OPTIONS_LIMITS,
  normalizeTerrainOptions,
  type ShapePreset,
  type TerrainOptions,
} from '../../../game-core';

export interface WorldConstructorControls {
  readonly onGenerate: (options: TerrainOptions) => void;
  readonly onExport: () => void;
  readonly onImport: (text: string) => void;
}

export interface WorldConstructorSection {
  readonly setStats: (text: string) => void;
  readonly applyOptions: (options: TerrainOptions) => void;
  readonly destroy: () => void;
}

const DIMENSION_OPTIONS: readonly number[] = [16, 32, 48, 64, 96, 128];
const CELL_SIZE_OPTIONS: readonly number[] = [0.5, 1, 1.5, 2];

interface RangeRef {
  readonly valueEl: HTMLSpanElement;
  readonly fmt: (value: number) => string;
}

export function mountWorldConstructor(
  host: HTMLElement,
  controls: WorldConstructorControls,
): WorldConstructorSection {
  const defaults = { ...TERRAIN_OPTIONS_DEFAULTS };
  let state: TerrainOptions = normalizeTerrainOptions(defaults);
  const rangeRefs = new Map<HTMLInputElement, RangeRef>();

  const grupo = createEl('div', { className: 'juegoPanelTerreno__grupo' });
  grupo.appendChild(createEl('p', {
    className: 'juegoPanelTerreno__tituloGrupo',
    textContent: 'Constructor',
  }));

  /* --- forma: segmentos de presets --- */
  const segForma = createEl('div', { className: 'juegoPanelTerreno__segmentos' });
  const shapeButtons = new Map<ShapePreset, HTMLButtonElement>();
  for (const preset of SHAPE_PRESETS) {
    const button = createEl('button', {
      className: 'juegoPanelTerreno__segmento',
      textContent: preset.label,
      type: 'button',
    });
    if (preset.key === defaults.shape) button.classList.add('juegoPanelTerreno__segmento--activo');
    button.addEventListener('click', () => {
      for (const sibling of Array.from(segForma.children)) {
        sibling.classList.toggle('juegoPanelTerreno__segmento--activo', sibling === button);
      }
      state = { ...state, shape: preset.key };
    });
    shapeButtons.set(preset.key, button);
    segForma.appendChild(button);
  }
  grupo.appendChild(segForma);

  /* --- seed: número + aleatorio --- */
  const seedRow = createEl('div', { className: 'juegoPanelTerreno__fila' });
  seedRow.appendChild(createEl('label', {
    className: 'juegoPanelTerreno__rangoLabel',
    textContent: 'Seed',
  }));
  const seedInput = createEl('input', { className: 'juegoPanelTerreno__entrada', type: 'number' });
  seedInput.min = String(TERRAIN_OPTIONS_LIMITS.minSeed);
  seedInput.max = String(TERRAIN_OPTIONS_LIMITS.maxSeed);
  seedInput.step = '1';
  seedInput.value = String(defaults.seed);
  seedInput.addEventListener('input', () => {
    const seed = Math.floor(Number(seedInput.value));
    if (Number.isFinite(seed)) state = { ...state, seed };
  });
  const randomButton = createEl('button', {
    className: 'juegoPanelTerreno__boton',
    type: 'button',
    textContent: 'Aleatorio',
  });
  randomButton.addEventListener('click', () => {
    const seed = Math.floor(Math.random() * 100000);
    seedInput.value = String(seed);
    state = { ...state, seed };
  });
  const seedControls = createEl('div', { className: 'juegoPanelTerreno__doble' });
  seedControls.append(seedInput, randomButton);
  seedRow.appendChild(seedControls);
  grupo.appendChild(seedRow);

  /* --- tamaño: ancho/profundidad/celda --- */
  const makeSelect = (
    label: string,
    options: readonly number[],
    initial: number,
    onChange: (value: number) => void,
  ): { readonly row: HTMLDivElement; readonly select: HTMLSelectElement } => {
    const row = createEl('div', { className: 'juegoPanelTerreno__fila' });
    row.appendChild(createEl('label', { className: 'juegoPanelTerreno__rangoLabel', textContent: label }));
    const select = createEl('select', { className: 'juegoPanelTerreno__entrada' });
    for (const option of options) {
      select.appendChild(createEl('option', {
        value: String(option),
        textContent: String(option),
      }));
    }
    select.value = String(initial);
    select.addEventListener('change', () => onChange(Number(select.value)));
    row.appendChild(select);
    return { row, select };
  };

  const widthSel = makeSelect('Ancho', DIMENSION_OPTIONS, defaults.width, (v) => { state = { ...state, width: v }; });
  const depthSel = makeSelect('Profundo', DIMENSION_OPTIONS, defaults.depth, (v) => { state = { ...state, depth: v }; });
  const cellSel = makeSelect('Celda', CELL_SIZE_OPTIONS, defaults.cellSize, (v) => { state = { ...state, cellSize: v }; });
  const dimsRow = createEl('div', { className: 'juegoPanelTerreno__doble' });
  dimsRow.append(widthSel.row, depthSel.row);
  grupo.appendChild(dimsRow);
  grupo.appendChild(cellSel.row);

  /* --- rangos continuos --- */
  const makeRange = (
    label: string,
    min: number,
    max: number,
    step: number,
    initial: number,
    fmt: (value: number) => string,
    onChange: (value: number) => void,
  ): HTMLInputElement => {
    const row = createEl('div', { className: 'juegoPanelTerreno__fila' });
    const labelEl = createEl('label', { className: 'juegoPanelTerreno__rangoLabel' });
    const valueEl = createEl('span', { className: 'juegoPanelTerreno__rangoValor', textContent: fmt(initial) });
    labelEl.append(document.createTextNode(label), valueEl);
    const input = createEl('input', { className: 'juegoPanelTerreno__rango', type: 'range' });
    input.min = String(min);
    input.max = String(max);
    input.step = String(step);
    input.value = String(initial);
    input.addEventListener('input', () => {
      const value = Number(input.value);
      valueEl.textContent = fmt(value);
      onChange(value);
    });
    rangeRefs.set(input, { valueEl, fmt });
    row.append(labelEl, input);
    grupo.appendChild(row);
    return input;
  };

  const maxHeightInput = makeRange(
    'Altura máx',
    TERRAIN_OPTIONS_LIMITS.minMaxHeight,
    TERRAIN_OPTIONS_LIMITS.maxMaxHeight,
    0.5,
    defaults.maxHeight,
    v => v.toFixed(1),
    v => { state = { ...state, maxHeight: v }; },
  );
  const waterLevelInput = makeRange(
    'Nivel agua',
    -2,
    4,
    0.1,
    defaults.waterLevel,
    v => v.toFixed(1),
    v => { state = { ...state, waterLevel: v }; },
  );
  const coastInput = makeRange(
    'Costa',
    TERRAIN_OPTIONS_LIMITS.minCoast,
    TERRAIN_OPTIONS_LIMITS.maxCoast - 0.04,
    0.01,
    defaults.coast,
    v => v.toFixed(2),
    v => { state = { ...state, coast: v }; },
  );
  const warpInput = makeRange(
    'Warp',
    TERRAIN_OPTIONS_LIMITS.minWarp,
    TERRAIN_OPTIONS_LIMITS.maxWarp - 0.04,
    0.01,
    defaults.warp,
    v => v.toFixed(2),
    v => { state = { ...state, warp: v }; },
  );
  const octavesInput = makeRange(
    'Octaves',
    TERRAIN_OPTIONS_LIMITS.minOctaves,
    TERRAIN_OPTIONS_LIMITS.maxOctaves,
    1,
    defaults.octaves,
    v => String(v),
    v => { state = { ...state, octaves: v }; },
  );
  const densityInput = makeRange(
    'Vegetación',
    0,
    100,
    1,
    defaults.vegetationDensity * 100,
    v => `${Math.round(v)}%`,
    v => { state = { ...state, vegetationDensity: v / 100 }; },
  );

  /* --- acciones: generar / exportar / importar --- */
  const generateButton = createEl('button', {
    className: 'juegoPanelTerreno__boton',
    type: 'button',
    textContent: 'Generar mundo',
  });
  const statsEl = createEl('p', { className: 'juegoPanelTerreno__statsLine', textContent: '' });
  generateButton.addEventListener('click', () => {
    try {
      controls.onGenerate(normalizeTerrainOptions(state));
    } catch (error) {
      statsEl.textContent = error instanceof Error ? `error: ${error.message}` : 'opciones inválidas';
    }
  });
  grupo.appendChild(generateButton);

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
    if (file) file.text().then(controls.onImport, () => undefined);
  });
  const fileRow = createEl('div', { className: 'juegoPanelTerreno__doble' });
  fileRow.append(fileInput, exportButton, importButton);
  grupo.appendChild(fileRow);
  grupo.appendChild(statsEl);

  host.appendChild(grupo);

  const setRange = (input: HTMLInputElement, value: number): void => {
    input.value = String(value);
    const ref = rangeRefs.get(input);
    if (ref) ref.valueEl.textContent = ref.fmt(value);
  };

  return {
    setStats: (text) => { statsEl.textContent = text; },
    applyOptions: (options) => {
      state = normalizeTerrainOptions(options);
      for (const [key, button] of shapeButtons) {
        button.classList.toggle('juegoPanelTerreno__segmento--activo', key === state.shape);
      }
      seedInput.value = String(state.seed);
      widthSel.select.value = String(state.width);
      depthSel.select.value = String(state.depth);
      cellSel.select.value = String(state.cellSize);
      setRange(maxHeightInput, state.maxHeight);
      setRange(waterLevelInput, state.waterLevel);
      setRange(coastInput, state.coast);
      setRange(warpInput, state.warp);
      setRange(octavesInput, state.octaves);
      setRange(densityInput, state.vegetationDensity * 100);
    },
    destroy: () => { grupo.remove(); },
  };
}
