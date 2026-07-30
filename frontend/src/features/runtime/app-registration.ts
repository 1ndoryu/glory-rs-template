/* wandori.us — App Registration
 * Registra todas las apps del OS en el AppRegistry.
 * Cada app define su id, título, icono, capacidades y render function.
 * Las apps solo devuelven contenido; el shell crea la ventana. */

import { Folder, FileUser, Settings, FileText, FolderCode, Trash2 } from 'lucide';
import { AppRegistry } from './app-registry';
import { createFinderPreview } from '../desktop/apps/finder/finder-preview';
import { createReaderPreview } from '../desktop/apps/reader/reader-preview';
import { createFontPanel } from '../settings/font-panel';
import { createTrashPreview } from '../desktop/apps/trash/trash-preview';
import { dispatchEvent } from '../analytics/dispatcher';
import type { MountedView, RenderContext } from '../../core/lifecycle';

/* === Finder — Galería de imágenes y documentos === */
AppRegistry.register({
  id: 'finder',
  title: 'Galería',
  icon: Folder,
  iconType: 'folder',
  singleton: true,
  requires: 'public',
  routePatterns: ['/gallery'],
  layout: 'full-bleed',
  render: (_ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'finder' });

    const content = createFinderPreview({
      onOpenArticle: (title: string) => {
        const slug = title.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
        import('../../router').then(r => r.navigate(`/article/${slug}`));
      },
    });

    return {
      element: content,
      destroy: () => {
        dispatchEvent({ type: 'app_closed', appId: 'finder' });
      },
    };
  },
});

/* === Reader — Lector de artículos === */
AppRegistry.register({
  id: 'reader',
  title: 'Documento',
  icon: FileText,
  iconType: 'document',
  singleton: false,
  requires: 'public',
  routePatterns: ['/article/:slug'],
  layout: 'full-bleed',
  render: (_ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'reader' });

    /* El contenido real se cargará desde la API en fases posteriores.
     * Por ahora usa el preview existente. */
    const content = createReaderPreview({ title: 'Documento' });

    return {
      element: content,
      destroy: () => {
        dispatchEvent({ type: 'app_closed', appId: 'reader' });
      },
    };
  },
});

/* === Settings — Configuración de fuentes y perfil === */
AppRegistry.register({
  id: 'settings',
  title: 'Configuración',
  icon: Settings,
  iconType: 'application',
  singleton: true,
  requires: 'admin',
  render: (_ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'settings' });

    const content = createFontPanel();

    return {
      element: content,
      destroy: () => {
        dispatchEvent({ type: 'app_closed', appId: 'settings' });
      },
    };
  },
});

/* === About — Página about === */
AppRegistry.register({
  id: 'about',
  title: 'About',
  icon: FileUser,
  iconType: 'document',
  singleton: true,
  requires: 'public',
  routePatterns: ['/about'],
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'about' });

    const container = document.createElement('div');
    void import('../../pages/about').then(async m => {
      if (ctx.signal.aborted) return;
      container.appendChild(await m.renderAbout());
    });

    return {
      element: container,
      destroy: () => {
        dispatchEvent({ type: 'app_closed', appId: 'about' });
      },
    };
  },
});

/* === Trash (Papelera) — Papelera del workspace === */
AppRegistry.register({
  id: 'trash',
  title: 'Papelera',
  icon: Trash2,
  iconType: 'application',
  singleton: true,
  requires: 'public',
  render: (_ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'trash' });

    const content = createTrashPreview();

    return {
      element: content,
      destroy: () => {
        dispatchEvent({ type: 'app_closed', appId: 'trash' });
      },
    };
  },
});

/* === Projects — Página de proyectos === */
AppRegistry.register({
  id: 'projects',
  title: 'Proyectos',
  icon: FolderCode,
  iconType: 'folder',
  singleton: true,
  requires: 'public',
  routePatterns: ['/projects'],
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'projects' });

    const container = document.createElement('div');
    void import('../../pages/projects').then(async m => {
      if (ctx.signal.aborted) return;
      container.appendChild(await m.renderProjects());
    });

    return {
      element: container,
      destroy: () => {
        dispatchEvent({ type: 'app_closed', appId: 'projects' });
      },
    };
  },
});
