/* [058A-4] Lógica de colocación de un grupo de iconos en el snap-grid.
 * Extraída de workspace-icon-grid.ts (límite de líneas de componente, regla
 * de util máx 150). El grupo mantiene el offset relativo de cada seleccionado
 * respecto al icono arrastrado (delta del plan); sin resolución de colisiones
 * del grupo contra otros iconos (Windows tampoco al arrastrar un grupo sobre
 * una rejilla snap; mejora futura documentada). Los nodos sin position
 * (auto-flow) no participan del movimiento. */

import type { PlacementPlan } from './icon-grid';
import type { NodeId } from '../../runtime/workspace/types';
import type { ResolvedNode } from '../../runtime/workspace/types';

export interface GroupMove {
  nodeId: NodeId;
  position: { col: number; row: number };
}

/** Calcula los moves de colocación del grupo arrastrado: el move del icono
 * arrastrado (tomado del plan) más el delta aplicado a cada seleccionado con
 * position. Devuelve null si el arrastrado no tiene move/position válidos (el
 * caller cae al comportamiento de icono único). */
export function buildGroupPlacementMoves(
  desktopNodes: readonly ResolvedNode[],
  draggedId: NodeId,
  plan: PlacementPlan,
  selectedIds: readonly NodeId[],
): GroupMove[] | null {
  const draggedMove = plan.moves.find(m => m.nodeId === draggedId);
  const draggedNode = desktopNodes.find(n => n.id === draggedId);
  if (!draggedMove || !draggedNode?.position) return null;

  const dCol = draggedMove.position.col - draggedNode.position.col;
  const dRow = draggedMove.position.row - draggedNode.position.row;
  const moves: GroupMove[] = [draggedMove];
  for (const id of selectedIds) {
    if (id === draggedId) continue;
    const n = desktopNodes.find(x => x.id === id);
    if (!n?.position) continue;
    moves.push({ nodeId: id, position: { col: n.position.col + dCol, row: n.position.row + dRow } });
  }
  return moves;
}
