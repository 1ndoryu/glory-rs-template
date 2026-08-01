/* wandori.us — Admin Panel
 * Panel de administración. Orquesta tabs y delega a módulos.
 * [Auditoría v4 §1.2] Migrado a createEl(). */

import { SettingsService, AnalyticsService } from '../services';
import { showProfile } from '../store';
import { showToast } from '../components/ui/toast';
import { createTextarea } from '../components/ui/textarea';
import { createFontPanel } from '../features/settings/font-panel';
import { safeClick, safeRun, safeEffect } from '../utils/safe-async';
import { renderArticleList, openEditor, disposeAdminArticleLists } from './admin-articles';
import { renderProjectList, disposeAdminProjectLists } from './admin-projects';
import { renderProductList, disposeAdminProductLists } from './admin-products';
import { createTabs } from '../components/ui/tabs';
import { createVacio } from '../components/ui/empty-state';
import { createEl } from '../utils/dom';

/** Cleanup de recursos editoriales antes de desmontar la página Admin. */
export function disposeAdminPage(page: HTMLElement): void {
  disposeAdminArticleLists(page);
  disposeAdminProjectLists(page);
  disposeAdminProductLists(page);
}

export async function renderAdmin(): Promise<HTMLElement> {
  showProfile.set(false);

  const page = createEl('div', { className: 'admin-pagina' });

  /* El header (h1 "admin" + botón "salir") duplicaba la barra de título de la
   * ventana y el logout de la app Cuenta ("cerrar sesión" en account-view.ts).
   * Eliminado: la ventana ya se titula "Admin" y el logout vive en Cuenta. */
  /* Barra de pestañas universal (components/ui/tabs.ts). El estado activo lo
   * resuelve el componente (clase + aria-selected), sin inline styles ni
   * utilidades externas de padding/margin. [317A-1] */
  /* [317A-2] admin-contenido cierra la cadena de fill-height de la ventana
   * (app-contenedor -> admin-pagina -> admin-contenido) para que los estados
   * vacios centrados llenen toda la altura. */
  const contentArea = createEl('div', { id: 'admin-articulos', className: 'admin-contenido' });

  function switchTab(name: string): void {
    disposeAdminPage(page);
    contentArea.innerHTML = '';
    contentArea.id = `admin-${name}`;

    switch (name) {
      case 'articulos': {
        const btnNuevo = createEl('button', { className: 'boton mb-md', textContent: '+ nuevo articulo' });
        btnNuevo.addEventListener('click', () => openEditor());
        contentArea.appendChild(btnNuevo);
        const lista = createEl('div', { className: 'admin-lista' });
        contentArea.appendChild(lista);
        renderArticleList(lista);
        break;
      }
      case 'proyectos': {
        const lista = createEl('div', { className: 'admin-lista' });
        contentArea.appendChild(lista);
        renderProjectList(lista);
        break;
      }
      case 'productos': {
        const lista = createEl('div', { className: 'admin-lista' });
        contentArea.appendChild(lista);
        renderProductList(lista);
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

  const tabs = createTabs({
    tabs: [
      { id: 'articulos', label: 'articulos' },
      { id: 'proyectos', label: 'proyectos' },
      { id: 'productos', label: 'productos' },
      { id: 'fuentes', label: 'fuentes' },
      { id: 'sitio', label: 'sitio' },
      { id: 'estadisticas', label: 'estadisticas' },
    ],
    initial: 'articulos',
    onSwitch: switchTab,
  });

  page.append(tabs.el, contentArea);
  return page;
}

function renderSitioTab(): HTMLElement {
  const container = createEl('div', { className: 'flex-columna gap-lg' });
  container.appendChild(createEl('h3', { textContent: 'contenido del sitio' }));

  let aboutContent = '';
  const aboutArea = createTextarea({
    label: 'contenido about (html)',
    placeholder: '<h1>about</h1><p>tu contenido...</p>',
    rows: 8,
    onInput: (v) => { aboutContent = v; },
  });
  container.appendChild(aboutArea);

  safeEffect(async () => {
    const s = await SettingsService.getAll();
    aboutContent = s.about_content || '';
    const textarea = aboutArea.querySelector('textarea');
    if (textarea) textarea.value = aboutContent;
  })();

  const btnGuardarSitio = createEl('button', { className: 'boton', textContent: 'guardar' });
  btnGuardarSitio.addEventListener('click', safeClick(async () => {
    const result = await safeRun(SettingsService.save({ about_content: aboutContent }), 'error al guardar');
    if (result.ok) showToast('contenido actualizado');
  }));
  container.appendChild(btnGuardarSitio);
  return container;
}

function renderEstadisticasTab(contentArea: HTMLElement): void {
  const statsContainer = createEl('div');
  statsContainer.innerHTML = '<p class="cargando">cargando...</p>';
  contentArea.appendChild(statsContainer);

  AnalyticsService.getStats().then(stats => {
    statsContainer.innerHTML = '';
    const grid = createEl('div', { className: 'stats-grid' });
    const metrics = [
      { valor: stats.total_page_views, etiqueta: 'page views' },
      { valor: stats.total_clicks, etiqueta: 'clicks' },
      { valor: stats.total_downloads, etiqueta: 'descargas' },
      { valor: stats.total_purchases, etiqueta: 'compras' },
    ];

    for (const m of metrics) {
      const item = createEl('div', { className: 'stats-item' },
        createEl('div', { className: 'stats-valor', textContent: String(m.valor) }),
        createEl('div', { className: 'stats-etiqueta', textContent: m.etiqueta }),
      );
      grid.appendChild(item);
    }
    statsContainer.appendChild(grid);

    if (stats.top_articles.length > 0) {
      statsContainer.appendChild(createEl('h3', { className: 'mt-lg mb-md', textContent: 'articulos mas vistos' }));
      for (const art of stats.top_articles) {
        const item = createEl('div', { className: 'admin-item' },
          createEl('span', { textContent: art.title }),
          createEl('span', { textContent: `${art.views} views` }),
        );
        statsContainer.appendChild(item);
      }
    }
  }).catch(() => {
    /* [317A-2] Estado de error via componente universal (en vez de innerHTML). */
    statsContainer.replaceChildren(createVacio('error al cargar estadisticas'));
  });
}
