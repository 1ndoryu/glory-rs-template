/* wandori.us — Default Release
 * Release estático que representa el layout actual del escritorio.
 * Este es el "release inmutable" v1 que se usa como base hasta que
 * el admin publique releases desde el backend.
 * [Plan 297A-11 §9.2] */

import type { WorkspaceTree } from './types';

/** Release v1 — refleja el layout hardcodeado actual del desktop. */
export const DEFAULT_RELEASE: WorkspaceTree = {
  version: 1,
  nodes: {
    /* === Fila superior izquierda === */
    gallery: {
      id: 'gallery',
      parentId: 'desktop',
      type: 'app',
      label: 'Galería',
      refId: 'finder',
      position: { col: 0, row: 0 },
      mobileOrder: 0,
      requires: 'public',
    },
    projects: {
      id: 'projects',
      parentId: 'desktop',
      type: 'app',
      label: 'Proyectos',
      refId: 'projects',
      position: { col: 0, row: 1 },
      mobileOrder: 1,
      requires: 'public',
    },
    profile: {
      id: 'profile',
      parentId: 'desktop',
      type: 'shortcut',
      label: 'Perfil',
      refId: 'shell-profile',
      position: { col: 0, row: 2 },
      mobileOrder: 2,
      requires: 'public',
    },
    about: {
      id: 'about',
      parentId: 'desktop',
      type: 'app',
      label: 'About',
      refId: 'about',
      position: { col: 0, row: 3 },
      mobileOrder: 3,
      requires: 'public',
    },
    snake: {
      id: 'snake',
      parentId: 'desktop',
      type: 'app',
      label: 'Snake',
      refId: 'snake',
      position: { col: 0, row: 4 },
      mobileOrder: 4,
      requires: 'public',
    },
    /* === Admin-only (added dynamically based on auth) === */
    settings: {
      id: 'settings',
      parentId: 'desktop',
      type: 'app',
      label: 'Configuración',
      refId: 'settings',
      position: { col: 0, row: 5 },
      mobileOrder: 5,
      requires: 'admin',
    },
    admin: {
      id: 'admin',
      parentId: 'desktop',
      type: 'app',
      label: 'Admin',
      refId: 'admin',
      position: { col: 0, row: 6 },
      mobileOrder: 6,
      requires: 'admin',
    },
  },
};
