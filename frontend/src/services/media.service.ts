/* wandori.us — Media Service
 * Capa de servicio para operaciones con archivos multimedia.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.upload en font-panel.ts y admin-articles.ts. */

import { api } from '../api/client';
import type { Media } from '../api/types';

export const MediaService = {
  /** Subir un archivo multimedia. */
  async upload(file: File, folder?: string): Promise<Media> {
    const formData = new FormData();
    formData.append('file', file);
    if (folder) formData.append('folder', folder);
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
