/* wandori.us — Entry Point
 * Inicializa el router, estilos, layout y paginas. */

/* Estilos */
import './styles/variables.css';
import './styles/reset.css';
import './styles/base.css';
import './styles/layout.css';
import './styles/components.css';
import './styles/pages.css';
import './styles/desktop/desktop-shell.css';
import './styles/desktop/desktop-menu.css';
import './styles/desktop/desktop-apps.css';
import './styles/desktop/desktop-window.css';
import './styles/desktop/desktop-responsive.css';
import './styles/desktop/desktop-context-menu.css';

/* Core */
import { addRoute, setOutlet, initRouter } from './router';
import { createSidebar } from './components/layout/sidebar';
import { createProfile } from './components/layout/profile';
import { createDesktopShell } from './features/desktop/desktop-shell';
import './features/runtime/app-registration';
import './features/runtime/command-registration';
import { initKeyboardShortcuts } from './features/runtime/command-registration';
import { initRouteAppAdapter } from './features/runtime/route-app-adapter';
import { AppRegistry } from './features/runtime/app-registry';
import { initResourceTypeRegistry } from './features/runtime/resource-type-registry';
import { setActorCategory } from './features/analytics/dispatcher';
import { loadSavedFonts } from './features/settings/font-panel';
import { initTracking, trackPageView } from './features/analytics/tracker';
import { authStore, showProfile, showSidebar, siteConfig } from './store';
import { api } from './api/client';


/* Pages */
import { renderHome } from './pages/home';
import { renderArticle } from './pages/article';
import { renderAbout } from './pages/about';
import { renderGallery } from './pages/gallery';
import { renderProjects } from './pages/projects';
import { renderLogin } from './pages/login';
import { renderAdmin } from './pages/admin';
import { renderCheckoutSuccess, renderCheckoutCancel } from './pages/checkout';

/* === Guard de autenticacion === */
function requireAuth(): boolean {
  return authStore.get().isAuthenticated;
}

/* === Registrar rutas === */
addRoute({ path: '/', render: () => renderHome() });
addRoute({ path: '/article/:slug', render: (params) => renderArticle(params) });
addRoute({ path: '/about', render: () => renderAbout() });
addRoute({ path: '/gallery', render: () => renderGallery() });
addRoute({ path: '/projects', render: () => renderProjects() });
addRoute({ path: '/login', render: () => renderLogin() });
addRoute({ path: '/admin', render: () => renderAdmin(), guard: requireAuth });
addRoute({ path: '/checkout/success', render: () => renderCheckoutSuccess() });
addRoute({ path: '/checkout/cancel', render: () => renderCheckoutCancel() });

/* === Inicializar aplicacion === */
async function initApp(): Promise<void> {
  const app = document.getElementById('app');
  if (!app) return;

  /* [297A-8] Verificar sesión existente al arrancar.
   * Las cookies HttpOnly se envían automáticamente con credentials: 'include'.
   * Si /auth/me responde con usuario válido, marcamos como autenticado. */
  try {
    const user = await api.get<{ id: string; email: string }>('/api/auth/me');
    authStore.set({ isAuthenticated: true, userId: user.id });
    setActorCategory('authenticated');
  } catch {
    /* No hay sesión válida — permanecer como invitado */
    authStore.set({ isAuthenticated: false, userId: null });
    setActorCategory('anonymous');
  }

  /* Cargar fuentes y settings antes de renderizar */
  await loadSavedFonts();

  /* Limpiar */
  app.innerHTML = '';

  /* Sidebar — menu + entradas */
  const sidebar = createSidebar();
  app.appendChild(sidebar);

  /* [Plan §2.2] navigation.toggleExternalNav: toggle sidebar */
  showSidebar.subscribe((visible) => {
    sidebar.style.display = visible ? '' : 'none';
    app.style.gridTemplateColumns = visible ? '' : '1fr';
  });

  /* Columna derecha: superficie exclusiva del escritorio */
  const columnaDerecha = document.createElement('div');
  columnaDerecha.className = 'columna-derecha';

  /* Perfil y outlet conservan sus contratos; el shell solo cambia su presentación. */
  const profile = createProfile();
  const contenido = document.createElement('main');
  contenido.className = 'contenido-principal';

  const desktop = createDesktopShell(profile, contenido);
  columnaDerecha.appendChild(desktop.element);

  app.appendChild(columnaDerecha);

  /* [Plan §9.1] Actualizar actor category al cambiar auth durante la sesión */
  authStore.subscribe((state) => {
    setActorCategory(state.isAuthenticated ? 'authenticated' : 'anonymous');
  });

  /* Control de visibilidad del profile:
   * Se oculta cuando se esta viendo un articulo.
   * Usa setProfileVisible para mantener taskbar sincronizado. */
  showProfile.subscribe((visible) => {
    desktop.setProfileVisible(visible);
    /* Cuando no hay profile, centrar el contenido */
    if (visible) {
      columnaDerecha.classList.remove('columna-derecha--sin-profile');
    } else {
      columnaDerecha.classList.add('columna-derecha--sin-profile');
    }
  });

  /* Configurar outlet del router */
  setOutlet(contenido);

  /* Control de visibilidad del contenido principal (legacy outlet):
   * Se oculta cuando la ruta es manejada por una app del runtime (ventana propia),
   * o en home cuando las entradas están desactivadas. */
  function updateContenidoVisibility(): void {
    const path = window.location.pathname;
    const isHome = path === '/';
    const showEntries = siteConfig.get().showEntriesOnHome;
    const isAppRoute = !!AppRegistry.findByRoute(path);
    desktop.contentWindow.style.display = (isAppRoute || (isHome && !showEntries)) ? 'none' : '';
  }
  siteConfig.subscribe(() => updateContenidoVisibility());
  updateContenidoVisibility();
  import('./router').then(({ onNavigate }) => {
    onNavigate((path) => {
      trackPageView(path);
      updateContenidoVisibility();
    });
  });

  /* Tracking de page views */
  initTracking();

  /* Iniciar atajos de teclado del OS */
  initKeyboardShortcuts();

  /* Iniciar RouteAppAdapter — intercepta rutas de apps para abrir ventanas */
  initRouteAppAdapter();

  /* Iniciar Resource Type Registry — asociaciones tipo→app */
  initResourceTypeRegistry();

  /* Iniciar router */
  initRouter();
}

/* Arrancar cuando el DOM este listo */
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}
