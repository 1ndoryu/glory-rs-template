/* GAME-01 — Vertical slice jugable con fallback offline.
 * Orquesta input, simulación, transporte realtime y renderer. El core sigue
 * puro; el socket se crea para cuentas e invitados temporales y siempre se
 * libera junto con la vista. El backend distingue ambas identidades. */

import type { MountedView, RenderContext } from '../../../../core/lifecycle';
import { authStore } from '../../../../store';
import { ApiError } from '../../../../api/client';
import { GameProfileService } from '../../../../services';
import { createEl } from '../../../../utils/dom';
import {
  createWorldState,
  simulateTick,
  snapshotFromState,
  FramePerformanceMonitor,
  type WorldState,
} from '../../../game-core';
import { evaluateGamePerformanceBudget } from './game-performance-budget';
import { detectWebGL } from './game-webgl-capabilities';
import { FIXTURE_MAP, FIXTURE_MAP_VERSION } from './game-fixture-map';
import { createGameInput, type GameInputHandle } from './game-playable-input';
import { mountGamePlayableScene, type GamePlayableSceneHandle } from './game-playable-scene';
import {
  createGameRealtimeClient,
  defaultGameSocketUrl,
  requestGameTicket,
  type GameRealtimeConnectionState,
} from './game-realtime-client';
import '../../../../styles/desktop/desktop-game-playable.css';

function normalizeRealtimeDirection(direction: { x: number; z: number }): { x: number; z: number } {
  if (!Number.isFinite(direction.x) || !Number.isFinite(direction.z)) return { x: 0, z: 0 };
  const length = Math.hypot(direction.x, direction.z);
  if (length <= 1) return direction;
  return { x: direction.x / length, z: direction.z / length };
}

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

/* [297A-49] El perfil se resuelve antes de montar WebGL/realtime. La vista
 * conserva un shell síncrono para que el runtime pueda cerrarla mientras la
 * petición está pendiente, y el AbortSignal evita recursos huérfanos. */
export function renderGamePlayable(context: RenderContext): MountedView {
  const view = createGamePlayableView();
  let disposed = false;
  let runtime: MountedView | null = null;
  let profileController: AbortController | null = null;
  let profileTimeout: number | null = null;
  const accountSessionAtStart = authStore.get().isAuthenticated;

  const setLoadingStatus = (message: string): void => {
    view.status.textContent = message;
    view.status.dataset.state = 'loading';
    view.status.hidden = false;
  };

  const hydrate = async (): Promise<void> => {
    if (context.signal.aborted || disposed) return;
    setLoadingStatus('cargando perfil de juego…');
    profileController = new AbortController();
    const abortProfile = (): void => profileController?.abort();
    profileTimeout = window.setTimeout(() => profileController?.abort(), 5_000);
    context.signal.addEventListener('abort', abortProfile, { once: true });

    let displayName = 'Jugador';
    let profileLoadWarning = false;
    let profileSessionExpired = false;
    try {
      const profile = await GameProfileService.get({ signal: profileController.signal });
      displayName = profile.displayName;
    } catch (error: unknown) {
      if (context.signal.aborted || disposed) return;
      /* 401 es el camino normal del invitado: no hay fila persistente y el
       * realtime obtiene identidad temporal por separado. Otros fallos no
       * bloquean el fallback offline, pero sí dejan diagnóstico accesible. */
      if (error instanceof ApiError && error.status === 401 && accountSessionAtStart) {
        profileLoadWarning = true;
        profileSessionExpired = true;
        setLoadingStatus('sesión expirada · modo local');
      } else if (!(error instanceof ApiError && error.status === 401)) {
        profileLoadWarning = true;
        setLoadingStatus('perfil no disponible · modo local');
      }
    } finally {
      if (profileTimeout !== null) window.clearTimeout(profileTimeout);
      profileTimeout = null;
      context.signal.removeEventListener('abort', abortProfile);
      profileController = null;
    }

    if (context.signal.aborted || disposed) return;
    try {
      runtime = mountGamePlayableRuntime(
        context,
        view,
        displayName,
        profileLoadWarning,
        profileSessionExpired,
      );
    } catch (error: unknown) {
      if (context.signal.aborted || disposed) return;
      setLoadingStatus('este dispositivo no pudo iniciar Bosque');
      console.error('[Bosque fixture] No se pudo montar el runtime.', error);
    }
  };

  void hydrate();

  return {
    element: view.element,
    destroy: () => {
      if (disposed) return;
      disposed = true;
      if (profileTimeout !== null) window.clearTimeout(profileTimeout);
      profileTimeout = null;
      profileController?.abort();
      runtime?.destroy?.();
      runtime = null;
    },
  };
}

