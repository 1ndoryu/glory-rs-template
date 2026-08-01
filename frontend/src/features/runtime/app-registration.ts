/* wandori.us — App Registration
 * Registra todas las apps del OS en el AppRegistry.
 * Cada app define su id, título, icono, capacidades y render function.
 * Las apps solo devuelven contenido; el shell crea la ventana. */

import { FileUser, Folder, Settings, FileText, FolderCode, Trash2, ShieldUser, UserRound, ShoppingBag, FolderOpen } from 'lucide';
import { createEl } from '../../utils/dom';
import { AppRegistry } from './app-registry';
import { createPathDeepLink } from './deep-links';
import { createFinderPreview } from '../desktop/apps/finder/finder-preview';
import { createReaderPreview, type ReaderOptions } from '../desktop/apps/reader/reader-preview';
import { createTrashPreview } from '../desktop/apps/trash/trash-preview';
import { dispatchEvent } from '../analytics/dispatcher';
import type { MountedView, RenderContext } from '../../core/lifecycle';
import { SettingsService } from '../../services';
import { appendSanitizedHtml } from '../../utils/sanitize-html';
import { mountAccountView } from './account-view';

/* === Finder === */
AppRegistry.register({
  id: 'finder',
  title: 'Galería',
  icon: Folder,
  iconType: 'folder',
  singleton: false,
  requires: 'public',
  routePatterns: ['/gallery'],
  deepLink: createPathDeepLink('/gallery'),
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
        const windowEl = content.closest('.desktop-window');
        if (!windowEl) return;
        const titleEl = windowEl.querySelector('.desktop-window__title');
        if (titleEl) titleEl.textContent = label;
      },
    });

    return {
      element: content,
      destroy: () => { dispatchEvent({ type: 'app_closed', appId: 'finder' }); },
    };
  },
});

/* === Reader === */
AppRegistry.register({
  id: 'reader',
  title: 'Documento',
  icon: FileText,
  iconType: 'document',
  singleton: false,
  requires: 'public',
  routePatterns: ['/article/:slug'],
  deepLink: createPathDeepLink('/article/:slug', ['slug']),
  layout: 'full-bleed',
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'reader' });

    /* `slug` es el único identificador público permitido por el deep link.
     * `resourceId` pertenece al workspace y no puede convertirse implícitamente
     * en slug: resolverlo requerirá un envelope público autorizado. */
    const opts: ReaderOptions = {
      slug: ctx.params?.slug,
      title: ctx.params?.title,
    };
    const content = createReaderPreview(opts);

    return {
      element: content,
      destroy: () => { dispatchEvent({ type: 'app_closed', appId: 'reader' }); },
    };
  },
});

/* === Cuenta === */
AppRegistry.register({
  id: 'account',
  title: 'Cuenta',
  icon: UserRound,
  iconType: 'application',
  singleton: true,
  requires: 'public',
  routePatterns: ['/login'],
  deepLink: createPathDeepLink('/login'),
  layout: 'padded',
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'account' });
    const view = mountAccountView(ctx);
    return {
      element: view.element,
      destroy: () => {
        view.destroy?.();
        dispatchEvent({ type: 'app_closed', appId: 'account' });
      },
    };
  },
});

/* === Settings === */
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

/* === About === */
AppRegistry.register({
  id: 'about',
  title: 'About',
  icon: FileUser,
  iconType: 'document',
  singleton: true,
  requires: 'public',
  routePatterns: ['/about'],
  deepLink: createPathDeepLink('/about'),
  layout: 'full-bleed',
  render: (ctx: RenderContext): MountedView => {
    dispatchEvent({ type: 'app_opened', appId: 'about' });

    const container = createEl('article', { className: 'desktop-about' });

    void (async () => {
      try {
        if (ctx.signal.aborted) return;
        const settings = await SettingsService.getAll();
        if (ctx.signal.aborted) return;
        const content = settings.about_content || '';
        if (content) {
          appendSanitizedHtml(container, content);
        } else {
          container.appendChild(createEl('p', { textContent: 'Contenido about no configurado.' }));
        }
      } catch {
        container.appendChild(createEl('p', { textContent: 'Error al cargar contenido about.' }));
      }
    })();

    return {
      element: container,
      destroy: () => { dispatchEvent({ type: 'app_closed', appId: 'about' }); },
    };
  },
});

