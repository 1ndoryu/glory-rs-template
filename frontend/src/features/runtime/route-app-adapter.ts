/* wandori.us — Route App Adapter
 * Conecta el router existente con el AppRegistry y WindowManager.
 * Cuando el router navega a una ruta que una app maneja, el adapter
 * abre una ventana en desktop o renderiza fullscreen en mobile. */

import { onNavigate } from '../../router';
import { AppRegistry } from './app-registry';
import {
  openWindow,
  focusWindow,
  findOpenWindow,
  restoreWindow,
} from './window-manager';
import { dispatchEvent } from '../analytics/dispatcher';

/** Registrar los comandos del sistema en el CommandRegistry. */
export function initRouteAppAdapter(): void {
  onNavigate((path) => {
    const app = AppRegistry.findByRoute(path);
    if (!app) return; /* Páginas normales (home, article, etc.) */

    /* Si es singleton y ya está abierto, solo enfocar */
    if (app.singleton) {
      const existing = findOpenWindow(app.id);
      if (existing) {
        if (existing.state === 'minimized') {
          restoreWindow(existing.instanceId);
        }
        focusWindow(existing.instanceId);
        dispatchEvent({ type: 'window_focused', appId: app.id });
        return;
      }
    }

    /* Abrir nueva ventana */
    openAppWindow(app.id);
  });
}

/** Abrir ventana para una app por ID. */
export async function openAppWindow(appId: string): Promise<void> {
  const app = AppRegistry.get(appId);
  if (!app) return;

  /* Singleton ya abierto → enfocar */
  if (app.singleton) {
    const existing = findOpenWindow(appId);
    if (existing) {
      if (existing.state === 'minimized') restoreWindow(existing.instanceId);
      focusWindow(existing.instanceId);
      return;
    }
  }

  /* Crear AbortController y RenderContext */
  const controller = new AbortController();
  const ctx = { signal: controller.signal };

  /* Instanciar contenido de la app */
  const view = await AppRegistry.instantiate(appId, ctx);
  if (!view) return;

  /* Abrir ventana con el contenido */
  openWindow(app, view, controller);
  dispatchEvent({ type: 'app_opened', appId });
}
