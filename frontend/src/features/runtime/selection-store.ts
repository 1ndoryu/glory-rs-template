/* wandori.us — Selection Store
 * Estado de selección de objetos del escritorio.
 * [Plan §3] Selección, activación y foco.
 * Clic selecciona; Ctrl alterna; Shift extiende rango.
 * Foco de teclado, selección de objetos y ventana activa son estados distintos. */

import { createStore, type Store } from '../../store';

export interface SelectionState {
  /** IDs de los objetos seleccionados. */
  readonly selectedIds: readonly string[];
  /** ID del último seleccionado (para extender rango con Shift). */
  readonly lastSelectedId: string | null;
  /** Si la selección es del workspace vacío (no un objeto). */
  readonly isBackground: boolean;
}

const initialState: SelectionState = {
  selectedIds: [],
  lastSelectedId: null,
  isBackground: false,
};

export const selectionStore: Store<SelectionState> = createStore(initialState);

/** Seleccionar un solo objeto (reemplaza selección). */
export function selectSingle(id: string): void {
  selectionStore.set({
    selectedIds: [id],
    lastSelectedId: id,
    isBackground: false,
  });
}

/** Alternar selección de un objeto (Ctrl/Cmd + clic). */
export function toggleSelect(id: string): void {
  const current = selectionStore.get();
  const isSelected = current.selectedIds.includes(id);
  const newIds = isSelected
    ? current.selectedIds.filter(i => i !== id)
    : [...current.selectedIds, id];
  selectionStore.set({
    selectedIds: newIds,
    lastSelectedId: id,
    isBackground: false,
  });
}

/** Extender selección desde el último seleccionado hasta el actual (Shift + clic).
 * idsInOrder = array ordenado de IDs visibles en el contenedor actual. */
export function extendSelect(id: string, idsInOrder: readonly string[]): void {
  const current = selectionStore.get();
  const anchor = current.lastSelectedId;
  if (!anchor || !idsInOrder.includes(anchor)) {
    selectSingle(id);
    return;
  }
  const startIdx = idsInOrder.indexOf(anchor);
  const endIdx = idsInOrder.indexOf(id);
  if (startIdx < 0 || endIdx < 0) {
    selectSingle(id);
    return;
  }
  const [from, to] = startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
  const rangeIds = idsInOrder.slice(from, to + 1);
  selectionStore.set({
    selectedIds: rangeIds,
    lastSelectedId: id,
    isBackground: false,
  });
}

/** Limpiar selección (clic en vacío o Escape). */
export function clearSelection(): void {
  selectionStore.set(initialState);
}

/** Marcar que se hizo clic en el fondo del workspace. */
export function selectBackground(): void {
  selectionStore.set({
    selectedIds: [],
    lastSelectedId: null,
    isBackground: true,
  });
}

/** Verificar si un objeto está seleccionado. */
export function isSelected(id: string): boolean {
  return selectionStore.get().selectedIds.includes(id);
}

/** Obtener IDs seleccionados. */
export function getSelectedIds(): readonly string[] {
  return selectionStore.get().selectedIds;
}
