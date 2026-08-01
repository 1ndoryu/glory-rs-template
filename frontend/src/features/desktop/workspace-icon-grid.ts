/* wandori.us — Workspace Icon Grid
 * Grid reactivo de iconos del escritorio suscrito a workspaceStore.
 * [Plan 297A-11 §9.4] Extraído de desktop-shell.ts para reducir acoplamiento. */

import {
  FileUser,
  Folder,
  ShieldUser,
  type IconNode,
} from 'lucide';
import { createEl } from '../../utils/dom';
import { createDesktopIcon } from './components/desktop-icon';
import { openAppWindow } from '../runtime/route-app-adapter';
import { authStore } from '../../store';
import { openContextMenu } from './components/desktop-context-menu';
import { selectionStore, selectSingle, clearSelection, isSelected } from '../runtime/selection-store';
import { workspaceStore, reorderDesktopNodes } from '../runtime/workspace/workspace-store';
import type { ResolvedNode } from '../runtime/workspace/types';
import { AppRegistry } from '../runtime/app-registry';
import { resolveResourceIcon, resolveResourceIconType } from '../runtime/resource-type-registry';
import { enableDrag } from './utils/icon-drag';
import { DESKTOP_MIN_WIDTH, getGridMetrics, planPlacement, reflowPositions } from './utils/icon-grid';
import { moveNodesPosition } from '../runtime/workspace/overlay-mutations';
import { reconcileChildren } from '../../utils/reconcile';
import { resolvePublicResourceTarget } from '../runtime/workspace/public-resource-locator';
import { showToast } from '../../components/ui/toast';

const SHELL_ICON_MAP: Record<string, IconNode> = {
  'profile': FileUser,
  'admin': ShieldUser,
};

export function resolveNodeIcon(node: ResolvedNode): IconNode {
  if (node.type === 'app' && node.refId) {
    const app = AppRegistry.get(node.refId);
    if (app) return app.icon;
  }
  /* [018A-79] Los recursos resuelven su icono en ResourceTypeRegistry (fuente
   * única con Finder y móvil); antes el fallback genérico devolvía carpeta. */
  if (node.type === 'resource' && node.resourceKind) {
    return resolveResourceIcon(node.resourceKind);
  }
  return SHELL_ICON_MAP[node.id] ?? Folder;
}

export function resolveNodeIconType(node: ResolvedNode): 'folder' | 'document' | 'application' {
  if (node.type === 'app' && node.refId) {
    const app = AppRegistry.get(node.refId);
    if (app?.iconType) return app.iconType;
  }
  if (node.type === 'folder') return 'folder';
  /* [018A-79] El tipo semántico también sale del registro (iconType por kind). */
  if (node.type === 'resource' && node.resourceKind) {
    return resolveResourceIconType(node.resourceKind);
  }
  if (node.type === 'shortcut') return 'document';
  return 'application';
}

function resolveActivate(
  node: ResolvedNode,
  extraActions?: Record<string, () => void>,
): (() => void) | undefined {
  if (extraActions?.[node.id]) return extraActions[node.id];
  if (node.type === 'folder') return () => { void openAppWindow('finder', { folderId: node.id }); };
  if (node.type === 'resource' && node.resourceKind) {
    const entry = resolvePublicResourceTarget(node);
    if (entry) return () => { void openAppWindow(entry.appId, entry.params); };
    return () => {
      showToast('Este recurso todavía no tiene una referencia pública disponible');
    };
  }
  if (node.refId) return () => { void openAppWindow(node.refId!); };
  return undefined;
}

export interface WorkspaceIconGrid {
  readonly element: HTMLElement;
  readonly destroy: () => void;
}

/* [018A-88] Reflejo visual de la selección en el escritorio: clase
 * .desktop-icon--selected + aria-selected. La clase ya existía en
 * createDesktopIcon y su CSS, pero nadie la cableaba al selectionStore
 * (la selección solo vivía en el store, sin estado visible). */
function applyIconSelection(el: HTMLElement, nodeId: string): void {
  const selected = isSelected(nodeId, 'desktop');
  el.classList.toggle('desktop-icon--selected', selected);
  el.setAttribute('aria-selected', String(selected));
}

/** [297A-20] Aplica la posición snap del nodo al elemento (o lo devuelve a auto-flow).
 * Usa custom properties para que el CSS decida la colocación y el media query
 * móvil pueda ignorarla sin JS. */
function applyIconPosition(el: HTMLElement, node: ResolvedNode): void {
  if (node.position) {
    el.classList.add('desktop-icon--posicionado');
    el.style.setProperty('--icono-col', String(node.position.col + 1));
    el.style.setProperty('--icono-row', String(node.position.row + 1));
  } else {
    el.classList.remove('desktop-icon--posicionado');
    el.style.removeProperty('--icono-col');
    el.style.removeProperty('--icono-row');
  }
}

