/* wandori.us — Media Service
 * Capa de servicio para operaciones con archivos multimedia.
 * [297A-14] Alineado al contrato real del backend: público GET /api/media
 * (solo clean+public+active); admin bajo /api/admin/media con subida,
 * listado (incluye processing/rejected), papelera, soft delete y restore. */

import { api } from '../api/client';
import type { Media } from '../api/types';

export const MediaService = {
  /** Subir un archivo multimedia (admin). El tipo lo decide el backend. */
  async upload(file: File, options?: { articleId?: string; altText?: string }): Promise<Media> {
    const formData = new FormData();
    formData.append('file', file);
    if (options?.articleId) formData.append('article_id', options.articleId);
    if (options?.altText) formData.append('alt_text', options.altText);
    return api.upload<Media>('/api/admin/media', formData);
  },

  /** Listar archivos multimedia públicos: solo clean + public + active. */
  async list(): Promise<Media[]> {
    return api.get<Media[]>('/api/media');
  },

  /** Listar media admin: envelope activo, incluye processing/rejected. */
  async listAdmin(): Promise<Media[]> {
    return api.get<Media[]>('/api/admin/media');
  },

  /** Listar media en la papelera (admin). */
  async listTrashed(): Promise<Media[]> {
    return api.get<Media[]>('/api/admin/media/trashed');
  },

  /** Eliminar media (admin) — soft delete: pasa a la papelera. */
  async delete(id: string): Promise<void> {
    return api.delete<void>(`/api/admin/media/${id}`);
  },

  /** Restaurar media desde la papelera (admin). */
  async restore(id: string): Promise<void> {
    return api.post<void>(`/api/admin/media/${id}/restore`, {});
  },
};
