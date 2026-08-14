import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { terrainOptionsPreset, type TerrainOptions } from '../../../game-core';
import {
  mountWorldConstructor,
  type WorldConstructorControls,
} from './game-world-constructor';

describe('sección constructor de mundo', () => {
  let host: HTMLElement;
  let onGenerate: ReturnType<typeof vi.fn<(options: TerrainOptions) => void>>;
  let onExport: ReturnType<typeof vi.fn<() => void>>;
  let onImport: ReturnType<typeof vi.fn<(text: string) => void>>;
  let controls: WorldConstructorControls;

  beforeEach(() => {
    host = document.createElement('section');
    document.body.appendChild(host);
    onGenerate = vi.fn();
    onExport = vi.fn();
    onImport = vi.fn();
    controls = { onGenerate, onExport, onImport };
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

  it('usa la forma activa y el seed aleatorio al generar', () => {
    mountWorldConstructor(host, controls);
    clickText('Continente');
    const seed = host.querySelector<HTMLInputElement>('input[type="number"]');
    expect(seed).not.toBeNull();
    if (seed) seed.value = '4242';
    seed?.dispatchEvent(new Event('input'));
    clickText('Generar mundo');
    expect(onGenerate.mock.calls[0][0]).toMatchObject({ shape: 'continente', seed: 4242 });
  });

  it('applyOptions sincroniza los controles antes de generar', () => {
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
    expect(host.querySelector('.juegoPanelTerreno__grupo')).toBeNull();
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
});
