/* 138A-5 — Persistencia local del Constructor de mundo.
 * Guarda las últimas opciones y el modo de render en localStorage (clave
 * versionada) para que la recarga no pierda valores. Sin backend: la fuente
 * portable sigue siendo el export/import JSON del panel. */

import {
  normalizeTerrainOptions,
  validateTerrainOptions,
  type TerrainOptions,
} from '../../../game-core';

export const CONSTRUCTOR_STORAGE_KEY = 'wandorius:constructor:v1';

/** Modo de render que el comparador muestra al recargar (138A-6 lo restringe). */
export type ConstructorTerrainMode = 'actual' | 'bloques' | 'suave';

export interface ConstructorPersistedState {
  readonly version: 1;
  readonly options: TerrainOptions;
  readonly mode: ConstructorTerrainMode;
}

const VALID_MODES: readonly ConstructorTerrainMode[] = ['actual', 'bloques', 'suave'];

/** Persiste el estado; devuelve false si el storage no está disponible. */
export function saveConstructorState(state: ConstructorPersistedState): boolean {
  try {
    window.localStorage.setItem(CONSTRUCTOR_STORAGE_KEY, JSON.stringify(state));
    return true;
  } catch {
    /* Quota/privacidad: la edición en vivo sigue funcionando sin persistir. */
    return false;
  }
}

/** Restaura el estado guardado; null si no existe o es inválido (fail-closed).
 *  Un modo ausente/inválido cae al default `bloques` conservando las opciones. */
export function loadConstructorState(): ConstructorPersistedState | null {
  try {
    const raw = window.localStorage.getItem(CONSTRUCTOR_STORAGE_KEY);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return null;
    const record = parsed as Record<string, unknown>;
    if (record.version !== 1) return null;
    /* Fail-closed estricto: las opciones deben venir COMPLETAS y válidas;
     * un payload parcial no se rellena con defaults en silencio. */
    const rawOptions = record.options;
    if (typeof rawOptions !== 'object' || rawOptions === null || Array.isArray(rawOptions)) return null;
    if (validateTerrainOptions(rawOptions).length > 0) return null;
    const options = normalizeTerrainOptions(rawOptions);
    const mode = typeof record.mode === 'string' && VALID_MODES.includes(record.mode as ConstructorTerrainMode)
      ? (record.mode as ConstructorTerrainMode)
      : 'bloques';
    return { version: 1, options, mode };
  } catch {
    /* JSON corrupto o storage no disponible: no se puede restaurar. */
    return null;
  }
}

/** Elimina el estado guardado (usado en teardown de tests y reset manual). */
export function clearConstructorState(): void {
  try {
    window.localStorage.removeItem(CONSTRUCTOR_STORAGE_KEY);
  } catch {
    /* Nada que limpiar si el storage no existe. */
  }
}
