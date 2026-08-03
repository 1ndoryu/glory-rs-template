/* GAME-01 — Administración de mapas del Bosque.
 * Carga el snapshot activo (`GET /api/game/maps/{mapId}`) y publica una nueva
 * versión inmutable (`POST /api/admin/game/maps`) con `expectedVersion`
 * (0 para la primera publicación; después debe coincidir con la activa).
 * [297A-64] El editor de mapa 2D consume este servicio: el envelope público se
 * valida estrictamente y el documento se revalida con el contrato puro de
 * `game-core` antes de editar o publicar. La frontera 401/403 la resuelve el
 * backend vía `AdminUser` + CSRF; 404 significa que aún no hay mapa publicado. */

import { generatedFetcher, unwrapGeneratedResponse, ApiError, type GeneratedResponse } from '../api/client';
import {
  validateMapVersion,
  MAP_VERSION_SCHEMA,
  type MapVersion,
  type MapValidationIssue,
} from '../features/game-core';

/** Id canónico del mapa del Bosque. Debe coincidir con `GAME_MAP_ID` en el
 * entorno del backend para que el runtime realtime sirva este mismo mapa. */
export const GAME_MAP_ID = 'bosque';

/** Envelope público del snapshot activo (Orval GameMapVersionPublic). */
export interface GameMapVersionPublic {
  mapId: string;
  version: number;
  schemaVersion: number;
  contentHash: string;
  publishedAt: string;
  document: unknown;
}

/** Borrador resuelto por el editor: el documento ya validado + la versión
 * activa que servirá como `expectedVersion` al publicar (0 sin publicaciones). */
export interface LoadedGameMap {
  document: MapVersion;
  activeVersion: number;
}

export function isValidGameMapVersionPublic(value: unknown): value is GameMapVersionPublic {
  if (typeof value !== 'object' || value === null) return false;
  const envelope = value as Record<string, unknown>;
  const keys = Object.keys(envelope).sort();
  if (keys.join(',') !== 'contentHash,document,mapId,publishedAt,schemaVersion,version') return false;
  return typeof envelope.mapId === 'string'
    && envelope.mapId.trim().length > 0
    && typeof envelope.version === 'number' && Number.isInteger(envelope.version) && envelope.version >= 0
    && envelope.schemaVersion === MAP_VERSION_SCHEMA
    && typeof envelope.contentHash === 'string' && envelope.contentHash.trim().length > 0
    && typeof envelope.publishedAt === 'string'
    && envelope.publishedAt.length > 0;
}

/** Valida el envelope y el documento del snapshot activo. */
export function parseActiveMapEnvelope(value: unknown): { envelope: GameMapVersionPublic; document: MapVersion; issues: readonly MapValidationIssue[] } | null {
  if (!isValidGameMapVersionPublic(value)) return null;
  const documentIssues = validateMapVersion(value.document);
  if (documentIssues.length > 0) return { envelope: value, document: value.document as MapVersion, issues: documentIssues };
  return { envelope: value, document: value.document as MapVersion, issues: [] };
}

export const GameMapAdminService = {
  /** Snapshot activo del mapa. `null` si no hay versión publicada (404). */
  async getActive(
    mapId: string,
    options?: { signal?: AbortSignal },
  ): Promise<LoadedGameMap | null> {
    const response = await generatedFetcher<GeneratedResponse<unknown>>(
      `/api/game/maps/${encodeURIComponent(mapId)}`,
      { method: 'GET', signal: options?.signal },
    );
    try {
      const payload = unwrapGeneratedResponse<unknown>(response, [200]);
      const parsed = parseActiveMapEnvelope(payload);
      if (!parsed) throw new Error('Envelope del mapa activo inválido');
      if (parsed.issues.length > 0) throw new Error(`Mapa activo inválido: ${parsed.issues[0].message}`);
      return { document: parsed.document, activeVersion: parsed.envelope.version };
    } catch (error) {
      /* 404 = todavía no hay publicación: el editor parte del fixture. */
      if (error instanceof ApiError && error.status === 404) return null;
      throw error;
    }
  },

  /** Publica una nueva versión inmutable del mapa. `expectedVersion` debe
   * coincidir con la activa (0 para la primera publicación); 409 = conflicto. */
  async publish(
    document: MapVersion,
    expectedVersion: number,
    options?: { signal?: AbortSignal },
  ): Promise<GameMapVersionPublic> {
    const issues = validateMapVersion(document);
    if (issues.length > 0) {
      throw new Error(`MapVersion inválido: ${issues.map((issue) => `${issue.path} ${issue.message}`).join('; ')}`);
    }
    const response = await generatedFetcher<GeneratedResponse<unknown>>(
      '/api/admin/game/maps',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          expectedVersion,
          mapId: document.id,
          document,
        }),
        signal: options?.signal,
      },
    );
    const payload = unwrapGeneratedResponse<unknown>(response, [200]);
    if (!isValidGameMapVersionPublic(payload)) {
      throw new Error('Respuesta de publicación de mapa inválida');
    }
    return payload;
  },
};
