/* wandori.us — Reader (Article Viewer)
 * Lector de artículos del OS. Carga contenido real desde la API.
 * [Auditoría v2] Reemplaza el preview hardcodeado de 297A-2.
 */

import { api } from '../../../../api/client';
import { appendSanitizedHtml } from '../../../../utils/sanitize-html';

export interface ReaderOptions {
  /** Slug del artículo a cargar. Si no se provee, muestra placeholder. */
  slug?: string;
  /** Título override (para artículos aún no cargados). */
  title?: string;
}

/** Crear el componente Reader que carga un artículo real desde la API. */
export function createReaderPreview(options: ReaderOptions): HTMLElement {
  const article = document.createElement('article');
  article.className = 'desktop-reader';

  const header = document.createElement('header');
  header.className = 'desktop-reader__header';

  const titleEl = document.createElement('h1');
  titleEl.className = 'desktop-reader__title';
  titleEl.textContent = options.title ?? 'Cargando…';

  const dateEl = document.createElement('time') as HTMLTimeElement;
  dateEl.className = 'desktop-reader__date';
  header.append(titleEl, dateEl);

  const body = document.createElement('div');
  body.className = 'desktop-reader__body';

  article.append(header, body);

  /* Cargar contenido real si hay slug */
  if (options.slug) {
    void loadArticle(options.slug, titleEl, dateEl, body);
  } else {
    /* Placeholder sin slug */
    const placeholder = document.createElement('p');
    placeholder.className = 'desktop-reader__empty';
    placeholder.textContent = 'Selecciona un artículo para leer.';
    body.appendChild(placeholder);
  }

  return article;
}

/** Cargar artículo desde la API y renderizar en el componente. */
async function loadArticle(
  slug: string,
  titleEl: HTMLElement,
  dateEl: HTMLTimeElement,
  body: HTMLElement,
): Promise<void> {
  try {
    const data = await api.get<{
      title: string;
      content: string;
      published_at: string | null;
      cover_image: string | null;
    }>(`/api/articles/${slug}`);

    titleEl.textContent = data.title;

    if (data.published_at) {
      const d = new Date(data.published_at);
      dateEl.dateTime = data.published_at;
      dateEl.textContent = d.toLocaleDateString('es', {
        day: 'numeric',
        month: 'long',
        year: 'numeric',
      });
    }

    /* Imagen de portada */
    if (data.cover_image) {
      const img = document.createElement('img');
      img.className = 'desktop-reader__image';
      img.src = data.cover_image;
      img.alt = data.title;
      img.loading = 'lazy';
      body.appendChild(img);
    }

    /* Contenido sanitizado */
    if (data.content) {
      appendSanitizedHtml(body, data.content);
    } else {
      const empty = document.createElement('p');
      empty.className = 'desktop-reader__empty';
      empty.textContent = 'Este artículo no tiene contenido.';
      body.appendChild(empty);
    }
  } catch {
    titleEl.textContent = 'Error al cargar';
    const error = document.createElement('p');
    error.className = 'desktop-reader__empty';
    error.textContent = `No se pudo cargar el artículo "${slug}".`;
    body.appendChild(error);
  }
}
