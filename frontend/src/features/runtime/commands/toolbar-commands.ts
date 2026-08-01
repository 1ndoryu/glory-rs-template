/* wandori.us — Toolbar Commands
 * Comandos referenciados por app toolbars (Papelera, Finder, Projects). */

import { adminOnly, CommandRegistry, type CommandContext, type CommandResult } from '../command-registry';
import { Folder, Trash2, FolderCode } from 'lucide';

CommandRegistry.register({
  id: 'trash:restore-all',
  label: 'Restaurar todo',
  icon: Folder,
  order: 50,
  contexts: ['toolbar'],
  undoPolicy: 'none',
  analyticsEvent: 'trash.restore_all',
  isAvailable: () => ({ state: 'enabled' }),
  execute: async (): Promise<CommandResult> => {
    const { showConfirm } = await import('../../../components/ui/confirm');
    const ok = await showConfirm('¿Restaurar todos los elementos?');
    if (!ok) return { status: 'cancelled' };
    const { getTombstonedNodes, restoreNode } = await import('../workspace/workspace-store');
    for (const node of getTombstonedNodes()) restoreNode(node.id);
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'trash:empty',
  label: 'Vaciar papelera',
  icon: Trash2,
  order: 51,
  contexts: ['toolbar'],
  undoPolicy: 'none',
  analyticsEvent: 'trash.empty',
  isAvailable: () => ({ state: 'enabled' }),
  execute: async (): Promise<CommandResult> => {
    const { showConfirm } = await import('../../../components/ui/confirm');
    const ok = await showConfirm('¿Vaciar la papelera? Los elementos no se pueden recuperar.');
    if (!ok) return { status: 'cancelled' };
    const { resetOverlay } = await import('../workspace/workspace-store');
    resetOverlay();
    return { status: 'success' };
  },
});

/* [018A-88] finder:new-folder ahora responde al contexto del menú contextual
 * del Finder: si llega un target que es carpeta (fondo de carpeta o carpeta
 * del grid), crea DENTRO de esa carpeta. Sin target (toolbar de la ventana)
 * conserva el fallback histórico: crear en el escritorio. */
CommandRegistry.register({
  id: 'finder:new-folder',
  label: 'Nueva carpeta',
  icon: Folder,
  order: 52,
  contexts: ['toolbar', 'finder', 'folder'],
  undoPolicy: 'none',
  analyticsEvent: 'finder.new_folder',
  isAvailable: () => ({ state: 'enabled' }),
  execute: async (ctx?: CommandContext): Promise<CommandResult> => {
    const { createFolder } = await import('../workspace/workspace-store');
    const targetId = ctx?.targets?.[0]?.id;
    let parentId: string = 'desktop';
    if (targetId) {
      const ws = (await import('../workspace/workspace-store')).workspaceStore.get();
      const node = Object.values(ws.nodes).find((n) => n.id === targetId || n.refId === targetId);
      if (node?.type === 'folder') parentId = node.id;
    }
    createFolder(parentId, 'Nueva carpeta');
    return { status: 'success' };
  },
});

CommandRegistry.register(adminOnly({
  id: 'projects:new',
  label: 'Nuevo proyecto',
  icon: FolderCode,
  order: 53,
  /* [018A-88] Disponible también desde el menú contextual del Finder. */
  contexts: ['toolbar', 'finder', 'folder'],
  undoPolicy: 'none',
  analyticsEvent: 'projects.new',
  /* [018A-26] La creación vive en el programa interno, no en la ruta legacy
   * /admin. adminOnly mantiene el comando fuera de toolbars públicas. */
  isAvailable: () => ({ state: 'enabled' }),
  execute: async (): Promise<CommandResult> => {
    const { openAppWindow } = await import('../route-app-adapter');
    await openAppWindow('project-editor');
    return { status: 'success' };
  },
}));
