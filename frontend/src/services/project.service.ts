/* wandori.us — Project Service
 * Capa de servicio para operaciones con proyectos.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.get/post en pages/projects.ts y admin-projects.ts.
 * [018A-33] La adaptación de url traduce el parche semántico del contrato OpenAPI. */

import { unwrapGeneratedResponse } from '../api/client';
import {
  createProject,
  deleteProject,
  getProject,
  listAllProjects,
  listProjects,
  updateProject,
} from '../api/generated/projects-handler/projects-handler';
import type { Project, CreateProjectRequest, UpdateProjectRequest } from '../api/types';

export const ProjectService = {
  /** Listar proyectos públicos (visibles). */
  async list(): Promise<Project[]> {
    const response = await listProjects();
    return unwrapGeneratedResponse<Project[]>(response, [200]);
  },

  /** Listar todos los proyectos (admin). */
  async listAll(): Promise<Project[]> {
    const response = await listAllProjects();
    return unwrapGeneratedResponse<Project[]>(response, [200]);
  },

  /** Obtener un proyecto por ID; opcionalmente abortable con el lifecycle. */
  async getById(id: string, options?: { signal?: AbortSignal }): Promise<Project> {
    const response = await getProject(id, options);
    return unwrapGeneratedResponse<Project>(response, [200]);
  },

  /** Crear un nuevo proyecto (admin). */
  async create(data: CreateProjectRequest): Promise<Project> {
    const response = await createProject(data);
    return unwrapGeneratedResponse<Project>(response, [201]);
  },

  /** Actualizar un proyecto (admin). */
  async update(id: string, data: UpdateProjectRequest): Promise<Project> {
    const response = await updateProject(id, {
      ...data,
      url: data.url === undefined
        ? undefined
        : data.url === null
          ? 'Clear'
          : { Set: data.url },
    });
    return unwrapGeneratedResponse<Project>(response, [200]);
  },

  /** Eliminar un proyecto (admin). */
  async delete(id: string): Promise<void> {
    const response = await deleteProject(id);
    unwrapGeneratedResponse<void>(response, [204]);
  },

};
