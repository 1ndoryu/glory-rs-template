import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  detectWebGL: vi.fn(),
  mountGamePlayableScene: vi.fn(),
  createGameInput: vi.fn(),
  cancelAnimationFrame: vi.fn(),
}));

vi.mock('./game-webgl-capabilities', () => ({ detectWebGL: mocks.detectWebGL }));
vi.mock('./game-playable-scene', () => ({ mountGamePlayableScene: mocks.mountGamePlayableScene }));
vi.mock('./game-playable-input', () => ({ createGameInput: mocks.createGameInput }));

import { renderGamePlayable } from './game-playable';

describe('Bosque playable WebGL lifecycle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.detectWebGL.mockReturnValue({ available: false, reason: 'WebGL bloqueado en el dispositivo' });
    mocks.createGameInput.mockReturnValue({
      controls: document.createElement('div'),
      getDirection: () => ({ x: 0, z: 0 }),
      destroy: vi.fn(),
    });
    mocks.mountGamePlayableScene.mockReturnValue({
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
    });
    vi.stubGlobal('ResizeObserver', class {
      observe(): void {}
      disconnect(): void {}
    });
    vi.stubGlobal('requestAnimationFrame', vi.fn(() => 7));
    vi.stubGlobal('cancelAnimationFrame', mocks.cancelAnimationFrame);
  });

  it('shows fallback before creating input or a Three scene', () => {
    const view = renderGamePlayable({ signal: new AbortController().signal });

    expect(view.element.dataset.state).toBeUndefined();
    expect(view.element.querySelector('canvas')).toBeNull();
    expect(view.element.querySelector('.juegoFixture__estado')?.textContent)
      .toContain('WebGL bloqueado');
    expect(view.element.querySelector('.juegoFixture__estado')?.getAttribute('aria-live')).toBe('polite');
    expect(mocks.createGameInput).not.toHaveBeenCalled();
    expect(mocks.mountGamePlayableScene).not.toHaveBeenCalled();
  });

  it('stops the frame loop and exposes an accessible error after context loss', () => {
    mocks.detectWebGL.mockReturnValue({ available: true, kind: 'webgl2' });

    const controller = new AbortController();
    const view = renderGamePlayable({ signal: controller.signal });
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
    expect(mocks.createGameInput.mock.results[0]?.value.destroy).toHaveBeenCalledOnce();
    expect(mocks.mountGamePlayableScene.mock.results[0]?.value.destroy).toHaveBeenCalledOnce();
  });
});
