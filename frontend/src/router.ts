/* wandori.us — SPA Router
 * Routing por History API. Sin recarga de página.
 * Soporta parámetros dinámicos (:slug), guards de autenticación
 * y AbortSignal para lifecycle de vistas. */

import type { RenderContext } from './core/lifecycle';
export type { RenderContext };

export type RouteParams = Record<string, string>;

export interface Route {
  path: string;
  /** Recibe params y contexto con AbortSignal para teardown. */
  render: (params: RouteParams, ctx: RenderContext) => HTMLElement | Promise<HTMLElement>;
  guard?: () => boolean | Promise<boolean>;
}

type NavigationListener = (path: string) => void;

const routes: Route[] = [];
const listeners: Set<NavigationListener> = new Set();
let currentPath = '';
let outlet: HTMLElement | null = null;
let currentController: AbortController | null = null;

/* Registrar rutas */
export function addRoute(route: Route): void {
  routes.push(route);
}

/* Establecer el contenedor donde se renderizan las páginas */
export function setOutlet(el: HTMLElement): void {
  outlet = el;
}

/* Navegar programáticamente */
export function navigate(path: string): void {
  if (path === currentPath) return;
  history.pushState(null, '', path);
  handleRoute();
}

/* Obtener ruta actual */
export function getCurrentPath(): string {
  return currentPath;
}

/* Suscribirse a cambios de ruta */
export function onNavigate(listener: NavigationListener): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

/* Extraer parámetros de una ruta dinámica */
function matchRoute(pathname: string): { route: Route; params: RouteParams } | null {
  for (const route of routes) {
    const params = matchPath(route.path, pathname);
    if (params !== null) {
      return { route, params };
    }
  }
  return null;
}

function matchPath(pattern: string, pathname: string): RouteParams | null {
  const patternParts = pattern.split('/').filter(Boolean);
  const pathParts = pathname.split('/').filter(Boolean);

  if (patternParts.length !== pathParts.length) return null;

  const params: RouteParams = {};
  for (let i = 0; i < patternParts.length; i++) {
    if (patternParts[i].startsWith(':')) {
      params[patternParts[i].slice(1)] = decodeURIComponent(pathParts[i]);
    } else if (patternParts[i] !== pathParts[i]) {
      return null;
    }
  }
  return params;
}

/* Manejar cambio de ruta */
async function handleRoute(): Promise<void> {
  const pathname = window.location.pathname;
  currentPath = pathname;

  /* Abortar vista anterior si existe */
  if (currentController) {
    currentController.abort();
    currentController = null;
  }

  const matched = matchRoute(pathname);

  if (!matched) {
    /* 404 — ruta no encontrada */
    if (outlet) {
      outlet.innerHTML = '';
      const notFound = document.createElement('div');
      notFound.className = 'vacio';
      notFound.textContent = 'página no encontrada';
      outlet.appendChild(notFound);
    }
    return;
  }

  /* Verificar guard si existe */
  if (matched.route.guard) {
    const allowed = await matched.route.guard();
    if (!allowed) {
      navigate('/login');
      return;
    }
  }

  /* Crear nuevo AbortController para esta vista */
  currentController = new AbortController();
  const ctx: RenderContext = { signal: currentController.signal };

  /* Renderizar la página */
  if (outlet) {
    outlet.innerHTML = '';
    const element = await matched.route.render(matched.params, ctx);
    outlet.appendChild(element);
  }

  /* Notificar listeners */
  for (const listener of listeners) {
    listener(currentPath);
  }

  /* Scroll al top */
  window.scrollTo(0, 0);
}

/* Interceptar clicks en links internos */
function handleClick(e: MouseEvent): void {
  const target = e.target as HTMLElement;
  const anchor = target.closest('a');

  if (!anchor) return;
  if (anchor.origin !== window.location.origin) return;
  if (anchor.hasAttribute('data-external')) return;
  if (anchor.target === '_blank') return;

  e.preventDefault();
  navigate(anchor.pathname + anchor.search);
}

/* Inicializar el router */
export function initRouter(): void {
  window.addEventListener('popstate', handleRoute);
  document.addEventListener('click', handleClick);
  handleRoute();
}

