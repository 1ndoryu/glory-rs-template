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

/* Core */
import { addRoute, setOutlet, initRouter } from './router';
import { createSidebar } from './components/layout/sidebar';
import { createProfile } from './components/layout/profile';
import { createDesktopConcept } from './features/desktop/desktop-concept';
import { loadSavedFonts } from './features/settings/font-panel';
import { initTracking, trackPageView } from './features/analytics/tracker';
import { authStore, showProfile, siteConfig } from './store';


import { navigate } from './router';

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

  /* Auto-login en modo dev: siempre obtener un JWT válido (el token en
   * localStorage puede estar viejo/invalido tras recrear la BD). */
  if (import.meta.env.DEV) {
    const email = import.meta.env.VITE_ADMIN_EMAIL || 'wandorius@wandori.us';
    const password = import.meta.env.VITE_ADMIN_PASSWORD || 'Wand0rius!2026';
    const body = JSON.stringify({ email, password });
    const headers = { 'Content-Type': 'application/json' };

    try {
      let res = await fetch('/api/auth/login', { method: 'POST', headers, body });
      if (!res.ok) {
        /* Usuario no existe — registrarlo y usar su token directamente */
        res = await fetch('/api/auth/register', { method: 'POST', headers, body });
      }
      if (res.ok) {
        const data: { token: string } = await res.json();
        authStore.set({ token: data.token, isAuthenticated: true });
      }
    } catch (err) {
      console.warn('Auto-login dev falló:', err);
    }
  }

  /* Cargar fuentes y settings antes de renderizar */
  await loadSavedFonts();

  /* Limpiar */
  app.innerHTML = '';

  /* Sidebar — menu + entradas */
  const sidebar = createSidebar();
  app.appendChild(sidebar);

  /* Columna derecha: superficie exclusiva del escritorio */
  const columnaDerecha = document.createElement('div');
  columnaDerecha.className = 'columna-derecha';

  /* Perfil y outlet conservan sus contratos; el shell solo cambia su presentación. */
  const profile = createProfile();
  const contenido = document.createElement('main');
  contenido.className = 'contenido-principal';

  const desktop = createDesktopConcept(profile, contenido, {
    showAdminTools: authStore.get().isAuthenticated,
    onOpenAdmin: () => navigate('/admin'),
  });
  columnaDerecha.appendChild(desktop.element);

  app.appendChild(columnaDerecha);

  /* Control de visibilidad del profile:
   * Se oculta cuando se esta viendo un articulo */
  showProfile.subscribe((visible) => {
    desktop.profileWindow.style.display = visible ? '' : 'none';
    /* Cuando no hay profile, centrar el contenido */
    if (visible) {
      columnaDerecha.classList.remove('columna-derecha--sin-profile');
    } else {
      columnaDerecha.classList.add('columna-derecha--sin-profile');
    }
  });

  /* Configurar outlet del router */
  setOutlet(contenido);

  /* Control de visibilidad del contenido principal:
   * Se oculta en home cuando las entradas están desactivadas */
  function updateContenidoVisibility(): void {
    const isHome = window.location.pathname === '/';
    const showEntries = siteConfig.get().showEntriesOnHome;
    desktop.contentWindow.style.display = (isHome && !showEntries) ? 'none' : '';
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

  /* Iniciar router */
  initRouter();
}

/* Arrancar cuando el DOM este listo */
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}
