/* wandori.us — App Registration
 * Registra todas las apps del OS en el AppRegistry.
 * Cada app define su id, título, icono, capacidades y render function.
 * Las apps solo devuelven contenido; el shell crea la ventana. */

import { FileUser, Folder, Settings, FileText, FolderCode, Trash2, ShieldUser } from 'lucide';
import { AppRegistry } from './app-registry';
import { createFinderPreview } from '../desktop/apps/finder/finder-preview';
import { createReaderPreview, type ReaderOptions } from '../desktop/apps/reader/reader-preview';
import { createTrashPreview } from '../desktop/apps/trash/trash-preview';
import { dispatchEvent } from '../analytics/dispatcher';
import type { MountedView, RenderContext } from '../../core/lifecycle';
import { SettingsService } from '../../services';
import { appendSanitizedHtml } from '../../utils/sanitize-html';

/* === Finder — Explorador de archivos del OS === */
AppRegistry.register({
  id: 'finder',
  title: 'Galería',
  icon: Folder,
  iconType: 'folder',
  singleton: false,
  requires: 'public',
  routePatterns: ['/gallery'],
  layout: 'full-bleed',
  toolbar: [
    { label: 'Archivo', items: ['finder:new-folder'] },
  ],
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'finder' });

    const folderId = ctx.params?.folderId ?? 'desktop';

    const content = createFinderPreview({
      folderId,
      onOpenApp: (appId: string, params?: Record<string, string>) => {
        void import('./route-app-adapter').then(m => m.openAppWindow(appId, params));
      },
      onCreateFolder: () => {
        void import('../runtime/command-registry').then(({ CommandRegistry }) => {
          void CommandRegistry.execute('finder:new-folder');
        });
      },
      onNavigate: (_folderId: string, label: string) => {
        /* Actualizar título de la ventana cuando Finder navega entre carpetas.
         * DOM traversal desde content para encontrar el título de SU ventana.
         * Null guard: content aún no está en el DOM durante el montaje inicial. */
        const windowEl = content.closest('.desktop-window');
        if (!windowEl) return;
        const titleEl = windowEl.querySelector('.desktop-window__title');
        if (titleEl) titleEl.textContent = label;
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
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'reader' });

    const opts: ReaderOptions = {
      slug: ctx.params?.slug ?? ctx.params?.resourceId,
      title: ctx.params?.title,
    };
    const content = createReaderPreview(opts);

    return {
      element: content,
      destroy: () => {
        dispatchEvent({ type: 'app_closed', appId: 'reader' });
      },
    };
  },
});

/* === Settings — Configuración de fuentes y perfil ===
 * [Auditoría v3 §2.5] Lazy loading: código se carga bajo demanda. */
AppRegistry.registerLazy({
  id: 'settings',
  title: 'Configuración',
  icon: Settings,
  iconType: 'application',
  singleton: true,
  requires: 'admin',
  load: () => import('../settings/font-panel').then(m => ({
    render: (_ctx: RenderContext): MountedView => {
      dispatchEvent({ type: 'app_opened', appId: 'settings' });
      return {
        element: m.createFontPanel(),
        destroy: () => { dispatchEvent({ type: 'app_closed', appId: 'settings' }); },
      };
    },
  })),
});

/* === About — Página about ===
 * Carga contenido desde la API directamente, sin importar la página legacy.
 * [Auditoría v2] App autónoma del workspace. */
AppRegistry.register({
  id: 'about',
  title: 'About',
  icon: FileUser,
  iconType: 'document',
  singleton: true,
  requires: 'public',
  routePatterns: ['/about'],
  layout: 'full-bleed',
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'about' });

    const container = document.createElement('article');
    container.className = 'desktop-about';

    /* Cargar contenido desde la API */
    void (async () => {
      try {
        if (ctx.signal.aborted) return;
        const settings = await SettingsService.getAll();
        if (ctx.signal.aborted) return;
        const content = settings.about_content || '';
        if (content) {
          appendSanitizedHtml(container, content);
        } else {
          const p = document.createElement('p');
          p.textContent = 'Contenido about no configurado.';
          container.appendChild(p);
        }
      } catch {
        const p = document.createElement('p');
        p.textContent = 'Error al cargar contenido about.';
        container.appendChild(p);
      }
    })();

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
  toolbar: [
    { label: 'Archivo', items: ['trash:restore-all', '---', 'trash:empty'] },
  ],
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

/* === Admin — Panel de administración ===
 * [Auditoría v3 §2.5] Lazy loading. */
AppRegistry.registerLazy({
  id: 'admin',
  title: 'Admin',
  icon: ShieldUser,
  iconType: 'application',
  singleton: true,
  requires: 'admin',
  routePatterns: ['/admin'],
  load: () => import('../../pages/admin').then(m => ({
    render: (ctx: RenderContext): MountedView => {
      dispatchEvent({ type: 'app_opened', appId: 'admin' });
      const container = document.createElement('div');
      void m.renderAdmin().then(el => {
        if (!ctx.signal.aborted) container.appendChild(el);
      });
      return {
        element: container,
        destroy: () => { dispatchEvent({ type: 'app_closed', appId: 'admin' }); },
      };
    },
  })),
});

/* === Projects — Página de proyectos ===
 * [Auditoría v3 §2.5] Lazy loading. */
AppRegistry.registerLazy({
  id: 'projects',
  title: 'Proyectos',
  icon: FolderCode,
  iconType: 'folder',
  singleton: true,
  requires: 'public',
  routePatterns: ['/projects'],
  toolbar: [
    { label: 'Archivo', items: ['projects:new'] },
  ],
  load: () => import('../../pages/projects').then(m => ({
    render: (ctx: RenderContext): MountedView => {
      dispatchEvent({ type: 'app_opened', appId: 'projects' });
      const container = document.createElement('div');
      void m.renderProjects().then(el => {
        if (!ctx.signal.aborted) container.appendChild(el);
      });
      return {
        element: container,
        destroy: () => { dispatchEvent({ type: 'app_closed', appId: 'projects' }); },
      };
    },
  })),
});
