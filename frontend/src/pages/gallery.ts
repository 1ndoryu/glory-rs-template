/* wandori.us — Gallery Page
 * Muestra todas las imagenes de todos los articulos en un grid. */

import { MediaService } from '../services';
import { createModal } from '../components/ui/modal';
import { trackImageDownload } from '../features/analytics/tracker';
import { updateMeta, setPageJsonLd } from '../features/seo/meta';
import { showProfile } from '../store';
import type { Media } from '../api/types';

export async function renderGallery(): Promise<HTMLElement> {
  showProfile.set(true);
  updateMeta({ title: 'galeria', description: 'todas las imagenes de wandori.us' });
  setPageJsonLd('galeria', 'todas las imagenes de wandori.us');

  const page = document.createElement('div');

  const titulo = document.createElement('h1');
  titulo.textContent = 'galeria';
  titulo.style.marginBottom = 'var(--espacio-xl)';

  const cargando = document.createElement('p');
  cargando.className = 'cargando';
  cargando.textContent = 'cargando...';

  page.append(titulo, cargando);

  try {
    const mediaResponse = await MediaService.list();
    const media = (mediaResponse as any).items as Media[];
    page.innerHTML = '';

    page.appendChild(titulo);

    if (media.length === 0) {
      const vacio = document.createElement('p');
      vacio.className = 'vacio';
      vacio.textContent = 'no hay imagenes todavia';
      page.appendChild(vacio);
      return page;
    }

    const grid = document.createElement('div');
    grid.className = 'galeria-grid';

    for (const item of media) {
      const card = document.createElement('div');
      card.className = 'galeria-item';

      const img = document.createElement('img');
      img.src = item.file_path;
      img.alt = item.alt_text || '';
      img.loading = 'lazy';

      img.addEventListener('click', () => {
        const fullImg = document.createElement('img');
        fullImg.src = item.file_path;
        fullImg.alt = item.alt_text || '';
        fullImg.style.width = '100%';
        fullImg.style.border = 'var(--borde)';

        /* Boton descargar */
        const btnDescargar = document.createElement('button');
        btnDescargar.className = 'boton';
        btnDescargar.textContent = 'descargar';
        btnDescargar.style.marginTop = 'var(--espacio-md)';
        btnDescargar.addEventListener('click', () => {
          trackImageDownload(item.file_path);
          const a = document.createElement('a');
          a.href = item.file_path;
          a.download = item.file_path.split('/').pop() || 'imagen';
          a.click();
        });

        const container = document.createElement('div');
        container.append(fullImg, btnDescargar);

        createModal({
          titulo: 'imagen',
          contenido: container,
          ancho: '800px',
        });
      });

      card.appendChild(img);
      grid.appendChild(card);
    }

    page.appendChild(grid);
  } catch {
    /* API no disponible — mostrar imagenes de ejemplo */
    page.innerHTML = '';
    page.appendChild(titulo);

    const grid = document.createElement('div');
    grid.className = 'galeria-grid';

    const demoImages = [
      'https://images.unsplash.com/photo-1518837695005-2083093ee35b?w=400&q=80',
      'https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=400&q=80',
      'https://images.unsplash.com/photo-1499750310107-5fef28a66643?w=400&q=80',
      'https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=400&q=80',
      'https://images.unsplash.com/photo-1470071459604-3b5ec3a7fe05?w=400&q=80',
      'https://images.unsplash.com/photo-1441974231531-c6227db76b6e?w=400&q=80',
    ];

    for (const src of demoImages) {
      const card = document.createElement('div');
      card.className = 'galeria-item';
      const img = document.createElement('img');
      img.src = src;
      img.loading = 'lazy';
      card.appendChild(img);
      grid.appendChild(card);
    }

    page.appendChild(grid);
  }

  return page;
}
