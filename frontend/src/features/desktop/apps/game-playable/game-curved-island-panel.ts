/* GAME-01 — Panel temporal de configuración del terreno (como el #panel de la
 * referencia). Vive dentro de la ventana del Bosque, usa los tokens B&W del OS
 * y se destruye junto al runtime. Permite ajustar la curva del mundo, la
 * lluvia, los props, el follow de cámara y regenerar la isla. */

import { createEl } from '../../../../utils/dom';
import type { TerrainOptions } from '../../../game-core';
import {
  mountWorldConstructor,
  type WorldConstructorControls,
  type WorldConstructorSection,
} from './game-world-constructor';

export interface CurvedIslandPanelControls {
  readonly setCurvature: (down: number, pull: number) => void;
  readonly setRain: (amount: number) => void;
  readonly setPropsVisible: (visible: boolean) => void;
  readonly setCameraFollow: (follow: boolean) => void;
  readonly regenerate: () => void;
  /** [138A-1] Comparador de estilos del toolkit (opcional: solo si existe). */
  readonly setTerrainMode?: (mode: 'actual' | 'bloques' | 'suave') => void;
  /** [138A-4] Constructor de mundo (opcional: solo si existe). */
  readonly constructor?: WorldConstructorControls;
}

export interface CurvedIslandPanel {
  readonly setPick: (pick: { i: number; j: number; level: number | null } | null) => void;
  /** [138A-1] Línea de métricas del comparador (vacío la oculta). */
  readonly setTerrainMetrics: (text: string) => void;
  /** [138A-4] Línea de métricas del constructor (vacío la oculta). */
  readonly setConstructorStats: (text: string) => void;
  /** [138A-4] Sincroniza los controles del constructor con unas opciones. */
  readonly setConstructorOptions: (options: TerrainOptions) => void;
  /** [138A-4] Marca el segmento de estilo activo sin disparar el control. */
  readonly setTerrainMode: (mode: 'actual' | 'bloques' | 'suave') => void;
  readonly destroy: () => void;
}

const PRESETS: readonly { readonly key: string; readonly label: string; readonly down: number; readonly pull: number }[] = [
  { key: 'flat', label: 'Plano', down: 0, pull: 0 },
  { key: 'cozy', label: 'Cozy', down: 0.010, pull: 0.004 },
  { key: 'marble', label: 'Mármol', down: 0.026, pull: 0.012 },
];

export function mountCurvedIslandPanel(
  host: HTMLElement,
  controls: CurvedIslandPanelControls,
): CurvedIslandPanel {
  const stats = createEl('p', { className: 'juegoPanelTerreno__stats', textContent: '' });
  let metricsEl: HTMLParagraphElement | null = null;
  let constructorSection: WorldConstructorSection | null = null;

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

  /* El panel no debe orbitar la cámara ni disparar el picking del terreno: sus
   * eventos de puntero/rueda no burbujean al host de la escena. */
  for (const type of ['pointerdown', 'pointermove', 'pointerup', 'pointercancel', 'wheel'] as const) {
    panel.addEventListener(type, (event) => event.stopPropagation());
  }

  /* --- grupo: curva del mundo --- */
  const grupoCurva = createEl('div', { className: 'juegoPanelTerreno__grupo' });
  grupoCurva.appendChild(createEl('p', { className: 'juegoPanelTerreno__tituloGrupo', textContent: 'Curva del mundo' }));

  const sliders = buildSliderPair(grupoCurva, controls.setCurvature);

  const segPresets = createEl('div', { className: 'juegoPanelTerreno__segmentos' });
  for (const preset of PRESETS) {
    const button = createEl('button', {
      className: 'juegoPanelTerreno__segmento',
      textContent: preset.label,
      type: 'button',
    });
    if (preset.key === 'cozy') button.classList.add('juegoPanelTerreno__segmento--activo');
    button.addEventListener('click', () => {
      for (const sibling of Array.from(segPresets.children)) {
        sibling.classList.toggle('juegoPanelTerreno__segmento--activo', sibling === button);
      }
      sliders.set(preset.down, preset.pull);
    });
    segPresets.appendChild(button);
  }
  grupoCurva.appendChild(segPresets);
  body.appendChild(grupoCurva);

  /* --- grupo: isla --- */
  const grupoIsla = createEl('div', { className: 'juegoPanelTerreno__grupo' });
  grupoIsla.appendChild(createEl('p', { className: 'juegoPanelTerreno__tituloGrupo', textContent: 'Isla' }));

  const rainRow = createEl('div', { className: 'juegoPanelTerreno__fila' });
  const rainLabel = createEl('label', { className: 'juegoPanelTerreno__rangoLabel', textContent: 'Lluvia' });
  const rainValue = createEl('span', { className: 'juegoPanelTerreno__rangoValor', textContent: '60%' });
  rainLabel.appendChild(rainValue);
  const rainInput = buildRange(0, 100, 1, 60, (v) => {
    rainValue.textContent = `${Math.round(v)}%`;
    controls.setRain(v / 100);
  });
  rainRow.append(rainLabel, rainInput);
  grupoIsla.appendChild(rainRow);

  const propsCheck = buildCheck(grupoIsla, 'Árboles y rocas', true, (checked) => {
    controls.setPropsVisible(checked);
  });

  const followCheck = buildCheck(grupoIsla, 'Cámara sigue', true, (checked) => {
    controls.setCameraFollow(checked);
  });

  const regenButton = createEl('button', {
    className: 'juegoPanelTerreno__boton',
    type: 'button',
    textContent: 'Crecer nueva isla',
  });
  regenButton.addEventListener('click', () => {
    controls.regenerate();
  });
  grupoIsla.appendChild(regenButton);
  body.appendChild(grupoIsla);

  /* --- grupo: constructor de mundo (138A-4) --- */
  if (controls.constructor) {
    constructorSection = mountWorldConstructor(body, controls.constructor);
  }

  /* --- grupo: comparador de estilos (138A-1) --- */
  let setTerrainMode: (mode: 'actual' | 'bloques' | 'suave') => void = () => {};
  if (controls.setTerrainMode) {
    const grupoComparador = createEl('div', { className: 'juegoPanelTerreno__grupo' });
    grupoComparador.appendChild(createEl('p', {
      className: 'juegoPanelTerreno__tituloGrupo',
      textContent: 'Comparar estilos',
    }));
    const segEstilos = createEl('div', { className: 'juegoPanelTerreno__segmentos' });
    const estilos: readonly { key: 'actual' | 'bloques' | 'suave'; label: string }[] = [
      { key: 'actual', label: 'Actual' },
      { key: 'bloques', label: 'Bloques' },
      { key: 'suave', label: 'Suave' },
    ];
    const styleButtons = new Map<'actual' | 'bloques' | 'suave', HTMLButtonElement>();
    for (const estilo of estilos) {
      const button = createEl('button', {
        className: 'juegoPanelTerreno__segmento',
        textContent: estilo.label,
        type: 'button',
      });
      if (estilo.key === 'actual') button.classList.add('juegoPanelTerreno__segmento--activo');
      button.addEventListener('click', () => {
        setTerrainMode(estilo.key);
        controls.setTerrainMode?.(estilo.key);
      });
      styleButtons.set(estilo.key, button);
      segEstilos.appendChild(button);
    }
    setTerrainMode = (mode) => {
      for (const [key, button] of styleButtons) {
        button.classList.toggle('juegoPanelTerreno__segmento--activo', key === mode);
      }
    };
    grupoComparador.appendChild(segEstilos);
    metricsEl = createEl('p', { className: 'juegoPanelTerreno__statsLine', textContent: '' });
    grupoComparador.appendChild(metricsEl);
    body.appendChild(grupoComparador);
  }

  host.appendChild(panel);
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
      if (metricsEl) metricsEl.textContent = text;
    },
    setConstructorStats: (text) => {
      constructorSection?.setStats(text);
    },
    setConstructorOptions: (options) => {
      constructorSection?.applyOptions(options);
    },
    setTerrainMode,
    destroy: () => {
      panel.remove();
      stats.remove();
      metricsEl?.remove();
      constructorSection?.destroy();
      void propsCheck;
      void followCheck;
    },
  };
}

