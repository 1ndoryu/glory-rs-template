/* wandori.us — Workspace Commands
 * Comandos de workspace: trash, restore, reset, publish, clipboard, create-folder. */

import { CommandRegistry, type CommandContext, type CommandResult } from '../command-registry';
import {
  tombstoneNode,
  restoreNode,
  resetOverlay,
  workspaceStore,
  publishWorkspace,
  setClipboard,
  getClipboard,
  pasteFromClipboard,
  createFolder,
} from '../workspace/workspace-store';

function resolveWorkspaceNodeId(targetId: string): string | undefined {
  const ws = workspaceStore.get();
  const node = Object.values(ws.nodes).find(
    (n) => n.id === targetId || n.refId === targetId,
  );
  return node?.id;
}

/* === Trash / Restore / Reset === */

CommandRegistry.register({
  id: 'workspace:trash',
  label: 'Eliminar',
  order: 30,
  contexts: ['icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.trash',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found in workspace' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    tombstoneNode(nodeId);
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:restore',
  label: 'Restaurar',
  order: 31,
  contexts: ['trash'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.restore',
  isAvailable: (ctx) => {
    const nodeId = ctx.targets?.[0]?.id;
    if (!nodeId) return { state: 'hidden', reason: 'no target' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const nodeId = ctx?.targets?.[0]?.id;
    if (!nodeId) return { status: 'failure', reason: 'no target' };
    restoreNode(nodeId);
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:reset',
  label: 'Restablecer escritorio',
  order: 32,
  contexts: ['desktop'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.reset',
  isAvailable: () => ({ state: 'enabled' }),
  execute: (): CommandResult => {
    resetOverlay();
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:publish',
  label: 'Publicar escritorio',
  order: 33,
  contexts: ['desktop'],
  requires: 'admin',
  undoPolicy: 'none',
  analyticsEvent: 'workspace.published',
  isAvailable: (ctx) => {
    if (ctx.capability !== 'admin') return { state: 'hidden' };
    return { state: 'enabled' };
  },
  execute: async (): Promise<CommandResult> => {
    const result = await publishWorkspace();
    if (result) return { status: 'success' };
    return { status: 'failure', reason: 'Error al publicar' };
  },
});

/* === Clipboard === */

CommandRegistry.register({
  id: 'workspace:copy',
  label: 'Copiar',
  shortcut: 'ctrl+c',
  order: 40,
  contexts: ['icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.copy',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    setClipboard([nodeId], 'copy');
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:cut',
  label: 'Cortar',
  shortcut: 'ctrl+x',
  order: 41,
  contexts: ['icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.cut',
  isAvailable: (ctx) => {
    const targetId = ctx.targets?.[0]?.id;
    if (!targetId) return { state: 'hidden', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { state: 'hidden', reason: 'node not found' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    if (!targetId) return { status: 'failure', reason: 'no target' };
    const nodeId = resolveWorkspaceNodeId(targetId);
    if (!nodeId) return { status: 'failure', reason: 'node not found' };
    setClipboard([nodeId], 'cut');
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:paste',
  label: 'Pegar',
  shortcut: 'ctrl+v',
  order: 42,
  contexts: ['desktop', 'folder', 'icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.paste',
  isAvailable: () => {
    const clip = getClipboard();
    if (!clip || clip.nodeIds.length === 0) return { state: 'disabled', reason: 'clipboard vacío' };
    return { state: 'enabled' };
  },
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    let parentId: string = 'desktop';
    if (targetId) {
      const ws = workspaceStore.get();
      const nodeId = resolveWorkspaceNodeId(targetId);
      if (nodeId && ws.nodes[nodeId]?.type === 'folder') {
        parentId = nodeId;
      }
    }
    const pasted = pasteFromClipboard(parentId);
    if (pasted.length === 0) return { status: 'failure', reason: 'no se pudo pegar (ciclo o destino inválido)' };
    return { status: 'success' };
  },
});

CommandRegistry.register({
  id: 'workspace:create-folder',
  label: 'Nueva carpeta',
  order: 43,
  contexts: ['desktop', 'icon'],
  undoPolicy: 'none',
  analyticsEvent: 'workspace.create_folder',
  isAvailable: () => ({ state: 'enabled' }),
  execute: (ctx?: CommandContext): CommandResult => {
    const targetId = ctx?.targets?.[0]?.id;
    let parentId: string = 'desktop';
    if (targetId) {
      const ws = workspaceStore.get();
      const nodeId = resolveWorkspaceNodeId(targetId);
      if (nodeId && ws.nodes[nodeId]?.type === 'folder') {
        parentId = nodeId;
      }
    }
    createFolder(parentId, 'Nueva carpeta');
    return { status: 'success' };
  },
});