/* === Trash === */
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
      destroy: () => { dispatchEvent({ type: 'app_closed', appId: 'trash' }); },
    };
  },
});

/* === Admin === */
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
      /* [317A-2] Contenedor con fill-height: los estados vacios centrados ocupan toda la ventana. */
      const container = createEl('div', { className: 'app-contenedor' });
      let adminPage: HTMLElement | null = null;
      let disposed = false;

      void m.renderAdmin()
        .then(el => {
          if (disposed || ctx.signal.aborted) {
            m.disposeAdminPage(el);
            return;
          }
          adminPage = el;
          container.appendChild(el);
        })
        .catch(() => {
          if (!disposed && !ctx.signal.aborted) {
            container.textContent = 'Error al cargar Admin.';
          }
        });

      return {
        element: container,
        destroy: () => {
          disposed = true;
          if (adminPage) m.disposeAdminPage(adminPage);
          dispatchEvent({ type: 'app_closed', appId: 'admin' });
        },
      };
    },
  })),
});

/* === Article Editor — programa editorial admin === */
AppRegistry.registerLazy({
  id: 'article-editor',
  title: 'Editor de artículos',
  icon: FileText,
  iconType: 'document',
  singleton: false,
  requires: 'admin',
  layout: 'padded',
  load: () => import('../desktop/apps/article-editor/article-editor').then(m => ({
    render: (ctx: RenderContext): MountedView => m.renderArticleEditor(ctx),
  })),
});

/* === Project Editor — programa editorial admin === */
AppRegistry.registerLazy({
  id: 'project-editor',
  title: 'Editor de proyectos',
  icon: FolderCode,
  iconType: 'folder',
  singleton: false,
  requires: 'admin',
  layout: 'padded',
  load: () => import('../desktop/apps/project-editor/project-editor').then(m => ({
    render: (ctx: RenderContext): MountedView => m.renderProjectEditor(ctx),
  })),
});

/* === Product Editor — programa editorial admin === */
AppRegistry.registerLazy({
  id: 'product-editor',
  title: 'Editor de productos',
  icon: ShoppingBag,
  iconType: 'application',
  singleton: false,
  requires: 'admin',
  layout: 'padded',
  load: () => import('../desktop/apps/product-editor/product-editor').then(m => ({
    render: (ctx: RenderContext): MountedView => m.renderProductEditor(ctx),
  })),
});

/* === Media Library — biblioteca de media admin === */
AppRegistry.registerLazy({
  id: 'media-library',
  title: 'Biblioteca de media',
  icon: FolderOpen,
  iconType: 'folder',
  singleton: true,
  requires: 'admin',
  layout: 'padded',
  load: () => import('../desktop/apps/media-library/media-library').then(m => ({
    render: (ctx: RenderContext): MountedView => {
      dispatchEvent({ type: 'app_opened', appId: 'media-library' });
      const view = m.createMediaLibraryPreview({ signal: ctx.signal });
      return {
        element: view.element,
        destroy: () => {
          view.destroy();
          dispatchEvent({ type: 'app_closed', appId: 'media-library' });
        },
      };
    },
  })),
});

/* === Projects === */
AppRegistry.registerLazy({
  id: 'projects',
  title: 'Proyectos',
  icon: FolderCode,
  iconType: 'folder',
  singleton: true,
  requires: 'public',
  routePatterns: ['/projects'],
  deepLink: createPathDeepLink('/projects'),
  toolbar: [
    { label: 'Archivo', items: ['projects:new'] },
  ],
  load: () => import('../../pages/projects').then(m => ({
    render: (ctx: RenderContext): MountedView => {
      dispatchEvent({ type: 'app_opened', appId: 'projects' });
      /* [317A-2] Contenedor con fill-height: los estados vacios centrados ocupan toda la ventana. */
      const container = createEl('div', { className: 'app-contenedor' });
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
