import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Waves } from 'lucide';
import { terrainOptionsPreset, type TerrainOptions } from '../../../game-core';
import {
  mountWorldConstructor,
  type WorldConstructorControls,
} from './game-world-constructor';

describe('sección constructor de mundo (rail de iconos)', () => {
  let host: HTMLElement;
  let onGenerate: ReturnType<typeof vi.fn<(options: TerrainOptions) => void>>;
  let onExport: ReturnType<typeof vi.fn<() => void>>;
  let onImport: ReturnType<typeof vi.fn<(text: string) => void>>;
  let onChange: ReturnType<typeof vi.fn<(options: TerrainOptions) => void>>;
  let controls: WorldConstructorControls;

  beforeEach(() => {
    host = document.createElement('section');
    document.body.appendChild(host);
    onGenerate = vi.fn();
    onExport = vi.fn();
    onImport = vi.fn();
    onChange = vi.fn();
    controls = { onGenerate, onExport, onImport, onChange };
  });

  afterEach(() => {
    host.remove();
  });

  const clickText = (text: string): void => {
    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find(candidate => candidate.textContent === text);
    expect(button, `botón "${text}"`).toBeDefined();
    button?.click();
  };

  const railButton = (label: string): HTMLButtonElement => {
    const button = host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
    expect(button, `icono "${label}"`).toBeDefined();
    return button as HTMLButtonElement;
  };

  it('genera con las opciones por defecto al pulsar Generar mundo', () => {
    mountWorldConstructor(host, controls);
    clickText('Generar mundo');
    expect(onGenerate).toHaveBeenCalledTimes(1);
    expect(onGenerate.mock.calls[0][0]).toMatchObject({
      shape: 'isla',
      width: 48,
      depth: 32,
      cellSize: 1,
      maxHeight: 4,
      waterLevel: 0,
      vegetationDensity: 1,
    });
  });

  it('usa la forma activa y el seed editado al generar', () => {
    mountWorldConstructor(host, controls);
    clickText('Continente');
    const seed = host.querySelector<HTMLInputElement>('input[type="number"]');
    expect(seed).not.toBeNull();
    if (seed) seed.value = '4242';
    seed?.dispatchEvent(new Event('input'));
    clickText('Generar mundo');
    expect(onGenerate.mock.calls[0][0]).toMatchObject({ shape: 'continente', seed: 4242 });
  });

  it('emite onChange en tiempo real al editar un valor', () => {
    mountWorldConstructor(host, controls);
    const seed = host.querySelector<HTMLInputElement>('input[type="number"]');
    expect(seed).not.toBeNull();
    if (seed) seed.value = '9001';
    seed?.dispatchEvent(new Event('input'));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0]).toMatchObject({ seed: 9001 });
  });

  it('emite onChange al cambiar forma y vegetación sin pulsar Generar', () => {
    mountWorldConstructor(host, controls);
    clickText('Archipiélago');
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0]).toMatchObject({ shape: 'archipielago' });

    railButton('Mundo/Estilo').click();
    const density = Array.from(host.querySelectorAll<HTMLInputElement>('input[type="range"]'))
      .find(input => input.closest('.juegoPanelTerreno__fila')?.textContent?.includes('Vegetación'));
    expect(density).toBeDefined();
    if (!density) return;
    density.value = '25';
    density.dispatchEvent(new Event('input'));
    expect(onChange).toHaveBeenCalledTimes(2);
    expect(onChange.mock.calls[1][0]).toMatchObject({ vegetationDensity: 0.25 });
  });

  it('mantiene un solo subpanel abierto y conmuta con los iconos del rail', () => {
    mountWorldConstructor(host, controls);
    const openPanels = (): string[] => Array.from(host.querySelectorAll<HTMLElement>('.juegoConstructor__subpanel'))
      .map(panel => panel.getAttribute('aria-label') ?? '');

    expect(openPanels()).toEqual(['Terreno']);
    expect(railButton('Terreno').getAttribute('aria-pressed')).toBe('true');

    railButton('Mundo/Estilo').click();
    expect(openPanels()).toEqual(['Mundo/Estilo']);
    expect(railButton('Terreno').getAttribute('aria-pressed')).toBe('false');
    expect(railButton('Mundo/Estilo').getAttribute('aria-pressed')).toBe('true');

    railButton('Mundo/Estilo').click();
    expect(openPanels()).toEqual([]);
  });

  it('el subpanel Mundo cambia dimensiones y celda con onChange', () => {
    mountWorldConstructor(host, controls);
    railButton('Mundo/Estilo').click();
    const selects = Array.from(host.querySelectorAll<HTMLSelectElement>('select'));
    const ancho = selects.find(select => select.closest('.juegoPanelTerreno__fila')?.textContent?.includes('Ancho'));
    expect(ancho).toBeDefined();
    if (!ancho) return;
    ancho.value = '64';
    ancho.dispatchEvent(new Event('change'));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0]).toMatchObject({ width: 64 });
  });

  it('applyOptions sincroniza los controles activos antes de generar', () => {
    const section = mountWorldConstructor(host, controls);
    section.applyOptions(terrainOptionsPreset('valle'));
    clickText('Generar mundo');
    expect(onGenerate.mock.calls[0][0]).toMatchObject({
      shape: 'valle',
      seed: terrainOptionsPreset('valle').seed,
    });
  });

  it('muestra métricas con setStats y limpia el DOM en destroy', () => {
    const section = mountWorldConstructor(host, controls);
    section.setStats('mundo · chunks 6');
    const stats = host.querySelector('.juegoPanelTerreno__statsLine');
    expect(stats?.textContent).toContain('mundo · chunks 6');
    section.destroy();
    expect(host.querySelector('.juegoConstructor')).toBeNull();
  });

  it('exporta e importa JSON desde el input de archivo', async () => {
    const section = mountWorldConstructor(host, controls);
    clickText('Exportar JSON');
    expect(onExport).toHaveBeenCalledTimes(1);

    const fileInput = host.querySelector<HTMLInputElement>('input[type="file"]');
    expect(fileInput).not.toBeNull();
    const json = JSON.stringify({
      format: 'wandorius-map',
      version: 1,
      options: terrainOptionsPreset('archipielago'),
      map: null,
    });
    if (!fileInput) return;
    const file = new File([json], 'mundo.json', { type: 'application/json' });
    Object.defineProperty(fileInput, 'files', { value: [file], configurable: true });
    fileInput.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(onImport).toHaveBeenCalledTimes(1));
    expect(onImport).toHaveBeenCalledWith(json);
    section.destroy();
  });

  it('registra subpaneles extra del rail sin tocar el núcleo (OCP)', () => {
    mountWorldConstructor(host, controls, {
      title: 'Constructor',
      extraPanels: [{
        key: 'isla',
        label: 'Isla',
        icon: Waves,
        build: (container) => {
          container.appendChild(document.createElement('p')).textContent = 'Curva del mundo';
        },
      }],
    });

    const isla = railButton('Isla');
    expect(isla).toBeDefined();
    isla.click();
    const subpanel = host.querySelector<HTMLElement>('.juegoConstructor__subpanel');
    expect(subpanel?.getAttribute('aria-label')).toBe('Isla');
    expect(subpanel?.textContent).toContain('Curva del mundo');
    expect(isla.getAttribute('aria-pressed')).toBe('true');
  });
});
