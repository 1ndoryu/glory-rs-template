/* wandori.us — Toolbar Commands
 * Comandos referenciados por app toolbars (Papelera, Finder, Projects). */

import { CommandRegistry, type CommandResult } from '../command-registry';
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

CommandRegistry.register({
  id: 'finder:new-folder',
  label: 'Nueva carpeta',
  icon: Folder,
  order: 52,
  contexts: ['toolbar'],
  undoPolicy: 'none',
  analyticsEvent: 'finder.new_folder',
  isAvailable: () => ({ state: 'enabled' }),
  execute: async (): Promise<CommandResult> => {
    const { createFolder } = await import('../workspace/workspace-store');
    createFolder('desktop', 'Nueva carpeta');
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'projects:new',
  label: 'Nuevo proyecto',
  icon: FolderCode,
  order: 53,
  contexts: ['toolbar'],
  undoPolicy: 'none',
  analyticsEvent: 'projects.new',
  isAvailable: () => ({ state: 'enabled' }),
  execute: async (): Promise<CommandResult> => {
    const { navigate } = await import('../../../router');
    navigate('/admin');
    return { status: 'success' };
  },
});
