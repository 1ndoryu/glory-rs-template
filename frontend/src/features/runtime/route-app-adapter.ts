/* wandori.us — Route App Adapter
 * Conecta el router existente con el AppRegistry y WindowManager.
 * Registra un interceptor de rutas: cuando el router navega a una ruta
 * que una app maneja, el adapter abre una ventana y evita que el router
 * renderice en el outlet. Esto elimina el doble rendering. */

import { setRouteInterceptor, type RouteParams } from '../../router';
import { AppRegistry } from './app-registry';
import {
  openWindow,
  focusWindow,
  findOpenWindow,
  restoreWindow,
  windowStore,
} from './window-manager';
import { dispatchEvent } from '../analytics/dispatcher';
import { authStore } from '../../store';

/** Registrar el interceptor de rutas en el router.
 * Llamar una vez desde main.ts. */
export function initRouteAppAdapter(): void {
  setRouteInterceptor((pathname: string, _params: RouteParams): boolean => {
    const app = AppRegistry.findByRoute(pathname);
    if (!app) return false; /* No es ruta de app → router renderiza normalmente */

    /* Verificar capacidad */
    const isAuthenticated = authStore.get().isAuthenticated;
    if (app.requires === 'admin' && !isAuthenticated) return false;
    if (app.requires === 'authenticated' && !isAuthenticated) return false;

    /* Si es singleton y ya está abierto, solo enfocar */
    if (app.singleton) {
      const existing = findOpenWindow(app.id);
      if (existing) {
        if (existing.state === 'minimized') {
          restoreWindow(existing.instanceId);
        }
        focusWindow(existing.instanceId);
        dispatchEvent({ type: 'window_focused', appId: app.id });
        return true; /* Interceptor manejó la ruta */
      }
    }

    /* Abrir nueva ventana */
    void openAppWindow(app.id);
    return true; /* Interceptor manejó la ruta */
  });
}

/** Abrir ventana para una app por ID, con parámetros opcionales de instancia. */
export async function openAppWindow(
  appId: string,
  params?: Record<string, string>,
): Promise<void> {
  const app = AppRegistry.get(appId);
  if (!app) return;

  /* Para apps non-singleton con params (Finder con folderId),
   * buscar ventana existente con los mismos params y enfocarla. */
  if (!app.singleton && params) {
    const paramKey = Object.values(params).join(':');
    const existing = windowStore.get().find(
      w => w.appId === appId && w._paramKey === paramKey,
    );
    if (existing) {
      if (existing.state === 'minimized') restoreWindow(existing.instanceId);
      focusWindow(existing.instanceId);
      return;
    }
  }

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
  const ctx = { signal: controller.signal, params };

  /* Instanciar contenido de la app */
  const view = await AppRegistry.instantiate(appId, ctx);
  if (!view) return;

  /* Abrir ventana con el contenido */
  openWindow(app, view, controller, undefined, params);
  dispatchEvent({ type: 'app_opened', appId });
}
