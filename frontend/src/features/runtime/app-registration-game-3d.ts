/* [GAME-01-VIS-3D] Segundo boceto aislado: no reemplaza `game` y Three.js
 * solo entra en memoria al abrir esta aplicación. */

import { Box } from 'lucide';
import type { MountedView, RenderContext } from '../../core/lifecycle';
import { dispatchEvent } from '../analytics/dispatcher';
import { AppRegistry } from './app-registry';
import { createPathDeepLink } from './deep-links';

AppRegistry.registerLazy({
  id: 'game-3d',
  title: 'Bosque 3D',
  icon: Box,
  iconType: 'application',
  singleton: true,
  requires: 'public',
  deepLink: createPathDeepLink('/forest-3d'),
  layout: 'full-bleed',
  load: () => import('../desktop/apps/game-preview-3d/game-preview-3d').then((module) => ({
    render: (context: RenderContext): MountedView => {
      dispatchEvent({ type: 'app_opened', appId: 'game-3d' });
      const view = module.renderGame3dPreview(context);
      let destroyed = false;
      return {
        element: view.element,
        destroy: () => {
          if (destroyed) return;
          destroyed = true;
          view.destroy?.();
          dispatchEvent({ type: 'app_closed', appId: 'game-3d' });
        },
      };
    },
  })),
});
