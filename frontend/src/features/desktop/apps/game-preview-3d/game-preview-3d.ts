/* [GAME-01-VIS-3D] Vista del segundo boceto visual.
 * El contenido conoce únicamente su escenario; ventanas, taskbar y móvil
 * siguen perteneciendo al shell. Three.js se importa después de abrirla. */

import type { MountedView, RenderContext } from '../../../../core/lifecycle';
import { createEl } from '../../../../utils/dom';
import '../../../../styles/desktop/desktop-game-preview-3d.css';
import type { ForestSceneHandle } from './forest-scene';

interface Game3dPreviewElements {
  readonly element: HTMLElement;
  readonly sceneHost: HTMLElement;
  readonly status: HTMLElement;
  readonly resetButton: HTMLButtonElement;
}

export function createGame3dPreview(): Game3dPreviewElements {
  const sceneHost = createEl('div', {
    className: 'bosque3d__escena',
    ariaLabel: 'Escena interactiva del Bosque 3D',
  });
  const status = createEl('p', {
    className: 'bosque3d__estado',
    textContent: 'preparando diorama 3d…',
  });
  status.setAttribute('aria-live', 'polite');
  const resetButton = createEl('button', {
    className: 'boton bosque3d__recentrar',
    textContent: 'recentrar',
    type: 'button',
  });
  const guide = createEl(
    'header',
    { className: 'bosque3d__guia' },
    createEl('div', {},
      createEl('h2', { className: 'bosque3d__titulo', textContent: 'Bosque 3D' }),
      createEl('p', { className: 'bosque3d__ayuda', textContent: 'arrastra para orbitar · rueda o pellizca para acercar' }),
    ),
    resetButton,
  );
  return {
    element: createEl('section', { className: 'bosque3d', ariaLabel: 'Segundo boceto visual Bosque 3D' }, sceneHost, status, guide),
    sceneHost,
    status,
    resetButton,
  };
}

export function renderGame3dPreview(context: RenderContext): MountedView {
  const preview = createGame3dPreview();
  let scene: ForestSceneHandle | null = null;
  let destroyed = false;

  const reset = (): void => scene?.resetCamera();
  const destroy = (): void => {
    if (destroyed) return;
    destroyed = true;
    context.signal.removeEventListener('abort', destroy);
    preview.resetButton.removeEventListener('click', reset);
    scene?.destroy();
    scene = null;
  };
  preview.resetButton.addEventListener('click', reset);
  context.signal.addEventListener('abort', destroy, { once: true });

  void import('./forest-scene')
    .then(({ mountForestScene }) => {
      if (destroyed || context.signal.aborted) return;
      scene = mountForestScene(preview.sceneHost, {
        onContextLost: () => {
          if (destroyed) return;
          preview.status.textContent = 'la vista 3d perdió el contexto gráfico; cierra y vuelve a abrir el boceto';
          preview.status.dataset.state = 'error';
          preview.status.hidden = false;
        },
      });
      /* Se conserva oculto para poder mostrar un fallo tardío de contexto
       * WebGL sin crear otro overlay ni dejar un nodo huérfano. */
      preview.status.hidden = true;
    })
    .catch((error: unknown) => {
      if (destroyed) return;
      console.error('[Bosque 3D] No se pudo iniciar WebGL.', error);
      preview.status.textContent = 'este dispositivo no pudo iniciar la vista 3d';
      preview.status.dataset.state = 'error';
    });

  return { element: preview.element, destroy };
}
