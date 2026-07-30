/* wandori.us — Sidebar Component
 * Menu de navegacion + lista de entradas debajo de proyectos.
 * Responsive: se colapsa en mobile. */

import { navigate, getCurrentPath, onNavigate } from '../../router';
import { getArticles } from '../../pages/home';
import type { Article } from '../../api/types';

interface NavItem {
  etiqueta: string;
  ruta: string;
}

const navItems: NavItem[] = [
  { etiqueta: 'inicio', ruta: '/' },
  { etiqueta: 'about', ruta: '/about' },
  { etiqueta: 'galeria', ruta: '/gallery' },
  { etiqueta: 'proyectos', ruta: '/projects' },
];

/* Cache de articulos para evitar re-fetch en cada navegacion */
let articlesCache: Article[] | null = null;
let articlesFetching: Promise<Article[]> | null = null;

/** Limpiar cache de articulos — llamar despues de crear/editar/eliminar */
export function clearArticleCache(): void {
  articlesCache = null;
  articlesFetching = null;
}

async function getCachedArticles(): Promise<Article[]> {
  if (articlesCache) return articlesCache;
  if (articlesFetching) return articlesFetching;

  articlesFetching = getArticles().then((articles) => {
    articlesCache = articles;
    articlesFetching = null;
    return articles;
  }).catch(() => {
    articlesFetching = null;
    return [];
  });

  return articlesFetching;
}

export function createSidebar(): HTMLElement {
  const sidebar = document.createElement('aside');
  sidebar.className = 'sidebar';

  /* Navegacion */
  const nav = document.createElement('nav');
  nav.className = 'sidebar-nav';

  /* Separador invisible — espacio entre nav y entradas */
  const sep = document.createElement('div');
  sep.className = 'sidebar-separador';

  /* Lista de entradas */
  const entradas = document.createElement('div');
  entradas.className = 'sidebar-entradas';

  function renderNav(path: string): void {
    nav.innerHTML = '';
    for (const item of navItems) {
      const a = document.createElement('a');
      a.href = item.ruta;
      a.className = 'sidebar-nav-link';
      a.textContent = item.etiqueta;
      if (path === item.ruta || (item.ruta !== '/' && path.startsWith(item.ruta))) {
        a.classList.add('activo');
      }
      a.addEventListener('click', (e) => {
        e.preventDefault();
        navigate(item.ruta);
      });
      nav.appendChild(a);
    }
  }

  /* Cargar y renderizar entradas en el sidebar (usa cache) */
  async function renderEntries(path: string): Promise<void> {
    entradas.innerHTML = '';

    const articles = await getCachedArticles();
    const sorted = [...articles].sort((a, b) => {
      if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;
      const dateA = a.published_at || a.created_at;
      const dateB = b.published_at || b.created_at;
      return new Date(dateB).getTime() - new Date(dateA).getTime();
    });

    for (const article of sorted) {
      const a = document.createElement('a');
      a.href = `/article/${article.slug}`;
      a.className = 'sidebar-entrada-link';
      a.textContent = article.title;
      if (path === `/article/${article.slug}`) {
        a.classList.add('activo');
      }
      a.addEventListener('click', (e) => {
        e.preventDefault();
        navigate(`/article/${article.slug}`);
      });
      entradas.appendChild(a);
    }
  }

  renderNav(getCurrentPath());
  renderEntries(getCurrentPath());

  onNavigate((path) => {
    renderNav(path);
    renderEntries(path);
  });

  sidebar.append(nav, sep, entradas);
  return sidebar;
}
