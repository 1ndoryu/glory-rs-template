/* wandori.us — SPA Router
 * Routing por History API. Sin recarga de página.
 * Soporta parámetros dinámicos (:slug), guards de autenticación
 * y AbortSignal para lifecycle de vistas. */

import { createEl } from './utils/dom';
import type { RenderContext } from './core/lifecycle';
export type { RenderContext };

export type RouteParams = Record<string, string>;

export interface Route {
  path: string;
  render: (params: RouteParams, ctx: RenderContext) => HTMLElement | Promise<HTMLElement>;
  guard?: () => boolean | Promise<boolean>;
}

type NavigationListener = (path: string) => void;
type RouteInterceptor = (pathname: string, params: RouteParams) => boolean | Promise<boolean>;

const routes: Route[] = [];
const listeners: Set<NavigationListener> = new Set();
let currentPath = '';
let outlet: HTMLElement | null = null;
let currentController: AbortController | null = null;
let routeInterceptor: RouteInterceptor | null = null;

export function addRoute(route: Route): void {
  routes.push(route);
}

export function setOutlet(el: HTMLElement): void {
  outlet = el;
}

export function navigate(path: string): void {
  if (path === currentPath) return;
  history.pushState(null, '', path);
  handleRoute();
}

export function getCurrentPath(): string {
  return currentPath;
}

export function setRouteInterceptor(interceptor: RouteInterceptor): void {
  routeInterceptor = interceptor;
}

export function onNavigate(listener: NavigationListener): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

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

async function handleRoute(): Promise<void> {
  const pathname = window.location.pathname;
  currentPath = pathname;

  if (currentController) {
    currentController.abort();
    currentController = null;
  }

  const matched = matchRoute(pathname);

  if (!matched) {
    if (outlet) {
      outlet.innerHTML = '';
      outlet.appendChild(createEl('div', { className: 'vacio', textContent: 'página no encontrada' }));
    }
    return;
  }

  if (matched.route.guard) {
    const allowed = await matched.route.guard();
    if (!allowed) {
      navigate('/login');
      return;
    }
  }

  if (routeInterceptor) {
    const handled = await routeInterceptor(pathname, matched.params);
    if (handled) {
      for (const listener of listeners) {
        listener(currentPath);
      }
      return;
    }
  }

  currentController = new AbortController();
  const ctx: RenderContext = { signal: currentController.signal };

  if (outlet) {
    outlet.innerHTML = '';
    const element = await matched.route.render(matched.params, ctx);
    outlet.appendChild(element);
  }

  for (const listener of listeners) {
    listener(currentPath);
  }

  window.scrollTo(0, 0);
}

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

export function initRouter(): () => void {
  window.addEventListener('popstate', handleRoute);
  document.addEventListener('click', handleClick);
  handleRoute();
  return () => {
    window.removeEventListener('popstate', handleRoute);
    document.removeEventListener('click', handleClick);
  };
}
