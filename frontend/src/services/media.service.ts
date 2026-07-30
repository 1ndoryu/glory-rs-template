/* wandori.us — Media Service
 * Capa de servicio para operaciones con archivos multimedia.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.upload en font-panel.ts y admin-articles.ts. */

import { api } from '../api/client';
import type { Media } from '../api/types';

export const MediaService = {
  /** Subir un archivo multimedia.
   *  @param file - Archivo a subir
   *  @param articleId - ID del artículo asociado (opcional)
   *  @param altText - Texto alternativo (opcional)
   *  @param folder - Carpeta de destino (opcional) */
  async upload(file: File, options?: { articleId?: string; altText?: string; folder?: string }): Promise<Media> {
    const formData = new FormData();
    formData.append('file', file);
    if (options?.articleId) formData.append('article_id', options.articleId);
    if (options?.altText) formData.append('alt_text', options.altText);
    if (options?.folder) formData.append('folder', options.folder);
    return api.upload<Media>('/api/media/upload', formData);
  },

  /** Obtener metadatos de un archivo multimedia. */
  async getById(id: string): Promise<Media> {
    return api.get<Media>(`/api/media/${id}`);
  },

  /** Listar archivos multimedia (admin). */
  async list(page = 1, perPage = 50): Promise<{ items: Media[]; total: number }> {
    return api.get<{ items: Media[]; total: number }>(
      `/admin/media?page=${page}&per_page=${perPage}`,
    );
  },

  /** Eliminar un archivo multimedia (admin). */
  async delete(id: string): Promise<void> {
    return api.delete<void>(`/admin/media/${id}`);
  },
};
