/* wandori.us — Route App Adapter
 * Conecta el router existente con el AppRegistry y WindowManager.
 * Registra un interceptor de rutas: cuando el router navega a una ruta
 * que una app maneja, el adapter abre una ventana y evita que el router
 * renderice en el outlet. Esto elimina el doble rendering. */

import { onNavigate, setRouteInterceptor, pushPath, showRouteNotFound, type RouteParams } from '../../router';
import { AppRegistry } from './app-registry';
import {
  openWindow,
  focusWindow,
  findOpenWindow,
  restoreWindow,
  closeWindow,
  windowStore,
} from './window-manager';
import { clearMobileStack } from '../mobile/mobile-stack';
import { dispatchEvent } from '../analytics/dispatcher';
import { authStore } from '../../store';
import { getPresentationMode } from '../../utils/viewport';
import { getCanonicalAppPath, parseAppParams, stableParamsKey, type AppOpenHistory } from './deep-links';

/** Handler de presentación móvil registrado por el MobileShell. */
let mobileOpenHandler: ((appId: string, params?: Record<string, string>, options?: { history?: AppOpenHistory }) => Promise<void>) | null = null;

export function setMobileOpenHandler(
  handler: ((appId: string, params?: Record<string, string>, options?: { history?: AppOpenHistory }) => Promise<void>) | null,
): () => void {
  mobileOpenHandler = handler;
  return () => {
    if (mobileOpenHandler === handler) mobileOpenHandler = null;
  };
}

/** Liberar contenido del OS cuando la URL deja una app válida.
 * La ventana shell de Perfil no pertenece a una app y se conserva. */
function clearRuntimeApps(): void {
  if (getPresentationMode() === 'mobile') {
    clearMobileStack('sync');
    return;
  }

  for (const win of windowStore.get()) {
    if (win.instanceId !== 'shell-profile') closeWindow(win.instanceId, 'sync');
  }
}

function reconcileRuntimeForRoute(pathname: string): void {
  if (AppRegistry.findByRoute(pathname)) return;
  clearRuntimeApps();
}

/** Registrar el interceptor de rutas en el router.
 * Llamar una vez desde main.ts. */
export function initRouteAppAdapter(): () => void {
  const stopRouteReconciliation = onNavigate(reconcileRuntimeForRoute);
  const stopInterceptor = setRouteInterceptor(async (pathname: string, params: RouteParams): Promise<boolean> => {
    const app = AppRegistry.findByRoute(pathname);
    if (!app) return false; /* No es ruta de app → router renderiza normalmente */

    /* Si el shell móvil todavía no está listo, dejar que el router renderice
     * la ruta normal; nunca interceptar y dejar un outlet vacío. */
    if (getPresentationMode() === 'mobile' && !mobileOpenHandler) return false;

    /* Validar parámetros públicos antes de hidratar la app. */
    const safeParams = parseAppParams(app, params);
    if (safeParams === null) {
      clearRuntimeApps();
      showRouteNotFound();
      return true;
    }

    /* Verificar capacidad */
    const capability = authStore.get().capability;
    const hierarchy = ['public', 'authenticated', 'admin'] as const;
    const currentLevel = hierarchy.indexOf(capability);
    const requiredLevel = hierarchy.indexOf(app.requires);
    if (requiredLevel > currentLevel) {
      clearRuntimeApps();
      showRouteNotFound();
      return true;
    }

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

    /* Abrir la app con los parámetros de la ruta; el adapter elige
     * ventana desktop o pila móvil según la presentación activa. */
    await openAppWindow(app.id, safeParams, { history: 'none' });
    return true; /* Interceptor manejó la ruta */
  });

  return () => {
    stopInterceptor();
    stopRouteReconciliation();
  };
}

/** Abrir ventana para una app por ID, con parámetros opcionales de instancia. */
export async function openAppWindow(
  appId: string,
  params?: Record<string, string>,
  options: { history?: AppOpenHistory } = {},
): Promise<void> {
  const app = AppRegistry.get(appId);
  if (!app) return;

  /* La apertura programática también es una frontera de autorización:
   * no depende de que la llamada venga del router o de un comando visible. */
  const capability = authStore.get().capability;
  const hierarchy = ['public', 'authenticated', 'admin'] as const;
  if (hierarchy.indexOf(app.requires) > hierarchy.indexOf(capability)) return;

  /* Para apps non-singleton con params (Finder con folderId),
   * buscar ventana existente con los mismos params y enfocarla. */
  if (!app.singleton && params) {
    const paramKey = stableParamsKey(params);
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

  /* La presentación decide el chrome; la app y sus parámetros siguen siendo los mismos. */
  if (getPresentationMode() === 'mobile') {
    /* En móvil nunca caer al chrome desktop por una carrera de inicialización. */
    if (mobileOpenHandler) await mobileOpenHandler(appId, params, options);
    return;
  }

  /* Crear AbortController y RenderContext */
  const controller = new AbortController();
  const ctx = { signal: controller.signal, params };

  /* Instanciar contenido de la app */
  const view = await AppRegistry.instantiate(appId, ctx);
  if (!view) return;

  /* Para Finder non-singleton, usar el nombre de la carpeta como título de ventana */
  let titleOverride: string | undefined;
  if (appId === 'finder' && params?.folderId) {
    const { workspaceStore } = await import('./workspace/workspace-store');
    const ws = workspaceStore.get();
    const folderNode = ws.nodes[params.folderId];
    titleOverride = folderNode?.label ?? (params.folderId === 'desktop' ? 'Escritorio' : 'Galería');
  }

  const canonicalPath = getCanonicalAppPath(app, params);
  /* Push antes de publicar el cambio en windowStore: el sincronizador verá
   * la URL ya actualizada y no podrá reemplazar la entrada intencional. */
  if (options.history !== 'none' && canonicalPath) pushPath(canonicalPath);
  openWindow(app, view, controller, undefined, params, titleOverride);
  dispatchEvent({ type: 'app_opened', appId });
}
