/* GAME-01 — Vertical slice jugable offline.
 * Orquesta input, simulación pura y renderer. No abre red, no persiste estado y
 * no crea chrome del OS. El fixture se reemplazará por contratos publicados en
 * fases posteriores, sin cambiar el lifecycle de la app. */

import type { MountedView, RenderContext } from '../../../../core/lifecycle';
import { createEl } from '../../../../utils/dom';
import {
  createWorldState,
  simulateTick,
  snapshotFromState,
  FramePerformanceMonitor,
  type WorldState,
} from '../../../game-core';
import { evaluateGamePerformanceBudget } from './game-performance-budget';
import { FIXTURE_MAP, FIXTURE_MAP_VERSION } from './game-fixture-map';
import { createGameInput, type GameInputHandle } from './game-playable-input';
import { mountGamePlayableScene, type GamePlayableSceneHandle } from './game-playable-scene';
import '../../../../styles/desktop/desktop-game-playable.css';

interface GamePlayableElements {
  readonly element: HTMLElement;
  readonly sceneHost: HTMLElement;
  readonly status: HTMLElement;
}

export function createGamePlayableView(): GamePlayableElements {
  const sceneHost = createEl('div', {
    className: 'juegoFixture__escena',
    ariaLabel: 'Escena jugable offline del Bosque',
  });
  const status = createEl('p', {
    className: 'juegoFixture__estado',
    textContent: 'cargando fixture offline…',
  });
  status.setAttribute('aria-live', 'polite');
  const guide = createEl('header', { className: 'juegoFixture__guia' },
    createEl('div', {},
      createEl('h2', { className: 'juegoFixture__titulo', textContent: 'Bosque · prueba jugable' }),
      createEl('p', { className: 'juegoFixture__ayuda', textContent: 'mueve con WASD o las flechas · toca el pad en móvil' }),
    ),
  );
  return {
    element: createEl('section', { className: 'juegoFixture', ariaLabel: 'Bosque, fixture jugable offline' }, sceneHost, status, guide),
    sceneHost,
    status,
  };
}

export function renderGamePlayable(context: RenderContext): MountedView {
  const view = createGamePlayableView();
  /* La carga lazy puede resolver después de que WindowManager haya abortado la
   * vista. No montar listeners, observers ni WebGL si el scope ya terminó. */
  if (context.signal.aborted) {
    return { element: view.element, destroy: () => {} };
  }
  const input: GameInputHandle = createGameInput();
  view.element.appendChild(input.controls);

  let scene: GamePlayableSceneHandle | null = null;
  let state: WorldState = createWorldState([{ id: 'local', position: { x: 0, z: -0.5 }, radius: 0.38 }]);
  let sequence = 0;
  let frameHandle = 0;
  let lastTime = performance.now();
  let visible = !document.hidden;
  let destroyed = false;
  const frameMonitor = new FramePerformanceMonitor({ maxSamples: 120 });
  let frameCount = 0;

  const stopFrameLoop = (): void => {
    if (frameHandle !== 0) cancelAnimationFrame(frameHandle);
    frameHandle = 0;
  };

  const setStatus = (message: string, error = false): void => {
    view.status.textContent = message;
    view.status.dataset.state = error ? 'error' : 'ready';
    view.status.hidden = !error;
  };

  const renderFrame = (now: number): void => {
    frameHandle = 0;
    if (destroyed || !visible || !scene) return;

    const frameStart = performance.now();
    const delta = Math.min(Math.max((now - lastTime) / 1000, 0), 0.1);
    lastTime = now;
    const direction = input.getDirection();
    try {
      state = simulateTick(
        state,
        FIXTURE_MAP,
        [{ playerId: 'local', direction, sequence: sequence++ }],
        delta,
      );
      scene.update(snapshotFromState(state));
      scene.render();
      frameMonitor.record(performance.now() - frameStart);
      frameCount += 1;
      const streaming = scene.streamingStats();
      const rendererMetrics = scene.rendererMetrics();
      const performanceSnapshot = frameMonitor.snapshot();
      const performanceBudget = evaluateGamePerformanceBudget(performanceSnapshot, rendererMetrics);
      view.element.dataset.visibleChunks = String(streaming.visibleChunks);
      view.element.dataset.visibleInstances = String(streaming.visibleInstances);
      view.element.dataset.frameP95Ms = performanceSnapshot.p95Ms.toFixed(2);
      view.element.dataset.rendererDrawCalls = String(rendererMetrics.drawCalls);
      view.element.dataset.rendererTriangles = String(rendererMetrics.triangles);
      view.element.dataset.rendererGeometries = String(rendererMetrics.geometries);
      view.element.dataset.rendererTextures = String(rendererMetrics.textures);
      view.element.dataset.rendererBudgetStatus = performanceBudget.status;
      view.element.dataset.rendererBudgetFrameStatus = performanceBudget.frame.status;
      view.element.dataset.rendererBudgetHeapStatus = performanceBudget.jsHeapUsedBytes.status;
      if (rendererMetrics.jsHeapUsedBytes !== undefined) {
        view.element.dataset.jsHeapUsedBytes = String(rendererMetrics.jsHeapUsedBytes);
      }
      if (rendererMetrics.jsHeapLimitBytes !== undefined) {
        view.element.dataset.jsHeapLimitBytes = String(rendererMetrics.jsHeapLimitBytes);
      }
      if (frameCount % 30 === 0) {
        setStatus(
          `offline · chunks ${streaming.visibleChunks} · props ${streaming.visibleInstances} · p95 ${performanceSnapshot.p95Ms.toFixed(1)}ms`,
          false,
        );
      }
    } catch (error: unknown) {
      stopFrameLoop();
      setStatus('no se pudo ejecutar el fixture offline', true);
      console.error('[Bosque fixture] Tick rechazado.', error);
      return;
    }
    frameHandle = requestAnimationFrame(renderFrame);
  };

  const startFrameLoop = (): void => {
    if (destroyed || !visible || frameHandle !== 0) return;
    lastTime = performance.now();
    frameHandle = requestAnimationFrame(renderFrame);
  };

  const onVisibilityChange = (): void => {
    visible = !document.hidden;
    if (visible) startFrameLoop();
    else stopFrameLoop();
  };
  const onResize = (): void => scene?.resize();

  const destroy = (): void => {
    if (destroyed) return;
    destroyed = true;
    stopFrameLoop();
    context.signal.removeEventListener('abort', destroy);
    document.removeEventListener('visibilitychange', onVisibilityChange);
    resizeObserver.disconnect();
    input.destroy();
    scene?.destroy();
    scene = null;
  };

  context.signal.addEventListener('abort', destroy, { once: true });
  document.addEventListener('visibilitychange', onVisibilityChange);
  const resizeObserver = new ResizeObserver(onResize);

  try {
    scene = mountGamePlayableScene(view.sceneHost, FIXTURE_MAP, FIXTURE_MAP_VERSION);
    resizeObserver.observe(view.sceneHost);
    scene.update(snapshotFromState(state));
    setStatus('offline · movimiento local · sin red', false);
    startFrameLoop();
  } catch (error: unknown) {
    setStatus('este dispositivo no pudo iniciar el fixture 3d', true);
    console.error('[Bosque fixture] No se pudo iniciar la escena.', error);
  }

  return { element: view.element, destroy };
}
