/* wandori.us — Default Release
 * Release estático que representa el layout actual del escritorio.
 * Solo incluye nodos públicos. Los nodos admin se añaden dinámicamente
 * en stores.ts cuando el usuario está autenticado.
 * [Auditoría v4 §5.4] Nodos admin separados del bundle público. */

import type { WorkspaceNode, WorkspaceTree } from './types';

/** Release v1 — solo nodos públicos. */
export const DEFAULT_RELEASE: WorkspaceTree = {
  version: 1,
  nodes: {
    gallery: {
      id: 'gallery', parentId: 'desktop', type: 'folder', label: 'Galería',
      position: { col: 0, row: 0 }, mobilePosition: { col: 0, row: 0 }, mobileOrder: 0, requires: 'public',
    },
    projects: {
      id: 'projects', parentId: 'desktop', type: 'app', label: 'Proyectos', refId: 'projects',
      position: { col: 0, row: 1 }, mobilePosition: { col: 1, row: 0 }, mobileOrder: 1, requires: 'public',
    },
    profile: {
      id: 'profile', parentId: 'desktop', type: 'shortcut', label: 'Perfil', refId: 'shell-profile',
      position: { col: 0, row: 2 }, mobilePosition: { col: 2, row: 0 }, mobileOrder: 2, requires: 'public',
    },
    about: {
      id: 'about', parentId: 'desktop', type: 'app', label: 'About', refId: 'about',
      position: { col: 0, row: 3 }, mobilePosition: { col: 0, row: 1 }, mobileOrder: 3, requires: 'public',
    },
    trash: {
      id: 'trash', parentId: 'desktop', type: 'app', label: 'Papelera', refId: 'trash',
      position: { col: 1, row: 0 }, mobilePosition: { col: 1, row: 1 }, mobileOrder: 6, requires: 'public',
    },
  },
};

/** Nodos exclusivos de admin — NO incluidos en DEFAULT_RELEASE.
 *  Se añaden dinámicamente en stores.ts cuando capability = 'admin'. */
export const ADMIN_NODES: Record<string, WorkspaceNode> = {
  settings: {
    id: 'settings', parentId: 'desktop', type: 'app', label: 'Configuración', refId: 'settings',
    position: { col: 0, row: 4 }, mobilePosition: { col: 2, row: 1 }, mobileOrder: 4, requires: 'admin',
  },
  admin: {
    id: 'admin', parentId: 'desktop', type: 'app', label: 'Admin', refId: 'admin',
    position: { col: 0, row: 5 }, mobilePosition: { col: 0, row: 2 }, mobileOrder: 5, requires: 'admin',
  },
  mediaLibrary: {
    id: 'mediaLibrary', parentId: 'desktop', type: 'app', label: 'Biblioteca de media', refId: 'media-library',
    position: { col: 1, row: 1 }, mobilePosition: { col: 2, row: 2 }, mobileOrder: 7, requires: 'admin',
  },
  analytics: {
    id: 'analytics', parentId: 'desktop', type: 'app', label: 'Estadísticas', refId: 'analytics',
    position: { col: 1, row: 2 }, mobilePosition: { col: 0, row: 3 }, mobileOrder: 8, requires: 'admin',
  },
};
