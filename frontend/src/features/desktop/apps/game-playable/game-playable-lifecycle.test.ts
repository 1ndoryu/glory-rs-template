import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '../../../../api/client';
import { authStore } from '../../../../store';

const mocks = vi.hoisted(() => ({
  detectWebGL: vi.fn(),
  mountGamePlayableScene: vi.fn(),
  createGameInput: vi.fn(),
  cancelAnimationFrame: vi.fn(),
  getGameProfile: vi.fn(),
  listGameCharacters: vi.fn(),
}));

vi.mock('./game-webgl-capabilities', () => ({ detectWebGL: mocks.detectWebGL }));
vi.mock('./game-playable-scene', () => ({ mountGamePlayableScene: mocks.mountGamePlayableScene }));
vi.mock('./game-playable-input', () => ({ createGameInput: mocks.createGameInput }));
vi.mock('../../../../services', () => ({
  GameCharacterService: { list: mocks.listGameCharacters },
  GameProfileService: { get: mocks.getGameProfile },
}));

import { renderGamePlayable } from './game-playable';

async function flushHydration(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe('Bosque playable WebGL lifecycle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    authStore.set({ isAuthenticated: false, userId: null, capability: 'public' }, 'init');
    mocks.getGameProfile.mockResolvedValue({ displayName: 'Guardián', characterId: 'forest-scout', revision: 0, updatedAt: '2026-08-02T00:00:00Z' });
    mocks.listGameCharacters.mockResolvedValue([{ id: 'forest-scout', displayName: 'Explorador', bodyTone: 'ink' }]);
    mocks.detectWebGL.mockReturnValue({ available: false, reason: 'WebGL bloqueado en el dispositivo' });
    mocks.createGameInput.mockImplementation(() => ({
      controls: document.createElement('div'),
      getDirection: () => ({ x: 0, z: 0 }),
      destroy: vi.fn(),
    }));
    mocks.mountGamePlayableScene.mockImplementation(() => ({
      canvas: document.createElement('canvas'),
      update: vi.fn(),
      resize: vi.fn(),
      render: vi.fn(),
      streamingStats: () => ({ cacheSize: 0, visibleChunks: 0, visibleInstances: 0, visibleAssets: 0 }),
      rendererMetrics: () => ({
        rendererInfoAvailable: true,
        rendererMemoryAvailable: true,
        drawCalls: 0,
        triangles: 0,
        lines: 0,
        points: 0,
        geometries: 0,
        textures: 0,
      }),
      destroy: vi.fn(),
    }));
    vi.stubGlobal('ResizeObserver', class {
      observe(): void {}
      disconnect(): void {}
    });
    vi.stubGlobal('requestAnimationFrame', vi.fn(() => 7));
    vi.stubGlobal('cancelAnimationFrame', mocks.cancelAnimationFrame);
  });

  it('loads the validated profile before deciding whether WebGL can mount', async () => {
    const view = renderGamePlayable({ signal: new AbortController().signal });

    expect(view.element.querySelector('canvas')).toBeNull();
    expect(view.element.querySelector('.juegoFixture__estado')?.textContent)
      .toContain('cargando perfil');
    expect(mocks.detectWebGL).not.toHaveBeenCalled();

    await flushHydration();

    expect(mocks.getGameProfile).toHaveBeenCalledOnce();
    expect(mocks.detectWebGL).toHaveBeenCalledOnce();
    expect(view.element.querySelector('.juegoFixture__estado')?.textContent)
      .toContain('WebGL bloqueado');
    expect(view.element.dataset.playerName).toBe('Guardián');
    expect(view.element.dataset.characterId).toBe('forest-scout');
    view.destroy?.();
  });

  it('keeps a revoked account out of guest realtime and preserves the warning state', async () => {
    authStore.set({ isAuthenticated: true, userId: 'account-1', capability: 'authenticated' }, 'sync');
    mocks.detectWebGL.mockReturnValue({ available: true, kind: 'webgl2' });
    mocks.getGameProfile.mockRejectedValue(new ApiError(401, { error: 'unauthorized' }, 'API Error: 401'));

    const view = renderGamePlayable({ signal: new AbortController().signal });
    await flushHydration();

    expect(view.element.dataset.playerName).toBe('Jugador');
    expect(view.element.querySelector('.juegoFixture__estado')?.textContent)
      .toContain('sesión expirada');
    expect((view.element.querySelector('.juegoFixture__estado') as HTMLElement).dataset.state).toBe('error');
    expect(mocks.mountGamePlayableScene).toHaveBeenCalledOnce();
    view.destroy?.();
  });

  it('cleans hydration handles when the catalog has no valid character', async () => {
    const clearTimeoutSpy = vi.spyOn(window, 'clearTimeout');
    const controller = new AbortController();
    const removeAbortSpy = vi.spyOn(controller.signal, 'removeEventListener');
    mocks.listGameCharacters.mockResolvedValue([]);

    const view = renderGamePlayable({ signal: controller.signal });
    await flushHydration();

    expect(mocks.getGameProfile).toHaveBeenCalledOnce();
    expect(mocks.detectWebGL).not.toHaveBeenCalled();
    expect(mocks.mountGamePlayableScene).not.toHaveBeenCalled();
    expect(view.element.querySelector('.juegoFixture__estado')?.textContent)
      .toContain('personaje no disponible');
    expect(clearTimeoutSpy).toHaveBeenCalled();
    expect(removeAbortSpy).toHaveBeenCalledOnce();
    view.destroy?.();
  });

  it('aborts the profile request and clears its timeout before resolution', async () => {
    let resolveProfile: ((profile: { displayName: string; characterId: string; revision: number; updatedAt: string }) => void) | undefined;
    const clearTimeoutSpy = vi.spyOn(window, 'clearTimeout');
    mocks.getGameProfile.mockReturnValue(new Promise(resolve => { resolveProfile = resolve; }));
    const controller = new AbortController();
    const view = renderGamePlayable({ signal: controller.signal });

    controller.abort();
    resolveProfile?.({ displayName: 'Tarde', characterId: 'forest-scout', revision: 0, updatedAt: '2026-08-02T00:00:00Z' });
    await flushHydration();

    expect(mocks.detectWebGL).not.toHaveBeenCalled();
    expect(mocks.createGameInput).not.toHaveBeenCalled();
    expect(mocks.mountGamePlayableScene).not.toHaveBeenCalled();
    expect(clearTimeoutSpy).toHaveBeenCalled();
    view.destroy?.();
  });

  it('mounts and destroys the playable view repeatedly without retaining handles', async () => {
    mocks.detectWebGL.mockReturnValue({ available: true, kind: 'webgl2' });

    const views = Array.from({ length: 12 }, () => renderGamePlayable({ signal: new AbortController().signal }));
    await flushHydration();
    views.forEach(view => view.destroy?.());

    expect(mocks.createGameInput).toHaveBeenCalledTimes(12);
    expect(mocks.mountGamePlayableScene).toHaveBeenCalledTimes(12);
    for (const result of mocks.createGameInput.mock.results) {
      expect(result.value.destroy).toHaveBeenCalledOnce();
    }
    for (const result of mocks.mountGamePlayableScene.mock.results) {
      expect(result.value.destroy).toHaveBeenCalledOnce();
    }
  });

  it('stops the frame loop and exposes an accessible error after context loss', async () => {
    mocks.detectWebGL.mockReturnValue({ available: true, kind: 'webgl2' });

    const controller = new AbortController();
    const view = renderGamePlayable({ signal: controller.signal });
    await flushHydration();
    const contextLost = new Event('webglcontextlost', { cancelable: true });
    mocks.mountGamePlayableScene.mock.results[0]?.value.canvas.dispatchEvent(contextLost);

    expect(contextLost.defaultPrevented).toBe(true);
    expect(mocks.cancelAnimationFrame).toHaveBeenCalledWith(7);
    expect(view.element.querySelector('.juegoFixture__estado')?.textContent)
      .toContain('perdió el contexto 3D');
    expect((view.element.querySelector('.juegoFixture__estado') as HTMLElement | null)?.dataset.state).toBe('error');

    Object.defineProperty(document, 'hidden', { configurable: true, value: true });
    document.dispatchEvent(new Event('visibilitychange'));
    Object.defineProperty(document, 'hidden', { configurable: true, value: false });
    document.dispatchEvent(new Event('visibilitychange'));
    expect(globalThis.requestAnimationFrame).toHaveBeenCalledTimes(1);

    view.destroy?.();
    Object.defineProperty(document, 'hidden', { configurable: true, value: false });
    expect(mocks.createGameInput.mock.results[0]?.value.destroy).toHaveBeenCalledOnce();
    expect(mocks.mountGamePlayableScene.mock.results[0]?.value.destroy).toHaveBeenCalledOnce();
  });
});
