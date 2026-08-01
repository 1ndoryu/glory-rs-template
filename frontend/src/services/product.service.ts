/* wandori.us — Product Service
 * Capa de servicio para operaciones con productos.
 * [297A-14] Alineado al contrato canónico: admin bajo /api/admin/products,
 * público por artículo y checkout en rutas públicas reales. */

import { api } from '../api/client';
import type { CreateProductRequest, Product, UpdateProductRequest } from '../api/types';

export const ProductService = {
  /** Obtener un producto por ID (admin). */
  async getById(id: string, options?: { signal?: AbortSignal }): Promise<Product> {
    return api.get<Product>(`/api/admin/products/${id}`, options);
  },

  /** Obtener productos activos asociados a un artículo (público). */
  async getByArticleId(articleId: string): Promise<Product | null> {
    try {
      const products = await api.get<Product[]>(`/api/articles/${articleId}/products`);
      return products[0] ?? null;
    } catch {
      return null;
    }
  },

  /** Listar todos los productos (admin). */
  async listAll(): Promise<Product[]> {
    return api.get<Product[]>('/api/admin/products');
  },

  /** Crear un nuevo producto (admin). Nace inactivo/private por defecto. */
  async create(data: CreateProductRequest): Promise<Product> {
    return api.post<Product>('/api/admin/products', data);
  },

  /** Actualizar un producto (admin). */
  async update(id: string, data: UpdateProductRequest): Promise<Product> {
    return api.put<Product>(`/api/admin/products/${id}`, data);
  },

  /** Eliminar un producto (admin). */
  async delete(id: string): Promise<void> {
    return api.delete<void>(`/api/admin/products/${id}`);
  },

  /** Crear sesión de checkout para un producto (público). */
  async createCheckout(productId: string, email: string): Promise<{ checkout_url: string }> {
    return api.post<{ checkout_url: string }>(`/api/products/${productId}/checkout`, { email });
  },
};
