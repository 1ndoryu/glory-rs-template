/* wandori.us — Workspace Service
 * Capa de servicio para el workspace (releases, draft, overlay).
 * [Auditoría v4 §4.2] — Separa flatten (puro) de publish (HTTP).
 * [Plan §9.2] — Preparación para 297A-11 draft/release y 297A-13 overlay remoto.
 * [018A-35] El transporte es generado; las conversiones conservan el modelo
 * rico del runtime sin exponerlo al contrato HTTP. */

import { unwrapGeneratedResponse } from '../api/client';
import {
  getActiveRelease,
  getReleaseByVersion,
  listReleases,
  publishRelease,
} from '../api/generated/workspace-handler/workspace-handler';
import {
  getOverlay,
  updateOverlay,
} from '../api/generated/workspace-overlay-handler/workspace-overlay-handler';
import type { WorkspaceOverlay, WorkspaceTree } from '../features/runtime/workspace/types';

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

export interface WorkspaceOverlayResponse {
  overlay: WorkspaceOverlay;
  revision: number;
  updated_at: string;
}

export interface UpdateWorkspaceOverlayRequest {
  overlay: WorkspaceOverlay;
  expected_revision: number;
}

function toReleaseInfo(data: {
  version: number;
  tree: Record<string, unknown>;
  published_at: string;
}): ReleaseInfo {
  return {
    version: data.version,
    published_at: data.published_at,
    tree: data.tree as unknown as WorkspaceTree,
  };
}

function toOverlayResponse(data: {
  overlay: Record<string, unknown>;
  revision: number;
  updated_at: string;
}): WorkspaceOverlayResponse {
  return {
    overlay: data.overlay as unknown as WorkspaceOverlay,
    revision: data.revision,
    updated_at: data.updated_at,
  };
}

export const WorkspaceService = {
  /** Obtener el release activo (público). */
  async getActiveRelease(): Promise<ReleaseInfo | null> {
    const response = await getActiveRelease();
    if (response.status === 404) return null;
    return toReleaseInfo(unwrapGeneratedResponse(response, [200]));
  },

  /** Obtener un release por versión (admin). */
  async getReleaseByVersion(version: number): Promise<ReleaseInfo> {
    const response = await getReleaseByVersion(version);
    return toReleaseInfo(unwrapGeneratedResponse(response, [200]));
  },

  /** Listar historial de releases (admin). */
  async listReleases(): Promise<ReleaseListItem[]> {
    const response = await listReleases();
    const result = unwrapGeneratedResponse<{ items: Array<{
      id: string;
      version: number;
      published_at: string;
      published_by?: string | null;
    }> }>(response, [200]);
    return result.items.map((item) => ({
      ...item,
      published_by: item.published_by ?? null,
    }));
  },

  /** Publicar un nuevo release (admin). */
  async publish(tree: WorkspaceTree): Promise<ReleaseInfo> {
    const response = await publishRelease({ tree: tree as unknown as Record<string, unknown> });
    return toReleaseInfo(unwrapGeneratedResponse(response, [201]));
  },

  /** Guardar/actualizar el overlay remoto con revisión optimista. */
  async saveOverlay(request: UpdateWorkspaceOverlayRequest): Promise<WorkspaceOverlayResponse> {
    const response = await updateOverlay(request);
    return toOverlayResponse(unwrapGeneratedResponse(response, [200]));
  },

  /** Obtener el overlay remoto de la cuenta autenticada. */
  async getOverlay(): Promise<WorkspaceOverlayResponse> {
    const response = await getOverlay();
    return toOverlayResponse(unwrapGeneratedResponse(response, [200]));
  },
};
