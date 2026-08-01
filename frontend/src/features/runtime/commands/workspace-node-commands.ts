/* wandori.us — Workspace Node Commands
 * [018A-90] Comandos de gestión de nodos del workspace (abrir, renombrar,
 * eliminar) visibles en el menú contextual de carpetas ('folder') e iconos
 * ('icon'). Se separaron de workspace-commands.ts al superar el límite de
 * líneas del archivo: la gestión sobre una carpeta es un dominio propio. */

import { CommandRegistry, type CommandContext, type CommandResult } from '../command-registry';
import {
  tombstoneNode,
  tombstoneSubtree,
  renameNode,
  workspaceStore,
} from '../workspace/workspace-store';
import { showConfirm } from '../../../components/ui/confirm';
import { showPrompt } from '../../../components/ui/prompt';
import { resolveWorkspaceNodeId } from './workspace-commands';

/* === workspace:open — Abrir carpeta en el Finder ===
 * [018A-90] Si hay una ventana del Finder enfocada, la navega (evento
 * finder:navigate sobre su content, que el preview traduce a navigateTo y
 * sincroniza title/params vía onNavigate); si no, abre una ventana nueva
 * con la carpeta. */

CommandRegistry.register({
  id: 'workspace:open',
  label: 'Abrir',
  order: 20,
  contexts: ['folder'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.open_folder',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found' };
    const ws = workspaceStore.get();
    if (ws.nodes[nodeId]?.type !== 'folder') return { state: 'hidden', reason: 'not a folder' };
    return { state: 'enabled' };
  },
  execute: async (ctx?: CommandContext): Promise<CommandResult> => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    const ws = workspaceStore.get();
    const node = ws.nodes[nodeId];
    if (!node || node.type !== 'folder') return { status: 'failure', reason: 'not a folder' };

    const { windowStore } = await import('../window-manager');
    const focused = windowStore.get().find((w) => w.focused && w.appId === 'finder');
    if (focused?.content) {
      focused.content.dispatchEvent(new CustomEvent('finder:navigate', { detail: { folderId: nodeId } }));
      return { status: 'success' };
    }
    const { openAppWindow } = await import('../route-app-adapter');
    await openAppWindow('finder', { folderId: nodeId });
    return { status: 'success' };
  },
});

/* === workspace:rename — Renombrar nodo ===
 * [018A-90] Muta fieldOverrides.label vía renameNode. Usa window.prompt
 * (mismo mecanismo que workspace:rollback) por no existir aún un diálogo
 * de entrada con nombre en el sistema de UI. */

CommandRegistry.register({
  id: 'workspace:rename',
  label: 'Renombrar',
  order: 25,
  contexts: ['icon', 'folder'],
  undoPolicy: 'local',
  analyticsEvent: 'workspace.rename',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found in workspace' };
    return { state: 'enabled' };
  },
  execute: async (ctx?: CommandContext): Promise<CommandResult> => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    const ws = workspaceStore.get();
    const current = ws.nodes[nodeId]?.label ?? '';
    /* [018A-90] showPrompt en vez de window.prompt: el navegador integrado no
     * soporta prompt() y el diálogo propio mantiene la estética B&W del OS. */
    const label = await showPrompt('Nuevo nombre:', current);
    if (label === null) return { status: 'cancelled' };
    const trimmed = label.trim();
    if (!trimmed) return { status: 'failure', reason: 'nombre vacío' };
    if (trimmed === current) return { status: 'success' };
    renameNode(nodeId, trimmed);
    return { status: 'success' };
  },
});

/* === workspace:trash — Eliminar nodo ===
 * [018A-90] El menú contextual del Finder mandaba carpetas al contexto
 * 'folder' (que antes solo exponía acciones de creación), así que eliminar
 * solo existía en 'icon'. Ahora trash/copy/cut cubren ambos contextos.
 * Borrar una carpeta tumba su subárbol completo (tombstoneSubtree) y pide
 * confirmación; el borrado es restaurable desde la papelera (restoreNode
 * en cascada). */

CommandRegistry.register({
  id: 'workspace:trash',
  label: 'Eliminar',
  order: 30,
  contexts: ['icon', 'folder'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.trash',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found in workspace' };
    return { state: 'enabled' };
  },
  execute: async (ctx?: CommandContext): Promise<CommandResult> => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    const ws = workspaceStore.get();
    const node = ws.nodes[nodeId];
    if (node?.type === 'folder') {
      const confirmed = await showConfirm(`¿Eliminar la carpeta «${node.label}» y su contenido? Se podrá restaurar desde la papelera.`);
      if (!confirmed) return { status: 'cancelled' };
      tombstoneSubtree(nodeId);
      return { status: 'success' };
    }
    tombstoneNode(nodeId);
    return { status: 'success' };
  },
});
