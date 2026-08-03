/* GAME-01 — Gestión admin del catálogo de assets del Bosque.
 * Listado completo (activas e inactivas), alta y actualización de las
 * opciones allowlisted del catálogo de assets. Solo invocable con sesión
 * admin: la frontera 401/403 la resuelve el backend vía `AdminUser` + CSRF.
 * [297A-61] Replica el patrón de GameCharacterAdminService con el contrato
 * de 297A-60 (categorías del mapa, id máx 48, etiqueta máx 64). */

import { generatedFetcher, unwrapGeneratedResponse, type GeneratedResponse } from '../api/client';

/** Categorías del contrato del mapa (assetVersionId -> category). */
export const GAME_ASSET_CATEGORIES: ReadonlyArray<string> = [
  'terrain',
  'tree',
  'rock',
  'water',
  'character',
  'generic',
];

/** Entrada completa del catálogo, visible solo para admin: incluye el estado
 * (activa/inactiva) y la fecha de creación que el contrato público oculta. */
export interface GameAssetAdminEntry {
  id: string;
  displayName: string;
  category: string;
  isActive: boolean;
  createdAt: string;
}

export interface CreateAdminAssetInput {
  id: string;
  displayName: string;
  category: string;
}

export interface UpdateAdminAssetInput {
  displayName: string;
  category: string;
  isActive: boolean;
}

const ID_PATTERN = /^[a-z0-9-]{1,48}$/;

const MAX_LABEL_CHARS = 64;

export function isValidAdminAssetId(id: string): boolean {
  return ID_PATTERN.test(id);
}

/** Mismo contrato que el backend: 1–64 caracteres, sin controles. */
export function isValidAdminAssetLabel(label: string): boolean {
  const trimmed = label.trim();
  if (trimmed !== label || trimmed.length === 0) return false;
  if (Array.from(trimmed).length > MAX_LABEL_CHARS) return false;
  return !Array.from(trimmed).some((character) => /\p{Cc}/u.test(character));
}

export function isValidAdminAssetCategory(category: string): boolean {
  return GAME_ASSET_CATEGORIES.includes(category);
}

export function isValidAdminAssetEntry(value: unknown): value is GameAssetAdminEntry {
  if (typeof value !== 'object' || value === null) return false;
  const entry = value as Record<string, unknown>;
  const keys = Object.keys(entry).sort();
  if (keys.join(',') !== 'category,createdAt,displayName,id,isActive') return false;
  return typeof entry.id === 'string'
    && ID_PATTERN.test(entry.id)
    && typeof entry.displayName === 'string'
    && isValidAdminAssetLabel(entry.displayName)
    && typeof entry.category === 'string'
    && isValidAdminAssetCategory(entry.category)
    && typeof entry.isActive === 'boolean'
    && typeof entry.createdAt === 'string';
}

export const GameAssetAdminService = {
  async listAll(options?: { signal?: AbortSignal }): Promise<GameAssetAdminEntry[]> {
    const response = await generatedFetcher<GeneratedResponse<unknown>>(
      '/api/admin/game/assets',
      { method: 'GET', signal: options?.signal },
    );
    const payload = unwrapGeneratedResponse<unknown>(response, [200]);
    if (!Array.isArray(payload) || !payload.every(isValidAdminAssetEntry)) {
      throw new Error('Catálogo admin de assets inválido');
    }
    return payload;
  },

  async create(
    input: CreateAdminAssetInput,
    options?: { signal?: AbortSignal },
  ): Promise<GameAssetAdminEntry> {
    const response = await generatedFetcher<GeneratedResponse<unknown>>(
      '/api/admin/game/assets',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(input),
        signal: options?.signal,
      },
    );
    const payload = unwrapGeneratedResponse<unknown>(response, [200]);
    if (!isValidAdminAssetEntry(payload)) {
      throw new Error('Respuesta de creación inválida');
    }
    return payload;
  },

  async update(
    id: string,
    input: UpdateAdminAssetInput,
    options?: { signal?: AbortSignal },
  ): Promise<GameAssetAdminEntry> {
    const response = await generatedFetcher<GeneratedResponse<unknown>>(
      `/api/admin/game/assets/${encodeURIComponent(id)}`,
      {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(input),
        signal: options?.signal,
      },
    );
    const payload = unwrapGeneratedResponse<unknown>(response, [200]);
    if (!isValidAdminAssetEntry(payload)) {
      throw new Error('Respuesta de actualización inválida');
    }
    return payload;
  },
};
