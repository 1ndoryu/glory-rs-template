/* [GAME-01-VIS] Boceto visual estático del Bosque.
 * Esta vista existe para aprobar dirección artística dentro del OS antes de
 * introducir Canvas, input, estado de juego, red o backend. El SVG es un
 * asset original y la app solo devuelve contenido; el shell conserva chrome,
 * ventana, taskbar y presentación móvil. */

import bosqueBocetoUrl from '../../../../assets/game-preview/bosque-boceto.svg';
import '../../../../styles/desktop/desktop-game-preview.css';
import type { MountedView } from '../../../../core/lifecycle';
import { createEl } from '../../../../utils/dom';

export function createGamePreview(): HTMLElement {
  const image = createEl('img', {
    className: 'bosqueBoceto__imagen',
    src: bosqueBocetoUrl,
    alt: 'Bosque cenital de tinta con árboles, lagos, sendero y dos figuras de escala',
  });

  const scene = createEl(
    'figure',
    { className: 'bosqueBoceto__lienzo' },
    image,
    createEl('figcaption', {
      className: 'bosqueBoceto__etiqueta',
      textContent: 'boceto visual · sin lógica',
    }),
  );

  const localMarker = createEl('span', { className: 'bosqueBoceto__marca bosqueBoceto__marca--local' });
  const remoteMarker = createEl('span', { className: 'bosqueBoceto__marca bosqueBoceto__marca--remota' });
  const legend = createEl(
    'div',
    { className: 'bosqueBoceto__leyenda', ariaLabel: 'Leyenda del boceto' },
    createEl('span', {}, localMarker, 'tú'),
    createEl('span', {}, remoteMarker, 'otra persona'),
  );

  return createEl(
    'section',
    { className: 'bosqueBoceto', ariaLabel: 'Boceto visual del Bosque' },
    scene,
    legend,
  );
}

export function renderGamePreview(): MountedView {
  return {
    element: createGamePreview(),
    /* No hay listeners, timers ni recursos dinámicos en este prototipo. */
    destroy: () => {},
  };
}
