/* 138A-5 — Persistencia local del Constructor de mundo.
 * Guarda las últimas opciones, el modo de render y el modo de cámara en
 * localStorage (clave versionada) para que la recarga no pierda valores.
 * Sin backend: la fuente portable sigue siendo el export/import JSON. */

import {
  normalizeTerrainOptions,
  validateTerrainOptions,
  type RenderStyle,
  type TerrainOptions,
  normalizeWorldPalette,
  validateWorldPalette,
  type WorldPalette,
} from '../../../game-core';
import {
  DEFAULT_CAMERA_MODE,
  isCameraMode,
  type CameraMode,
} from './game-camera-modes';

export const CONSTRUCTOR_STORAGE_KEY = 'wandorius:constructor:v1';

/* [138A-8] Estado de la ventana lateral del Constructor: colapsado, lado y
 * ancho redimensionable. Límites sensatos para el panel completo. */
export const CONSTRUCTOR_PANEL_MIN_WIDTH = 240;
export const CONSTRUCTOR_PANEL_MAX_WIDTH = 520;
export const CONSTRUCTOR_PANEL_DEFAULT_WIDTH = 320;

export interface ConstructorPanelState {
  readonly collapsed: boolean;
  readonly side: 'left' | 'right';
  readonly width: number;
}

/** Normaliza un estado de panel válido; null ante cualquier valor malo
 *  (fail-closed: no se rellenan defaults en silencio). */
export function normalizePanelState(value: unknown): ConstructorPanelState | null {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.collapsed !== 'boolean') return null;
  if (record.side !== 'left' && record.side !== 'right') return null;
  if (typeof record.width !== 'number' || !Number.isFinite(record.width)
    || record.width < CONSTRUCTOR_PANEL_MIN_WIDTH || record.width > CONSTRUCTOR_PANEL_MAX_WIDTH) {
    return null;
  }
  return {
    collapsed: record.collapsed,
    side: record.side,
    width: Math.round(record.width * 10) / 10,
  };
}

export interface ConstructorPersistedState {
  readonly version: 1;
  readonly options: TerrainOptions;
  /** Modo de render que el comparador muestra al recargar (unión única
   *  `RenderStyle` compartida con el panel; 138A-6). */
  readonly mode: RenderStyle;
  /** [138A-7] Modo de cámara restaurado al recargar (fail-closed a `libre`). */
  readonly camera: CameraMode;
  /** [138A-8] Paleta del mundo persistida; ausente si nunca se guardó. */
  readonly palette?: WorldPalette;
  /** [138A-8] Estado de la ventana del Constructor (colapso/lado/ancho). */
  readonly panel?: ConstructorPanelState;
}

const VALID_MODES: readonly RenderStyle[] = ['bloques', 'suave'];

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
 *  Un modo ausente/inválido (incluido el histórico `actual`) cae al default
 *  `bloques` conservando las opciones; la cámara ausente/inválida cae a
 *  `libre` (compatibilidad con estados guardados antes de 138A-7). */
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
    const mode = typeof record.mode === 'string' && VALID_MODES.includes(record.mode as RenderStyle)
      ? (record.mode as RenderStyle)
      : 'bloques';
    const camera = isCameraMode(record.camera) ? record.camera : DEFAULT_CAMERA_MODE;
    /* [138A-8] La paleta y el panel son opcionales y solo se restauran si
     * son VÁLIDOS; payloads corruptos caen a omitidos sin bloquear la carga. */
    const palette = validateWorldPalette(record.palette).length === 0
      ? normalizeWorldPalette(record.palette)
      : undefined;
    const panel = normalizePanelState(record.panel) ?? undefined;
    return {
      version: 1,
      options,
      mode,
      camera,
      ...(palette ? { palette } : {}),
      ...(panel ? { panel } : {}),
    };
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
