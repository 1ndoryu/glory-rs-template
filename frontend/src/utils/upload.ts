/* wandori.us — Upload Helper
 * Sube archivos al backend via /api/media.
 * Retorna la URL del archivo subido. */

import { api } from '../api/client';
import type { Media } from '../api/types';

export interface UploadResult {
  url: string;
  media: Media;
}

/**
 * Sube un archivo al backend y retorna la URL.
 * @param file - Archivo a subir
 * @param articleId - ID del artículo asociado (opcional)
 * @param altText - Texto alternativo (opcional)
 */
export async function uploadFile(
  file: File,
  articleId?: string,
  altText?: string,
): Promise<UploadResult> {
  const formData = new FormData();
  formData.append('file', file);
  if (articleId) formData.append('article_id', articleId);
  if (altText) formData.append('alt_text', altText);

  const media = await api.upload<Media>('/api/media', formData);
  return { url: media.file_path, media };
}

/**
 * Abre un file picker y sube el archivo seleccionado.
 * @param accept - Tipos de archivo aceptados (ej: 'image/*')
 * @param articleId - ID del artículo asociado (opcional)
 */
export async function pickAndUpload(
  accept: string,
  articleId?: string,
): Promise<UploadResult | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.style.display = 'none';

    function cleanup(): void {
      if (input.parentNode) input.parentNode.removeChild(input);
    }

    input.addEventListener('change', async () => {
      const file = input.files?.[0];
      cleanup();
      if (!file) {
        resolve(null);
        return;
      }

      try {
        const result = await uploadFile(file, articleId);
        resolve(result);
      } catch (e) {
        reject(e);
      }
    });

    /* Cancelar si el usuario cierra el picker */
    input.addEventListener('cancel', () => {
      cleanup();
      resolve(null);
    });

    document.body.appendChild(input);
    input.click();
  });
}
