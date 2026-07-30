/* wandori.us — Home Page
 * Pagina de inicio. Las entradas son condicionales (configurable).
 * Cuando no hay entradas, el profile (foto+nombre+links) esta centrado. */

import { ArticleService } from '../services';
import { navigate } from '../router';
import { trackArticleClick } from '../features/analytics/tracker';
import { resetMeta, setSiteJsonLd } from '../features/seo/meta';
import { siteConfig, showProfile } from '../store';
import type { Article } from '../api/types';

function formatDate(iso: string): string {
  const d = new Date(iso);
  const meses = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  return `${d.getDate()} ${meses[d.getMonth()]} ${d.getFullYear()}`;
}

function renderEntry(article: Article): HTMLElement {
  const entrada = document.createElement('article');
  entrada.className = 'entrada' + (article.is_pinned ? ' entrada-fijada' : '');

  const titulo = document.createElement('h2');
  titulo.className = 'entrada-titulo';

  const link = document.createElement('a');
  link.href = `/article/${article.slug}`;
  link.textContent = article.title;
  link.addEventListener('click', (e) => {
    e.preventDefault();
    trackArticleClick(article.id);
    navigate(`/article/${article.slug}`);
  });
  titulo.appendChild(link);

  const fecha = document.createElement('time');
  fecha.className = 'entrada-fecha';
  fecha.textContent = article.published_at ? formatDate(article.published_at) : formatDate(article.created_at);

  entrada.append(titulo, fecha);

  if (article.cover_image) {
    const img = document.createElement('img');
    img.className = 'entrada-imagen';
    img.src = article.cover_image;
    img.alt = article.title;
    img.loading = 'lazy';
    entrada.appendChild(img);
  }

  if (article.excerpt) {
    const extracto = document.createElement('p');
    extracto.className = 'entrada-extracto';
    extracto.textContent = article.excerpt;
    entrada.appendChild(extracto);
  }

  return entrada;
}

/* Articulos de ejemplo cuando la API no esta disponible */
export const demoArticles: Article[] = [
  {
    id: 'demo-1',
    title: 'el silencio de las maquinas',
    slug: 'el-silencio-de-las-maquinas',
    content: {},
    excerpt: 'hay algo en el ruido blanco de los servidores que me recuerda al mar.',
    cover_image: 'https://images.unsplash.com/photo-1518837695005-2083093ee35b?w=800&q=80',
    status: 'published',
    is_pinned: true,
    published_at: '2026-07-15T10:00:00Z',
    created_at: '2026-07-15T10:00:00Z',
    updated_at: '2026-07-15T10:00:00Z',
  },
  {
    id: 'demo-2',
    title: 'fragmentos de codigo y otras nostalgias',
    slug: 'fragmentos-de-codigo-y-otras-nostalgias',
    content: {},
    excerpt: 'escribi mi primera linea de codigo a los catorce anos.',
    cover_image: 'https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=800&q=80',
    status: 'published',
    is_pinned: false,
    published_at: '2026-06-22T14:30:00Z',
    created_at: '2026-06-22T14:30:00Z',
    updated_at: '2026-06-22T14:30:00Z',
  },
  {
    id: 'demo-3',
    title: 'sobre diseno y otros actos de fe',
    slug: 'sobre-diseno-y-otros-actos-de-fe',
    content: {},
    excerpt: 'el buen diseno no se nota. eso dicen.',
    cover_image: null,
    status: 'published',
    is_pinned: false,
    published_at: '2026-05-10T08:00:00Z',
    created_at: '2026-05-10T08:00:00Z',
    updated_at: '2026-05-10T08:00:00Z',
  },
  {
    id: 'demo-4',
    title: 'wandori.us — notas sobre construir en publico',
    slug: 'wandori-us-notas-sobre-construir-en-publico',
    content: {},
    excerpt: 'decidi construir este sitio en publico.',
    cover_image: 'https://images.unsplash.com/photo-1499750310107-5fef28a66643?w=800&q=80',
    status: 'published',
    is_pinned: false,
    published_at: '2026-04-01T12:00:00Z',
    created_at: '2026-04-01T12:00:00Z',
    updated_at: '2026-04-01T12:00:00Z',
  },
];

export async function renderHome(): Promise<HTMLElement> {
  resetMeta();
  setSiteJsonLd();
  showProfile.set(true);

  const page = document.createElement('div');

  /* Si no se muestran entradas en home, retornar vacio */
  if (!siteConfig.get().showEntriesOnHome) {
    return page;
  }

  /* Mostrar entradas */
  const cargando = document.createElement('p');
  cargando.className = 'cargando';
  cargando.textContent = 'cargando...';
  page.appendChild(cargando);

  let articles: Article[] = [];

  try {
    const data = await ArticleService.list(1, 20);
    articles = data.items;
  } catch {
    articles = [];
  }

  page.innerHTML = '';

  const sorted = [...articles].sort((a, b) => {
    if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;
    const dateA = a.published_at || a.created_at;
    const dateB = b.published_at || b.created_at;
    return new Date(dateB).getTime() - new Date(dateA).getTime();
  });

  for (const article of sorted) {
    page.appendChild(renderEntry(article));
  }

  return page;
}

/* Obtener articulos para el sidebar (exportado) */
export async function getArticles(): Promise<Article[]> {
  try {
    const data = await ArticleService.list(1, 50);
    return data.items;
  } catch {
    return [];
  }
}
