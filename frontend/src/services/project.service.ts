/* wandori.us — Project Service
 * Capa de servicio para operaciones con proyectos.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.get/post en pages/projects.ts y admin-projects.ts. */

import { api } from '../api/client';
import type { Project, CreateProjectRequest } from '../api/types';

export const ProjectService = {
  /** Listar proyectos públicos (visibles). */
  async list(): Promise<Project[]> {
    return api.get<Project[]>('/api/projects');
  },

  /** Listar todos los proyectos (admin). */
  async listAll(): Promise<Project[]> {
    return api.get<Project[]>('/admin/projects');
  },

  /** Obtener un proyecto por ID. */
  async getById(id: string): Promise<Project> {
    return api.get<Project>(`/admin/projects/${id}`);
  },

  /** Crear un nuevo proyecto (admin). */
  async create(data: CreateProjectRequest): Promise<Project> {
    return api.post<Project>('/admin/projects', data);
  },

  /** Actualizar un proyecto (admin). */
  async update(id: string, data: Partial<CreateProjectRequest>): Promise<Project> {
    return api.put<Project>(`/admin/projects/${id}`, data);
  },

  /** Eliminar un proyecto (admin). */
  async delete(id: string): Promise<void> {
    return api.delete<void>(`/admin/projects/${id}`);
  },

  /** Reordenar proyectos (admin). */
  async reorder(ids: string[]): Promise<void> {
    return api.post<void>('/admin/projects/reorder', { ids });
  },
};
