/* wandori.us — Grid Geometry for free icon placement
 * Cálculos puros y geometría del snap-grid del escritorio (297A-20).
 * Separa la lógica de celdas/colisiones del DOM para poder testearla. */

import type { GridPosition, NodeId, WorkspaceNode } from '../../runtime/workspace/types';

export type { GridPosition } from '../../runtime/workspace/types';

/** Ancho mínimo de viewport para posicionamiento libre (desktop/tablet ≥768). */
export const DESKTOP_MIN_WIDTH = 769;

/** Métricas del grid medidas desde el DOM. */
export interface GridMetrics {
  readonly columns: number;
  readonly rows: number;
  readonly cellWidth: number;
  readonly cellHeight: number;
  readonly columnGap: number;
  readonly rowGap: number;
  readonly left: number;
  readonly right: number;
  readonly top: number;
  /** Grid en direction: rtl (col 0 = columna derecha, crece hacia la izquierda). */
  readonly rtl: boolean;
}

/** Medir columnas, celdas y gaps del grid real (desktop/tablet). */
export function getGridMetrics(
  gridEl: HTMLElement,
  itemSelector = '.desktop-icon--interactive',
): GridMetrics {
  const rect = gridEl.getBoundingClientRect();
  const cs = getComputedStyle(gridEl);
  const columnGap = parseFloat(cs.columnGap) || 0;
  const rowGap = parseFloat(cs.rowGap) || 0;
  const template = cs.gridTemplateColumns;
  const columns = !template || template === 'none' ? 1 : template.split(' ').length;
  const first = gridEl.querySelector<HTMLElement>(itemSelector);
  const cellWidth = first ? first.getBoundingClientRect().width : 0;
  /* [297A-20] Altura de fila = grid-auto-rows (fijo) para que la geometría
   * coincida con el CSS grid real; fallback al alto del icono si no es fijo. */
  const autoRows = parseFloat(cs.gridAutoRows);
  const cellHeight = autoRows > 0 ? autoRows : (first ? first.getBoundingClientRect().height : 0);
  const rows = Math.max(1, Math.floor((rect.height + rowGap) / (cellHeight + rowGap)));
  return {
    columns,
    rows,
    cellWidth,
    cellHeight,
    columnGap,
    rowGap,
    left: rect.left,
    right: rect.right,
    top: rect.top,
    rtl: cs.direction === 'rtl',
  };
}

/** Celda snap bajo unas coordenadas de viewport, o null si cae fuera del grid. */
export function getCellAt(
  x: number,
  y: number,
  metrics: GridMetrics,
): GridPosition | null {
  const { left, right, top, cellWidth, cellHeight, columnGap, rowGap, columns, rows, rtl } = metrics;
  if (cellWidth <= 0 || cellHeight <= 0) return null;
  const col = rtl
    ? Math.floor((right - x + columnGap) / (cellWidth + columnGap))
    : Math.floor((x - left + columnGap) / (cellWidth + columnGap));
  const row = Math.floor((y - top + rowGap) / (cellHeight + rowGap));
  if (col < 0 || row < 0 || col >= columns || row >= rows) return null;
  return { col, row };
}

/** Clave estable de una celda para ocupación. */
export function cellKey(position: GridPosition): string {
  return `${position.col},${position.row}`;
}

/** Mapa celda → nodeId para todos los nodos con posición explícita. */
export function occupiedCells(nodes: readonly WorkspaceNode[]): Map<string, NodeId> {
  const map = new Map<string, NodeId>();
  for (const node of nodes) {
    if (node.position) map.set(cellKey(node.position), node.id);
  }
  return map;
}

/** Primera celda libre desde la fila de partida; autogrow si el grid está lleno.
 * [297A-20] `avoid` excluye una celda de la búsqueda (p.ej. la celda destino
 * del icono arrastrado) para no proponer dos iconos en la misma celda. */
export function findFreeCell(
  nodes: readonly WorkspaceNode[],
  start: GridPosition,
  metrics: GridMetrics,
  avoid?: GridPosition,
): GridPosition {
  const occupied = occupiedCells(nodes);
  for (let row = start.row; row < metrics.rows; row++) {
    for (let col = 0; col < metrics.columns; col++) {
      const candidate = { col, row };
      if (avoid && candidate.col === avoid.col && candidate.row === avoid.row) continue;
      if (!occupied.has(cellKey(candidate))) return candidate;
    }
  }
  /* Grid lleno: autogrow en la fila siguiente */
  return { col: start.col, row: metrics.rows };
}

/** Movimientos a aplicar al soltar un icono en una celda (resuelve colisiones). */
export interface PlacementPlan {
  readonly moves: ReadonlyArray<{ nodeId: NodeId; position: GridPosition }>;
}

export function planPlacement(
  nodes: readonly WorkspaceNode[],
  draggedId: NodeId,
  target: GridPosition,
  metrics: GridMetrics,
): PlacementPlan {
  const occupied = occupiedCells(nodes);
  const targetKey = cellKey(target);
  const occupant = occupied.get(targetKey);

  if (!occupant || occupant === draggedId) {
    return { moves: [{ nodeId: draggedId, position: target }] };
  }

  /* [297A-20] Evitar target: el arrastrado ocupará esa celda; sin esto el
   * ocupante y el arrastrado podrían terminar en la misma celda. */
  const free = findFreeCell(nodes.filter((n) => n.id !== occupant), target, metrics, target);
  return {
    moves: [
      { nodeId: occupant, position: free },
      { nodeId: draggedId, position: target },
    ],
  };
}

/** [297A-20] Reflow al cambiar el tamaño del grid (columns/rows).
 * Clampa posiciones fuera de rango y resuelve colisiones tras una reducción.
 * Devuelve SOLO los movimientos que cambian; sin re-render si no hace falta.
 * Eficiencia: se invoca únicamente cuando las métricas del grid cambian. */
export function reflowPositions(
  nodes: readonly WorkspaceNode[],
  metrics: GridMetrics,
): PlacementPlan {
  const positioned = nodes
    .filter((n) => n.position)
    .sort((a, b) => (a.position!.row - b.position!.row) || (a.position!.col - b.position!.col));

  const taken = new Set<string>();
  const moves: Array<{ nodeId: NodeId; position: GridPosition }> = [];

  for (const node of positioned) {
    const orig = node.position!;
    const col = Math.min(orig.col, metrics.columns - 1);
    const row = Math.min(orig.row, metrics.rows - 1);

    /* Primera celda libre: misma fila, empezando en la col clampada y yendo
     * hacia la derecha (c decreciente). Así el icono se mantiene lo más cerca
     * de su posición; si la fila está llena, baja a la siguiente fila. */
    let placed = false;
    for (let r = row; r < metrics.rows && !placed; r++) {
      for (let c = col; c >= 0; c--) {
        const key = cellKey({ col: c, row: r });
        if (taken.has(key)) continue;
        taken.add(key);
        if (c !== orig.col || r !== orig.row) {
          moves.push({ nodeId: node.id, position: { col: c, row: r } });
        }
        placed = true;
        break;
      }
    }
  }

  return { moves: [...moves] };
}
