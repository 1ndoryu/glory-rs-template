/* [058A-4][297A-20 DEPURACION TEMPORAL] Overlay del snap-grid que muestra el
 * límite del grid y cada celda con su col,row. Se activa con Ctrl+Shift+G.
 * PENDIENTE: eliminar junto con el CSS .desktop-icon-grid--depurar.
 * Extraído de workspace-icon-grid.ts (límite de líneas de componente). */

import { createEl } from '../../../utils/dom';
import { getGridMetrics } from './icon-grid';

export interface DebugGridOverlay {
  /** Activa/desactiva la capa de depuración (Ctrl+Shift+G). */
  readonly toggle: () => void;
  /** Redibuja las celdas si la capa está activa (no-op si no lo está). */
  readonly refresh: () => void;
  /** Elimina la capa y libera la referencia (destroy del grid). */
  readonly dispose: () => void;
}

/** Overlay de depuración del snap-grid: mantiene su propia capa DOM dentro
 * del grid y recalcula las celdas en cada reflow. */
export function createDebugGridOverlay(grid: HTMLElement): DebugGridOverlay {
  let layer: HTMLElement | null = null;

  const render = (): void => {
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
        /* [058A-1] rowGap efectivo: con align-content distribuido las filas
         * reales no están a rowGap uniforme; replicar la distribución. */
        const y = row * (metrics.cellHeight + metrics.rowGapEffective);
        cell.style.left = `${x}px`;
        cell.style.top = `${y}px`;
        cell.style.width = `${metrics.cellWidth}px`;
        cell.style.height = `${metrics.cellHeight}px`;
        layer.appendChild(cell);
      }
    }
  };

  const toggle = (): void => {
    const active = grid.classList.toggle('desktop-icon-grid--depurar');
    if (!active) {
      layer?.remove();
      layer = null;
      return;
    }
    if (!layer) {
      layer = createEl('div', { className: 'desktop-icon-grid__debug' });
      grid.appendChild(layer);
    }
    render();
  };

  return {
    toggle,
    refresh: render,
    dispose: (): void => {
      layer?.remove();
      layer = null;
    },
  };
}
