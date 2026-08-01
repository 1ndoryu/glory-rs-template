/* [GAME-01-VIS-3D] Puente temporal entre las rutas de los dos bocetos y el
 * RouteAppAdapter. El router exige una ruta conocida antes de delegarla; el
 * fallback solo aparece si el shell todavía no está montado. */

import { createEl } from '../../utils/dom';
import { addRoute } from '../../router';

function createPreviewFallback(): HTMLElement {
  return createEl('p', { textContent: 'abriendo boceto del bosque…' });
}

addRoute({ path: '/forest-2d', render: createPreviewFallback });
addRoute({ path: '/forest-3d', render: createPreviewFallback });
