/* wandori.us — Workspace Service
 * Capa de servicio para el workspace (releases, draft, overlay).
 * [Auditoría v4 §4.2] — Separa flatten (puro) de publish (HTTP).
 * [Plan §9.2] — Preparación para 297A-11 draft/release y 297A-13 overlay remoto. */

import { api } from '../api/client';
import type { WorkspaceTree } from '../features/runtime/workspace/types';

export interface ReleaseInfo {
  version: number;
  tree: WorkspaceTree;
  published_at: string;
}

export interface ReleaseListItem {
  id: string;
  version: number;
  published_at: string;
  published_by: string | null;
}

export const WorkspaceService = {
  /** Obtener el release activo (público). */
  async getActiveRelease(): Promise<ReleaseInfo | null> {
    try {
      return await api.get<ReleaseInfo>('/api/workspace/release');
    } catch {
      return null;
    }
  },

  /** Obtener un release por versión. */
  async getReleaseByVersion(version: number): Promise<ReleaseInfo> {
    return api.get<ReleaseInfo>(`/api/workspace/release/${version}`);
  },

  /** Listar historial de releases (admin). */
  async listReleases(): Promise<ReleaseListItem[]> {
    const res = await api.get<{ items: ReleaseListItem[] }>('/admin/workspace/releases');
    return res.items;
  },

  /** Publicar un nuevo release (admin). */
  async publish(tree: WorkspaceTree): Promise<ReleaseInfo> {
    return api.post<ReleaseInfo>('/admin/workspace/publish', { tree });
  },

  /** Guardar/actualizar el overlay remoto (usuario autenticado).
   *  [297A-13] Endpoint futuro. */
  async saveOverlay(overlay: unknown): Promise<void> {
    return api.post<void>('/api/workspace/overlay', { overlay });
  },

  /** Obtener el overlay remoto (usuario autenticado).
   *  [297A-13] Endpoint futuro. */
  async getOverlay(): Promise<unknown | null> {
    try {
      return await api.get<unknown>('/api/workspace/overlay');
    } catch {
      return null;
    }
  },
};
