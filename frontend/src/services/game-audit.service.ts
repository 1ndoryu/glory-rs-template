/* GAME-01 — Actividad auditada del catálogo del Bosque.
 * Transporta el listado admin de eventos sensibles (solo AdminUser en el
 * backend); el panel Admin lo muestra en el tab "juego". El contrato no
 * expone identidades: actor_kind, acción, entidad, payload visual y fecha. */

import { generatedFetcher, unwrapGeneratedResponse, type GeneratedResponse } from '../api/client';

export interface GameAuditEventEntry {
  id: number;
  actorKind: string;
  action: string;
  entityKind: string;
  entityId: string;
  payload: Record<string, unknown>;
  createdAt: string;
}

const ACTIONS_ALLOWLIST = new Set(['character.created', 'character.updated']);
const ACTOR_KINDS_ALLOWLIST = new Set(['admin', 'account', 'system']);
const ENTITY_KINDS_ALLOWLIST = new Set(['character']);

export function isValidAuditEvent(value: unknown): value is GameAuditEventEntry {
  if (typeof value !== 'object' || value === null) return false;
  const event = value as Record<string, unknown>;
  const keys = Object.keys(event).sort();
  if (keys.join(',') !== 'action,actorKind,createdAt,entityId,entityKind,id,payload') return false;
  return typeof event.id === 'number'
    && Number.isInteger(event.id)
    && typeof event.actorKind === 'string'
    && ACTOR_KINDS_ALLOWLIST.has(event.actorKind)
    && typeof event.action === 'string'
    && ACTIONS_ALLOWLIST.has(event.action)
    && typeof event.entityKind === 'string'
    && ENTITY_KINDS_ALLOWLIST.has(event.entityKind)
    && typeof event.entityId === 'string'
    && typeof event.payload === 'object'
    && event.payload !== null
    && !Array.isArray(event.payload)
    && typeof event.createdAt === 'string';
}

export interface ListAuditEventsOptions {
  entityId?: string;
  limit?: number;
  signal?: AbortSignal;
}

export const GameAuditService = {
  async listCharacterEvents(options?: ListAuditEventsOptions): Promise<GameAuditEventEntry[]> {
    const params = new URLSearchParams();
    if (options?.entityId) params.set('entityId', options.entityId);
    if (options?.limit !== undefined) params.set('limit', String(options.limit));
    const query = params.toString();
    const response = await generatedFetcher<GeneratedResponse<unknown>>(
      `/api/admin/game/audit/characters${query ? `?${query}` : ''}`,
      { method: 'GET', signal: options?.signal },
    );
    const payload = unwrapGeneratedResponse<unknown>(response, [200]);
    if (!Array.isArray(payload) || !payload.every(isValidAuditEvent)) {
      throw new Error('Actividad del catálogo inválida');
    }
    return payload;
  },
};
