/* wandori.us — Admin Panel
 * Panel de administración. Orquesta tabs y delega a módulos.
 * Solo accesible si autenticado. */

import { AuthService, SettingsService, AnalyticsService } from '../services';
import { authStore, showProfile } from '../store';
import { navigate } from '../router';
import { showToast } from '../components/ui/toast';
import { createTextarea } from '../components/ui/textarea';
import { createFontPanel } from '../features/settings/font-panel';
import { renderArticleList, openEditor } from './admin-articles';
import { renderProjectList } from './admin-projects';

/* === Render principal del admin === */
export async function renderAdmin(): Promise<HTMLElement> {
  showProfile.set(false);

  const page = document.createElement('div');
  page.className = 'admin-pagina';

  const header = document.createElement('div');
  header.className = 'admin-header';

  const titulo = document.createElement('h1');
  titulo.textContent = 'admin';

  const btnLogout = document.createElement('button');
  btnLogout.className = 'boton';
  btnLogout.textContent = 'salir';
  btnLogout.addEventListener('click', async () => {
    try { await AuthService.logout(); } catch { /* ignorar */ }
    authStore.set({ isAuthenticated: false, userId: null });
    showToast('sesion cerrada');
    navigate('/');
  });

  header.append(titulo, btnLogout);
  page.appendChild(header);

  /* Tabs */
  const tabs = document.createElement('div');
  tabs.className = 'flex-fila gap-lg mb-lg border-bottom';
  tabs.style.paddingBottom = 'var(--espacio-sm)';

  const contentArea = document.createElement('div');
  contentArea.id = 'admin-articulos';

  const tabNames = ['articulos', 'proyectos', 'fuentes', 'sitio', 'estadisticas'];

  function switchTab(name: string): void {
    tabs.querySelectorAll('.boton').forEach(b => {
      (b as HTMLElement).style.fontWeight = b.textContent === name ? 'var(--peso-medio)' : 'var(--peso-normal)';
    });
    contentArea.innerHTML = '';
    contentArea.id = `admin-${name}`;

    switch (name) {
      case 'articulos': {
        const btnNuevo = document.createElement('button');
        btnNuevo.className = 'boton mb-md';
        btnNuevo.textContent = '+ nuevo articulo';
        btnNuevo.addEventListener('click', () => openEditor());
        contentArea.appendChild(btnNuevo);

        const lista = document.createElement('div');
        lista.className = 'admin-lista';
        contentArea.appendChild(lista);
        renderArticleList(lista);
        break;
      }
      case 'proyectos': {
        const lista = document.createElement('div');
        lista.className = 'admin-lista';
        contentArea.appendChild(lista);
        renderProjectList(lista);
        break;
      }
      case 'fuentes':
        contentArea.appendChild(createFontPanel());
        break;
      case 'sitio':
        contentArea.appendChild(renderSitioTab());
        break;
      case 'estadisticas':
        renderEstadisticasTab(contentArea);
        break;
    }
  }

  for (const name of tabNames) {
    const btn = document.createElement('button');
    btn.className = 'boton';
    btn.textContent = name;
    btn.addEventListener('click', () => switchTab(name));
    tabs.appendChild(btn);
  }

  page.append(tabs, contentArea);
  switchTab('articulos');
  return page;
}

/* === Tab: Sitio === */
function renderSitioTab(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'flex-columna gap-lg';

  const sitioTitulo = document.createElement('h3');
  sitioTitulo.textContent = 'contenido del sitio';
  container.appendChild(sitioTitulo);

  let aboutContent = '';
  const aboutArea = createTextarea({
    label: 'contenido about (html)',
    placeholder: '<h1>about</h1><p>tu contenido...</p>',
    rows: 8,
    onInput: (v) => { aboutContent = v; },
  });
  container.appendChild(aboutArea);

  SettingsService.getAll().then(s => {
    aboutContent = s.about_content || '';
    const textarea = aboutArea.querySelector('textarea');
    if (textarea) textarea.value = aboutContent;
  }).catch(() => {});

  const btnGuardarSitio = document.createElement('button');
  btnGuardarSitio.className = 'boton';
  btnGuardarSitio.textContent = 'guardar';
  btnGuardarSitio.addEventListener('click', async () => {
    try {
      await SettingsService.save({ about_content: aboutContent });
      showToast('contenido actualizado');
    } catch { showToast('error al guardar'); }
  });
  container.appendChild(btnGuardarSitio);
  return container;
}

/* === Tab: Estadísticas === */
function renderEstadisticasTab(contentArea: HTMLElement): void {
  const statsContainer = document.createElement('div');
  statsContainer.innerHTML = '<p class="cargando">cargando...</p>';
  contentArea.appendChild(statsContainer);

  AnalyticsService.getStats().then(stats => {
    statsContainer.innerHTML = '';

    const grid = document.createElement('div');
    grid.className = 'stats-grid';

    const metrics = [
      { valor: stats.total_page_views, etiqueta: 'page views' },
      { valor: stats.total_clicks, etiqueta: 'clicks' },
      { valor: stats.total_downloads, etiqueta: 'descargas' },
      { valor: stats.total_purchases, etiqueta: 'compras' },
    ];

    for (const m of metrics) {
      const item = document.createElement('div');
      item.className = 'stats-item';
      const valor = document.createElement('div');
      valor.className = 'stats-valor';
      valor.textContent = String(m.valor);
      const etiqueta = document.createElement('div');
      etiqueta.className = 'stats-etiqueta';
      etiqueta.textContent = m.etiqueta;
      item.append(valor, etiqueta);
      grid.appendChild(item);
    }
    statsContainer.appendChild(grid);

    if (stats.top_articles.length > 0) {
      const tituloTop = document.createElement('h3');
      tituloTop.textContent = 'articulos mas vistos';
      tituloTop.classList.add('mt-lg', 'mb-md');
      statsContainer.appendChild(tituloTop);

      for (const art of stats.top_articles) {
        const item = document.createElement('div');
        item.className = 'admin-item';
        const nombre = document.createElement('span');
        nombre.textContent = art.title;
        const views = document.createElement('span');
        views.textContent = `${art.views} views`;
        item.append(nombre, views);
        statsContainer.appendChild(item);
      }
    }
  }).catch(() => {
    statsContainer.innerHTML = '<p class="vacio">error al cargar estadisticas</p>';
  });
}
