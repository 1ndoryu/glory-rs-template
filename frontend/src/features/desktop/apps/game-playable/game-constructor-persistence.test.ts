import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  terrainOptionsPreset,
  WORLD_PALETTE_DEFAULTS,
  type TerrainLayer,
} from '../../../game-core';
import {
  clearConstructorState,
  CONSTRUCTOR_STORAGE_KEY,
  normalizePanelState,
  loadConstructorState,
  saveConstructorState,
} from './game-constructor-persistence';

describe('persistencia del constructor de mundo', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    window.localStorage.clear();
  });

  it('guarda y restaura opciones y modo en la clave versionada', () => {
    const options = terrainOptionsPreset('archipielago');
    expect(saveConstructorState({ version: 1, options, mode: 'suave', camera: 'primera' })).toBe(true);
    expect(window.localStorage.getItem(CONSTRUCTOR_STORAGE_KEY)).not.toBeNull();

    const restored = loadConstructorState();
    expect(restored).toEqual({ version: 1, options, mode: 'suave', camera: 'primera' });
  });

  it('guarda y restaura el modo de cámara con el constructor (138A-7)', () => {
    const options = terrainOptionsPreset('isla');
    expect(saveConstructorState({ version: 1, options, mode: 'bloques', camera: 'tercera' })).toBe(true);
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'bloques', camera: 'tercera' });
  });

  it('devuelve null si no hay estado guardado', () => {
    expect(loadConstructorState()).toBeNull();
  });

  it('devuelve null con JSON corrupto o versión desconocida (fail-closed)', () => {
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, '{no es json');
    expect(loadConstructorState()).toBeNull();

    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({ version: 99 }));
    expect(loadConstructorState()).toBeNull();
  });

  it('devuelve null si las opciones son inválidas', () => {
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options: { ...terrainOptionsPreset('isla'), width: 7 },
      mode: 'bloques',
    }));
    expect(loadConstructorState()).toBeNull();
  });

  it('devuelve null con opciones parciales (no rellena defaults en silencio)', () => {
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options: { shape: 'isla' },
      mode: 'bloques',
    }));
    expect(loadConstructorState()).toBeNull();
  });

  it('un modo inválido cae al default bloques conservando las opciones', () => {
    const options = terrainOptionsPreset('valle');
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options,
      mode: 'wireframe',
    }));
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'bloques', camera: 'libre' });
  });

  it('el modo histórico actual cae a bloques al restaurar (138A-6)', () => {
    const options = terrainOptionsPreset('isla');
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options,
      mode: 'actual',
    }));
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'bloques', camera: 'libre' });
  });

  it('una cámara ausente o inválida cae a libre conservando opciones y modo (138A-7)', () => {
    const options = terrainOptionsPreset('valle');
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options,
      mode: 'suave',
      camera: 'orbit',
    }));
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'suave', camera: 'libre' });

    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({ version: 1, options, mode: 'suave' }));
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'suave', camera: 'libre' });
  });

  it('guarda y restaura la paleta y el estado del panel (138A-8)', () => {
    const options = terrainOptionsPreset('isla');
    const palette = { ...WORLD_PALETTE_DEFAULTS, sky: 0x123456 };
    const panel = { collapsed: true, side: 'left' as const, width: 360 };
    expect(saveConstructorState({ version: 1, options, mode: 'bloques', camera: 'libre', palette, panel }))
      .toBe(true);
    expect(loadConstructorState()).toEqual({
      version: 1,
      options,
      mode: 'bloques',
      camera: 'libre',
      palette,
      panel,
    });
  });

  it('una paleta o panel inválido se omiten sin bloquear la restauración (fail-closed)', () => {
    const options = terrainOptionsPreset('isla');
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options,
      mode: 'suave',
      camera: 'libre',
      palette: { ...WORLD_PALETTE_DEFAULTS, grass: -5 },
      panel: { collapsed: 'si', side: 'right', width: 9999 },
    }));
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'suave', camera: 'libre' });
  });

  it('normalizePanelState recorta el ancho a un decimal y valida lado/colapso', () => {
    expect(normalizePanelState({ collapsed: false, side: 'right', width: 311.17 }))
      .toEqual({ collapsed: false, side: 'right', width: 311.2 });
    expect(normalizePanelState({ collapsed: true, side: 'top', width: 320 })).toBeNull();
    expect(normalizePanelState({ collapsed: true, side: 'right', width: 100 })).toBeNull();
    expect(normalizePanelState(null)).toBeNull();
  });

  it('save devuelve false y load null si localStorage falla', () => {
    const blocked: Storage = {
      getItem: () => { throw new Error('denied'); },
      setItem: () => { throw new Error('quota'); },
      removeItem: () => { throw new Error('quota'); },
      clear: () => { throw new Error('quota'); },
      key: () => null,
      get length() { return 0; },
    };
    const getter = vi.spyOn(window, 'localStorage', 'get').mockReturnValue(blocked);
    expect(saveConstructorState({ version: 1, options: terrainOptionsPreset('isla'), mode: 'bloques', camera: 'libre' }))
      .toBe(false);
    expect(loadConstructorState()).toBeNull();
    getter.mockRestore();
  });

  it('clearConstructorState elimina la clave sin romper si no existe', () => {
    saveConstructorState({ version: 1, options: terrainOptionsPreset('isla'), mode: 'bloques', camera: 'libre' });
    clearConstructorState();
    expect(window.localStorage.getItem(CONSTRUCTOR_STORAGE_KEY)).toBeNull();
    clearConstructorState();
  });

  it('guarda y restaura el stack de capas del editor de mapa (138A-9)', () => {
    const options = terrainOptionsPreset('isla');
    const layers: readonly TerrainLayer[] = [
      {
        id: 'capa-path-1',
        name: 'Camino pintado',
        enabled: true,
        kind: 'path',
        shape: { kind: 'painted', cells: [[2, 3], [4, 5]] },
        falloff: 'smooth',
        falloffRadius: 1,
        bias: 1,
        blend: 'set',
        hardness: 0.5,
      },
      {
        id: 'capa-elevation-1',
        name: 'Elevación pintada',
        enabled: true,
        kind: 'elevation',
        shape: { kind: 'circle', cx: 0, cz: 0, radius: 3 },
        falloff: 'gauss',
        falloffRadius: 1.5,
        bias: 1,
        blend: 'add',
        height: 2,
        elevationMode: 'delta',
      },
    ];
    expect(saveConstructorState({ version: 1, options, mode: 'suave', camera: 'libre', layers })).toBe(true);
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'suave', camera: 'libre', layers });
  });

  it('capas inválidas se omiten sin bloquear la restauración del resto (138A-9)', () => {
    const options = terrainOptionsPreset('isla');
    const layers = [
      { id: 'mala', kind: 'path' }, // sin shape/hardness/blend...
      {
        id: 'capa-sand-1',
        name: 'Arena pintada',
        enabled: true,
        kind: 'sand',
        shape: { kind: 'painted', cells: [[0, 0]] },
        falloff: 'hard',
        falloffRadius: 0.5,
        bias: 0.8,
        blend: 'set',
        hardness: 0.4,
      },
    ];
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options,
      mode: 'bloques',
      camera: 'tercera',
      palette: { ...WORLD_PALETTE_DEFAULTS, sky: 0xabcdef },
      layers,
    }));
    expect(loadConstructorState()).toEqual({
      version: 1,
      options,
      mode: 'bloques',
      camera: 'tercera',
      palette: { ...WORLD_PALETTE_DEFAULTS, sky: 0xabcdef },
    });
  });

  it('guarda y restaura las opciones del generador de pasto (138A-10)', () => {
    const options = terrainOptionsPreset('isla');
    const grass = { enabled: true, density: 0.65, size: 1.2, color: 0x7ec850 };
    expect(saveConstructorState({ version: 1, options, mode: 'suave', camera: 'libre', grass }))
      .toBe(true);
    expect(loadConstructorState()).toEqual({
      version: 1,
      options,
      mode: 'suave',
      camera: 'libre',
      grass,
    });
  });

  it('opciones de pasto inválidas se omiten sin bloquear el resto (138A-10)', () => {
    const options = terrainOptionsPreset('isla');
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify({
      version: 1,
      options,
      mode: 'bloques',
      camera: 'tercera',
      grass: { enabled: true, density: 7, size: 1, color: 0x86c65c },
    }));
    expect(loadConstructorState()).toEqual({
      version: 1,
      options,
      mode: 'bloques',
      camera: 'tercera',
    });
  });
});
