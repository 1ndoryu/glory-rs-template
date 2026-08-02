import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { authStore } from '../store';
import { ApiError } from '../api/client';
import { GameProfileService, isValidGameProfile } from './game-profile.service';

describe('GameProfileService', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    authStore.set({ isAuthenticated: true, userId: 'user-1', capability: 'authenticated' }, 'sync');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('loads a bounded camelCase profile and sends credentials', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      displayName: 'Guardián',
      revision: 2,
      updatedAt: '2026-08-02T00:00:00Z',
    }), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(GameProfileService.get()).resolves.toEqual({
      displayName: 'Guardián',
      revision: 2,
      updatedAt: '2026-08-02T00:00:00Z',
    });

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/game/profile');
    expect(init.credentials).toBe('include');
    expect(init.signal).toBeUndefined();
  });

  it('preserves abort signals and rejects malformed successful responses', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      displayName: 'x'.repeat(25),
      revision: 0,
      updatedAt: 'now',
    }), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const controller = new AbortController();

    await expect(GameProfileService.get({ signal: controller.signal }))
      .rejects.toThrow('Respuesta de perfil de juego inválida');
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(init.signal).toBe(controller.signal);
  });

  it('keeps the guest boundary as an unauthorized API error', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ error: 'unauthorized' }), { status: 401 }),
    ));

    try {
      await GameProfileService.get();
      throw new Error('se esperaba 401');
    } catch (error: unknown) {
      expect(error).toEqual(expect.any(ApiError));
      expect(error).toMatchObject({ status: 401 });
    }
    expect(authStore.get().capability).toBe('public');
  });

  it('validates profile shape before the game consumes it', () => {
    expect(isValidGameProfile({ displayName: 'Jugador', revision: 0, updatedAt: 'now' })).toBe(true);
    expect(isValidGameProfile({ displayName: 'Ju\u200Bgador', revision: 0, updatedAt: 'now' })).toBe(false);
    expect(isValidGameProfile({ displayName: ' Jugador', revision: 0, updatedAt: 'now' })).toBe(false);
    expect(isValidGameProfile({ displayName: ' ', revision: 0, updatedAt: 'now' })).toBe(false);
    expect(isValidGameProfile({ displayName: 'Jugador', revision: -1, updatedAt: 'now' })).toBe(false);
    expect(isValidGameProfile({ displayName: 'Jugador', revision: 0, updatedAt: 7 })).toBe(false);
  });
});
