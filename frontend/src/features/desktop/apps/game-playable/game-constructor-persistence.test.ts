import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { terrainOptionsPreset } from '../../../game-core';
import {
  clearConstructorState,
  CONSTRUCTOR_STORAGE_KEY,
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
    expect(saveConstructorState({ version: 1, options, mode: 'suave' })).toBe(true);
    expect(window.localStorage.getItem(CONSTRUCTOR_STORAGE_KEY)).not.toBeNull();

    const restored = loadConstructorState();
    expect(restored).toEqual({ version: 1, options, mode: 'suave' });
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
    expect(loadConstructorState()).toEqual({ version: 1, options, mode: 'bloques' });
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
    expect(saveConstructorState({ version: 1, options: terrainOptionsPreset('isla'), mode: 'bloques' }))
      .toBe(false);
    expect(loadConstructorState()).toBeNull();
    getter.mockRestore();
  });

  it('clearConstructorState elimina la clave sin romper si no existe', () => {
    saveConstructorState({ version: 1, options: terrainOptionsPreset('isla'), mode: 'bloques' });
    clearConstructorState();
    expect(window.localStorage.getItem(CONSTRUCTOR_STORAGE_KEY)).toBeNull();
    clearConstructorState();
  });
});
