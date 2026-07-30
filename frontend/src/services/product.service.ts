/* wandori.us — Product Service
 * Capa de servicio para operaciones con productos.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.get/post en pages/article.ts y checkout.ts. */

import { api } from '../api/client';
import type { Product, CreateProductRequest } from '../api/types';

export const ProductService = {
  /** Obtener un producto por ID. */
  async getById(id: string): Promise<Product> {
    return api.get<Product>(`/api/products/${id}`);
  },

  /** Obtener producto asociado a un artículo. */
  async getByArticleId(articleId: string): Promise<Product | null> {
    try {
      return await api.get<Product>(`/api/products/by-article/${articleId}`);
    } catch {
      return null;
    }
  },

  /** Listar productos activos (público). */
  async listActive(): Promise<Product[]> {
    return api.get<Product[]>('/api/products');
  },

  /** Listar todos los productos (admin). */
  async listAll(): Promise<Product[]> {
    return api.get<Product[]>('/admin/products');
  },

  /** Crear un nuevo producto (admin). */
  async create(data: CreateProductRequest): Promise<Product> {
    return api.post<Product>('/admin/products', data);
  },

  /** Actualizar un producto (admin). */
  async update(id: string, data: Partial<CreateProductRequest & { is_active: boolean }>): Promise<Product> {
    return api.put<Product>(`/admin/products/${id}`, data);
  },

  /** Eliminar un producto (admin). */
  async delete(id: string): Promise<void> {
    return api.delete<void>(`/admin/products/${id}`);
  },
};