/* createEl no asigna min/max/step: se fijan como propiedades para que el
 * rango respete el paso y no se redondee al default del navegador (step=1). */
function buildRange(
  min: number,
  max: number,
  step: number,
  initial: number,
  onChange: (value: number) => void,
): HTMLInputElement {
  const input = createEl('input', { className: 'juegoPanelTerreno__rango', type: 'range' });
  input.min = String(min);
  input.max = String(max);
  input.step = String(step);
  input.value = String(initial);
  input.addEventListener('input', () => onChange(Number(input.value)));
  return input;
}

interface SliderRef {
  readonly input: HTMLInputElement;
  readonly valueEl: HTMLSpanElement;
  readonly fmt: (v: number) => string;
}

function buildSliderPair(
  grupo: HTMLElement,
  setCurvature: (down: number, pull: number) => void,
): { readonly set: (down: number, pull: number) => void } {
  let down = 0.010;
  let pull = 0.004;
  const refs: SliderRef[] = [];

  const make = (
    label: string,
    min: number,
    max: number,
    step: number,
    initial: number,
    fmt: (v: number) => string,
    apply: (v: number) => void,
  ): void => {
    const row = createEl('div', { className: 'juegoPanelTerreno__fila' });
    const labelEl = createEl('label', { className: 'juegoPanelTerreno__rangoLabel', textContent: label });
    const valueEl = createEl('span', { className: 'juegoPanelTerreno__rangoValor', textContent: fmt(initial) });
    labelEl.appendChild(valueEl);
    const input = buildRange(min, max, step, initial, (v) => {
      valueEl.textContent = fmt(v);
      apply(v);
    });
    refs.push({ input, valueEl, fmt });
    row.append(labelEl, input);
    grupo.appendChild(row);
  };

  make('Curva abajo', 0, 0.03, 0.0005, down, v => v.toFixed(4), (v) => { down = v; setCurvature(down, pull); });
  make('Tirón horizonte', 0, 0.016, 0.0002, pull, v => v.toFixed(4), (v) => { pull = v; setCurvature(down, pull); });

  return {
    set: (d, p) => {
      down = d;
      pull = p;
      refs[0].input.value = String(d);
      refs[0].valueEl.textContent = refs[0].fmt(d);
      refs[1].input.value = String(p);
      refs[1].valueEl.textContent = refs[1].fmt(p);
      setCurvature(d, p);
    },
  };
}

function buildCheck(
  grupo: HTMLElement,
  label: string,
  initial: boolean,
  onChange: (checked: boolean) => void,
): HTMLLabelElement {
  const wrap = createEl('label', { className: 'juegoPanelTerreno__check' });
  const input = createEl('input', { type: 'checkbox' });
  input.checked = initial;
  input.addEventListener('change', () => onChange(input.checked));
  wrap.append(input, document.createTextNode(label));
  grupo.appendChild(wrap);
  return wrap;
}
