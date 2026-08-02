import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GameAuditService, isValidAuditEvent } from './game-audit.service';

function auditEvent(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: 7,
    actorKind: 'admin',
    action: 'character.created',
    entityKind: 'character',
    entityId: 'forest-ranger',
    payload: { displayName: 'Guardabosques', bodyTone: 'middle', isActive: true },
    createdAt: '2026-08-02T00:00:00Z',
    ...overrides,
  };
}

describe('GameAuditService', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('loads the exact admin audit contract with query params', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify([auditEvent(), auditEvent({ action: 'character.updated', id: 8 })]), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);

    await expect(GameAuditService.listCharacterEvents({ entityId: 'forest-ranger', limit: 10 })).resolves.toEqual([
      expect.objectContaining({ id: 7, action: 'character.created' }),
      expect.objectContaining({ id: 8, action: 'character.updated' }),
    ]);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('/api/admin/game/audit/characters?entityId=forest-ranger&limit=10');
    expect(init.method).toBe('GET');
  });

  it('builds the URL without query when no filters are given', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify([]), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    await GameAuditService.listCharacterEvents();
    const [url] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('/api/admin/game/audit/characters');
  });

  it('rejects unknown actions, extra fields and malformed payloads', () => {
    expect(isValidAuditEvent(auditEvent())).toBe(true);
    expect(isValidAuditEvent(auditEvent({ action: 'character.deleted' }))).toBe(false);
    expect(isValidAuditEvent(auditEvent({ actorKind: 'root' }))).toBe(false);
    expect(isValidAuditEvent(auditEvent({ script: 'alert(1)' }))).toBe(false);
    expect(isValidAuditEvent(auditEvent({ payload: 'x' }))).toBe(false);
    expect(isValidAuditEvent(auditEvent({ id: '7' }))).toBe(false);
  });

  it('preserves the lifecycle abort signal', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify([]), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const controller = new AbortController();

    await GameAuditService.listCharacterEvents({ signal: controller.signal });
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(init.signal).toBe(controller.signal);
  });
});
