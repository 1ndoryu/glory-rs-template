/* wandori.us — Workspace Reorder Commands
 * Ordenación común para launcher móvil y superficies de escritorio.
 * Solo modifica mobileOrder mediante la mutación de workspace compartida. */

import { ArrowDown, ArrowUp } from 'lucide';
import { CommandRegistry, type CommandContext, type CommandResult } from '../command-registry';
import { getChildren, reorderWorkspaceNodes, workspaceStore } from '../workspace/workspace-store';

function resolveTargetNode(ctx?: CommandContext): ReturnType<typeof getChildren>[number] | undefined {
  const targetId = ctx?.targets?.[0]?.id;
  if (!targetId) return undefined;
  const ws = workspaceStore.get();
  return Object.values(ws.nodes).find((node) => node.id === targetId || node.refId === targetId);
}

function reorderTarget(ctx: CommandContext | undefined, direction: -1 | 1): CommandResult {
  const node = resolveTargetNode(ctx);
  if (!node) return { status: 'failure', reason: 'node not found' };

  const siblings = getChildren(node.parentId ?? 'desktop');
  const currentIndex = siblings.findIndex((sibling) => sibling.id === node.id);
  const targetIndex = currentIndex + direction;
  if (currentIndex < 0 || targetIndex < 0 || targetIndex >= siblings.length) {
    return { status: 'failure', reason: 'already at boundary' };
  }

  const orderedIds = siblings.map((sibling) => sibling.id);
  [orderedIds[currentIndex], orderedIds[targetIndex]] = [orderedIds[targetIndex], orderedIds[currentIndex]];
  reorderWorkspaceNodes(orderedIds);
  return { status: 'success' };
}

CommandRegistry.register({
  id: 'workspace:move-up',
  label: 'Mover arriba',
  icon: ArrowUp,
  order: 44,
  contexts: ['icon'],
  undoPolicy: 'local',
  analyticsEvent: 'workspace.move_up',
  isAvailable: (ctx) => {
    const node = resolveTargetNode(ctx);
    if (!node) return { state: 'hidden', reason: 'node not found' };
    const siblings = getChildren(node.parentId ?? 'desktop');
    return siblings.findIndex((sibling) => sibling.id === node.id) > 0
      ? { state: 'enabled' }
      : { state: 'disabled', reason: 'ya está arriba' };
  },
  execute: (ctx) => reorderTarget(ctx, -1),
});

CommandRegistry.register({
  id: 'workspace:move-down',
  label: 'Mover abajo',
  icon: ArrowDown,
  order: 45,
  contexts: ['icon'],
  undoPolicy: 'local',
  analyticsEvent: 'workspace.move_down',
  isAvailable: (ctx) => {
    const node = resolveTargetNode(ctx);
    if (!node) return { state: 'hidden', reason: 'node not found' };
    const siblings = getChildren(node.parentId ?? 'desktop');
    const index = siblings.findIndex((sibling) => sibling.id === node.id);
    return index >= 0 && index < siblings.length - 1
      ? { state: 'enabled' }
      : { state: 'disabled', reason: 'ya está abajo' };
  },
  execute: (ctx) => reorderTarget(ctx, 1),
});