export function createWorkspaceIconGrid(extraActions?: Record<string, () => void>): WorkspaceIconGrid {
  const grid = createEl('div', { className: 'desktop-icon-grid', ariaLabel: 'Objetos del escritorio' });

  const dragCleanups = new Map<string, () => void>();

  const stopWorkspace = workspaceStore.subscribe((ws) => {
    const desktopNodes = Object.values(ws.nodes)
      .filter((n) => n.parentId === 'desktop')
      .sort((a, b) => (a.mobileOrder ?? 0) - (b.mobileOrder ?? 0));

    const activableNodes = desktopNodes.filter(n => resolveActivate(n, extraActions));

    const activeIds = new Set(activableNodes.map(n => n.id));
    for (const [id, cleanup] of dragCleanups) {
      if (!activeIds.has(id)) {
        cleanup();
        dragCleanups.delete(id);
      }
    }

    reconcileChildren(
      grid,
      activableNodes,
      (node) => node.id,
      (node) => {
        const onActivate = resolveActivate(node, extraActions);
        if (!onActivate) return createEl('span'); /* placeholder */

        const iconEl = createDesktopIcon({
          label: node.label,
          type: resolveNodeIconType(node),
          selected: isSelected(node.id, 'desktop'),
          lucideIcon: resolveNodeIcon(node),
          onActivate,
        });

        iconEl.setAttribute('data-node-id', node.id);
        applyIconSelection(iconEl, node.id);

        iconEl.addEventListener('mousedown', (e) => {
          if (e.button === 0 && e.detail === 1) {
            const nid = iconEl.getAttribute('data-node-id');
            if (nid) selectSingle(nid, 'desktop');
          }
        });

        iconEl.addEventListener('contextmenu', (e) => {
          e.preventDefault();
          const nid = iconEl.getAttribute('data-node-id');
          if (!nid) return;
          const ws = workspaceStore.get();
          const currentNode = ws.nodes[nid];
          if (!currentNode) return;
          selectSingle(nid, 'desktop');
          openContextMenu({
            context: 'icon',
            targets: [{ id: currentNode.refId ?? nid, kind: currentNode.type === 'app' ? 'app' : 'shortcut' }],
            capability: authStore.get().capability,
            x: e.clientX,
            y: e.clientY,
          });
        });

        const cleanup = enableDrag({
          el: iconEl,
          nodeId: node.id,
          context: 'desktop',
          gridEl: grid,
          itemSelector: '.desktop-icon--interactive',
          onReorder: (draggedId, targetIndex) => {
            /* Reorder por índice (mobileOrder) — usado solo como fallback móvil.
             * En desktop/tablet el drag usa onPlaceCell (297A-20). */
            const ws = workspaceStore.get();
            const currentIds = Object.values(ws.nodes)
              .filter((n) => n.parentId === 'desktop')
              .sort((a, b) => (a.mobileOrder ?? 0) - (b.mobileOrder ?? 0))
              .map((n) => n.id);
            const fromIndex = currentIds.indexOf(draggedId);
            if (fromIndex < 0 || fromIndex === targetIndex) return;
            const reordered = [...currentIds];
            reordered.splice(fromIndex, 1);
            reordered.splice(targetIndex, 0, draggedId);
            reorderDesktopNodes(reordered);
          },
          onPlaceCell: (draggedId, col, row) => {
            /* [297A-20] Snap-grid: resuelve colisiones y persiste en el overlay.
             * workspaceStore ya devuelve nodos con position resuelta.
             * Un solo update de overlay por soltada (moves en batch). */
            const ws = workspaceStore.get();
            const desktopNodes = Object.values(ws.nodes).filter((n) => n.parentId === 'desktop');
            const metrics = getGridMetrics(grid);
            const plan = planPlacement(desktopNodes, draggedId, { col, row }, metrics);
            moveNodesPosition(plan.moves);
          },
        });
        dragCleanups.set(node.id, cleanup);

        applyIconPosition(iconEl, node);

        return iconEl;
      },
      (el, node) => {
        const label = el.querySelector('.desktop-icon__label');
        if (label && label.textContent !== node.label) {
          label.textContent = node.label;
        }
        const newType = resolveNodeIconType(node);
        el.classList.remove('desktop-icon--folder', 'desktop-icon--document', 'desktop-icon--application');
        el.classList.add(`desktop-icon--${newType}`);
        applyIconSelection(el, node.id);
        applyIconPosition(el, node);
      },
    );
  });

  /* [018A-88] Reflejo en vivo de la selección: al cambiar selectionStore
   * (clic en un icono, Ctrl+clic, clic en fondo) se re-aplica el estado
   * visual sin reconstruir el grid. */
  const stopSelection = selectionStore.subscribe(() => {
    for (const el of grid.children) {
      const id = el.getAttribute('data-node-id');
      if (id) applyIconSelection(el as HTMLElement, id);
    }
  });

  grid.addEventListener('mousedown', (e) => {
    if (e.target === grid) clearSelection();
  });

  /* [297A-20] Reflow eficiente al cambiar el tamaño del grid.
   * Debounce 150ms; solo recalcula si cambiaron columns/rows (layout real).
   * El reflow se aplica en un único update batch del overlay, y no toca el
   * store si no hay movimientos (métricas iguales ⇒ return O(1)). */
  let lastColumns = 0;
  let lastRows = 0;
  let resizeTimer: ReturnType<typeof setTimeout> | undefined;
  let frameHandle: number | undefined;

  /* [297A-20][DEPURACION TEMPORAL] Overlay que muestra el límite del grid y
   * cada celda con su col,row. Activar con Ctrl+Shift+G.
   * PENDIENTE: eliminar junto con el CSS .desktop-icon-grid--depurar. */
  let debugRender: (() => void) | undefined;

  const toggleDebugGrid = (): void => {
    const active = grid.classList.toggle('desktop-icon-grid--depurar');
    let layer = grid.querySelector<HTMLElement>('.desktop-icon-grid__debug');
    if (!active) {
      layer?.remove();
      debugRender = undefined;
      return;
    }
    if (!layer) {
      layer = createEl('div', { className: 'desktop-icon-grid__debug' });
      grid.appendChild(layer);
    }
    debugRender = (): void => {
      if (!layer) return;
      layer.replaceChildren();
      const metrics = getGridMetrics(grid);
      const rect = grid.getBoundingClientRect();
      for (let row = 0; row < metrics.rows; row++) {
        for (let col = 0; col < metrics.columns; col++) {
          const cell = createEl('div', { className: 'desktop-icon-grid__debug-celda' });
          cell.textContent = `${col},${row}`;
          /* Misma geometría que getCellAt: col 0 = derecha en RTL.
           * [297A-20] Fórmula corregida: right - (col+1)*cellWidth - col*gap
           * (antes se restaba un gap de más por columna y la cuadrícula
           * quedaba desplazada respecto a las celdas reales). */
          const x = metrics.rtl
            ? rect.width - (col + 1) * metrics.cellWidth - col * metrics.columnGap
            : col * (metrics.cellWidth + metrics.columnGap);
          const y = row * (metrics.cellHeight + metrics.rowGap);
          cell.style.left = `${x}px`;
          cell.style.top = `${y}px`;
          cell.style.width = `${metrics.cellWidth}px`;
          cell.style.height = `${metrics.cellHeight}px`;
          layer.appendChild(cell);
        }
      }
    };
    debugRender();
  };

  const onKeyDown = (e: KeyboardEvent): void => {
    if (e.ctrlKey && e.shiftKey && (e.key === 'G' || e.key === 'g')) {
      e.preventDefault();
      toggleDebugGrid();
    }
  };

  const doReflow = (): void => {
    /* En móvil (<769) las posiciones se ignoran; no reencuadrar. */
    if (window.innerWidth < DESKTOP_MIN_WIDTH) {
      const metrics = getGridMetrics(grid);
      lastColumns = metrics.columns;
      lastRows = metrics.rows;
      debugRender?.();
      return;
    }
    const metrics = getGridMetrics(grid);
    if (metrics.columns === lastColumns && metrics.rows === lastRows) {
      debugRender?.();
      return;
    }
    lastColumns = metrics.columns;
    lastRows = metrics.rows;
    const ws = workspaceStore.get();
    const desktopNodes = Object.values(ws.nodes).filter((n) => n.parentId === 'desktop');
    const plan = reflowPositions(desktopNodes, metrics);
    if (plan.moves.length > 0) moveNodesPosition(plan.moves);
    debugRender?.();
  };

  const onWindowResize = (): void => {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(doReflow, 150);
  };

  window.addEventListener('resize', onWindowResize);
  window.addEventListener('keydown', onKeyDown);
  /* Inicializar métricas tras el primer paint (el grid ya está en el DOM). */
  frameHandle = requestAnimationFrame(() => {
    doReflow();
    frameHandle = undefined;
  });

  const destroy = (): void => {
    stopWorkspace();
    stopSelection();
    window.removeEventListener('resize', onWindowResize);
    window.removeEventListener('keydown', onKeyDown);
    window.clearTimeout(resizeTimer);
    if (frameHandle !== undefined) cancelAnimationFrame(frameHandle);
    for (const cleanup of dragCleanups.values()) cleanup();
    dragCleanups.clear();
    grid.replaceChildren();
  };

  return { element: grid, destroy };
}
