/* wandori.us — Article Service
 * Capa de servicio para operaciones con artículos.
 * Abstrae el API client para que los consumidores no dependan de transporte HTTP.
 * [Auditoría v4 §4.1] — Rompe acoplamiento directo a api.get/post en 5+ archivos. */

import { api } from '../api/client';
import type {
  Article,
  CreateArticleRequest,
  UpdateArticleRequest,
  PaginatedArticles,
} from '../api/types';

export const ArticleService = {
  /** Listar artículos públicos con paginación. */
  async list(page = 1, perPage = 10): Promise<PaginatedArticles> {
    return api.get<PaginatedArticles>(`/api/articles?page=${page}&per_page=${perPage}`);
  },

  /** Listar artículos por estado (admin). */
  async listByStatus(
    status: 'draft' | 'published' | 'all' = 'all',
    page = 1,
    perPage = 50,
  ): Promise<PaginatedArticles> {
    const params = new URLSearchParams({ page: String(page), per_page: String(perPage) });
    if (status !== 'all') params.set('status', status);
    return api.get<PaginatedArticles>(`/admin/articles?${params}`);
  },

  /** Obtener un artículo por slug (público). */
  async getBySlug(slug: string): Promise<Article> {
    return api.get<Article>(`/api/articles/slug/${encodeURIComponent(slug)}`);
  },

  /** Obtener un artículo por ID (admin). */
  async getById(id: string): Promise<Article> {
    return api.get<Article>(`/admin/articles/${id}`);
  },

  /** Obtener artículo por alias (About, etc). */
  async getByAlias(alias: string): Promise<Article> {
    return api.get<Article>(`/api/articles/alias/${encodeURIComponent(alias)}`);
  },

  /** Crear un nuevo artículo (admin). */
  async create(data: CreateArticleRequest): Promise<Article> {
    return api.post<Article>('/admin/articles', data);
  },

  /** Actualizar un artículo existente (admin). */
  async update(id: string, data: UpdateArticleRequest): Promise<Article> {
    return api.put<Article>(`/admin/articles/${id}`, data);
  },

  /** Eliminar un artículo (admin). */
  async delete(id: string): Promise<void> {
    return api.delete<void>(`/admin/articles/${id}`);
  },

  /** Publicar un artículo (admin). */
  async publish(id: string): Promise<Article> {
    return api.post<Article>(`/admin/articles/${id}/publish`, {});
  },

  /** Obtener slugs de artículos publicados (para sitemap/SEO). */
  async listPublishedSlugs(): Promise<Array<{ slug: string; date: string }>> {
    return api.get<Array<{ slug: string; date: string }>>('/api/articles/slugs');
  },
};