function mountGamePlayableRuntime(
  context: RenderContext,
  view: GamePlayableElements,
  displayName: string,
  profileLoadWarning: boolean,
  profileSessionExpired: boolean,
): MountedView {
  const setStatus = (message: string, error = false): void => {
    view.status.textContent = message;
    view.status.dataset.state = error ? 'error' : 'ready';
    view.status.hidden = !error;
  };
  /* La carga lazy puede resolver después de que WindowManager haya abortado la
   * vista. No montar listeners, observers ni WebGL si el scope ya terminó. */
  if (context.signal.aborted) {
    return { element: view.element, destroy: () => {} };
  }
  view.element.dataset.playerName = displayName;
  const capabilities = detectWebGL();
  if (!capabilities.available) {
    setStatus(`3D no disponible: ${capabilities.reason ?? 'WebGL rechazado'}`, true);
    return { element: view.element, destroy: () => {} };
  }
  view.element.dataset.webglKind = capabilities.kind ?? 'unknown';
  const input: GameInputHandle = createGameInput();
  view.element.appendChild(input.controls);

  let scene: GamePlayableSceneHandle | null = null;
  let state: WorldState = createWorldState([{ id: 'local', position: { x: 0, z: -0.5 }, radius: 0.38 }]);
  let sequence = 0;
  let frameHandle = 0;
  let lastTime = performance.now();
  let visible = !document.hidden;
  let contextLost = false;
  let destroyed = false;
  let lastNetworkMoveAt = 0;
  const frameMonitor = new FramePerformanceMonitor({ maxSamples: 120 });
  let frameCount = 0;
  let realtimeState: GameRealtimeConnectionState = 'idle';
  /* La identidad de juego puede ser cuenta o invitado temporal. `authStore`
   * sigue gobernando permisos del OS; no debe bloquear el loop realtime público. */
  const realtime = createGameRealtimeClient({
    ticketProvider: requestGameTicket,
    socketFactory: (url) => new WebSocket(url),
    socketUrl: defaultGameSocketUrl(),
    onState: (next, message) => {
      realtimeState = next;
      if (next === 'error') setStatus(message ?? 'realtime no disponible', true);
    },
  });

  const stopFrameLoop = (): void => {
    if (frameHandle !== 0) cancelAnimationFrame(frameHandle);
    frameHandle = 0;
  };

  const renderFrame = (now: number): void => {
    frameHandle = 0;
    if (destroyed || contextLost || !visible || !scene) return;

    const frameStart = performance.now();
    const delta = Math.min(Math.max((now - lastTime) / 1000, 0), 0.1);
    lastTime = now;
    const direction = input.getDirection();
    try {
      const networkSnapshot = realtime?.getState() === 'connected'
        ? realtime.getRenderSnapshot(now)
        : null;
      if (networkSnapshot) {
        if (now - lastNetworkMoveAt >= 66) {
          realtime?.sendMove(normalizeRealtimeDirection(direction));
          lastNetworkMoveAt = now;
        }
        scene.update(networkSnapshot, realtime?.getPlayerId() ?? undefined);
      } else {
        state = simulateTick(
          state,
          FIXTURE_MAP,
          [{ playerId: 'local', direction, sequence: sequence++ }],
          delta,
        );
        scene.update(snapshotFromState(state));
      }
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
      if (frameCount % 30 === 0 && realtimeState !== 'error' && !profileLoadWarning) {
        const mode = realtimeState === 'connected'
          ? `conectado${realtime?.getMapVersion() ? ` · ${realtime.getMapVersion()}` : ''}`
          : realtimeState === 'connecting'
            ? 'conectando… · fallback local'
            : 'offline · movimiento local';
        setStatus(
          `${displayName} · ${mode} · chunks ${streaming.visibleChunks} · props ${streaming.visibleInstances} · p95 ${performanceSnapshot.p95Ms.toFixed(1)}ms`,
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
    if (destroyed || contextLost || !visible || frameHandle !== 0) return;
    lastTime = performance.now();
    frameHandle = requestAnimationFrame(renderFrame);
  };

  const onVisibilityChange = (): void => {
    visible = !document.hidden;
    if (visible) startFrameLoop();
    else stopFrameLoop();
  };
  const onContextLost = (event: Event): void => {
    event.preventDefault();
    contextLost = true;
    stopFrameLoop();
    setStatus('el navegador perdió el contexto 3D; cierra y vuelve a abrir Bosque', true);
  };
  const onResize = (): void => scene?.resize();

  const destroy = (): void => {
    if (destroyed) return;
    destroyed = true;
    stopFrameLoop();
    context.signal.removeEventListener('abort', destroy);
    document.removeEventListener('visibilitychange', onVisibilityChange);
    scene?.canvas.removeEventListener('webglcontextlost', onContextLost);
    resizeObserver.disconnect();
    input.destroy();
    realtime?.destroy();
    scene?.destroy();
    scene = null;
  };

  context.signal.addEventListener('abort', destroy, { once: true });
  document.addEventListener('visibilitychange', onVisibilityChange);
  /* El evento pertenece al canvas real de Three; no dependemos de bubbling. */
  const attachContextLossListener = (): void => {
    scene?.canvas.addEventListener('webglcontextlost', onContextLost, { once: true });
  };
  const resizeObserver = new ResizeObserver(onResize);

  try {
    scene = mountGamePlayableScene(view.sceneHost, FIXTURE_MAP, FIXTURE_MAP_VERSION);
    attachContextLossListener();
    resizeObserver.observe(view.sceneHost);
    scene.update(snapshotFromState(state));
    setStatus(
      profileSessionExpired
        ? `${displayName} · sesión expirada · modo local`
        : profileLoadWarning
          ? `${displayName} · perfil no disponible · modo local`
          : authStore.get().isAuthenticated
            ? `${displayName} · conectando… · fallback local mientras se autentica`
            : `${displayName} · conectando… · fallback local mientras se identifica el invitado`,
      profileLoadWarning,
    );
    if (!profileSessionExpired) void realtime?.connect();
    startFrameLoop();
  } catch (error: unknown) {
    setStatus('este dispositivo no pudo iniciar el fixture 3d', true);
    console.error('[Bosque fixture] No se pudo iniciar la escena.', error);
  }

  return { element: view.element, destroy };
}
