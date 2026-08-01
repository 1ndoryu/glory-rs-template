/* [GAME-01-VIS] Registro aislado del boceto Bosque.
 * Vive fuera del catálogo principal para mantener su límite y para que la
 * futura app del juego pueda evolucionar sin ampliar un registro monolítico. */

import { TreePine } from 'lucide';
import type { MountedView } from '../../core/lifecycle';
import { dispatchEvent } from '../analytics/dispatcher';
import { AppRegistry } from './app-registry';
import { createPathDeepLink } from './deep-links';

AppRegistry.registerLazy({
  id: 'game',
  title: 'Bosque',
  icon: TreePine,
  iconType: 'application',
  singleton: true,
  requires: 'public',
  deepLink: createPathDeepLink('/forest-2d'),
  layout: 'full-bleed',
  load: () => import('../desktop/apps/game-preview/game-preview').then((module) => ({
    render: (): MountedView => {
      dispatchEvent({ type: 'app_opened', appId: 'game' });
      const view = module.renderGamePreview();
      let destroyed = false;
      return {
        element: view.element,
        destroy: () => {
          if (destroyed) return;
          destroyed = true;
          view.destroy?.();
          dispatchEvent({ type: 'app_closed', appId: 'game' });
        },
      };
    },
  })),
});
