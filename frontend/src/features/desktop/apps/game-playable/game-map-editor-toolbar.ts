/* GAME-01 — Toolbar del Editor de mapa 2D del Bosque.
 * [297A-64] Construcción y refresco de la barra de herramientas del editor
 * (herramientas, paleta, deshacer/rehacer, borrar/duplicar, publicar) y del
 * pie con hint + issues. Separado de la vista para mantener <300 líneas por
 * módulo y que la vista solo cablee acciones. */

import { createEl } from '../../../../utils/dom';
import {
  getValidationIssues,
  hasChanges,
  TERRAIN_SURFACE_LABEL,
  TERRAIN_SURFACE_VALUES,
  type MapEditorState,
  type MapEditorTool,
} from './game-map-editor-core';

export interface EditorToolbarElements {
  readonly toolbar: HTMLElement;
  readonly hint: HTMLElement;
  readonly issuesEl: HTMLElement;
  readonly assetSlot: HTMLElement;
  readonly surfaceSelect: HTMLSelectElement;
  readonly toolButtons: ReadonlyMap<MapEditorTool, HTMLButtonElement>;
  readonly btnUndo: HTMLButtonElement;
  readonly btnRedo: HTMLButtonElement;
  readonly btnDelete: HTMLButtonElement;
  readonly btnDuplicate: HTMLButtonElement;
  readonly btnPublish: HTMLButtonElement;
  /** Refresca estados activos, disabled y el texto de validación/cambios. */
  readonly refresh: (state: MapEditorState) => void;
}

export function createEditorToolbar(): EditorToolbarElements {
  const toolButtons = new Map<MapEditorTool, HTMLButtonElement>();
  const btnSelect = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'seleccionar' });
  const btnPlace = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'colocar' });
  const btnSpawn = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'spawn' });
  const btnPaint = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'pintar' });
  toolButtons.set('select', btnSelect);
  toolButtons.set('place', btnPlace);
  toolButtons.set('spawn', btnSpawn);
  toolButtons.set('paint', btnPaint);

  /* [297A-64] La paleta se puebla al cargar el catálogo (opciones reales).
   * [297A-66] El pincel usa un select propio de superficies (suelo/agua). */
  const assetSlot = createEl('span', { className: 'juegoConfig__editor-asset' });
  const surfaceSlot = createEl('span', { className: 'juegoConfig__editor-surface' });
  const btnUndo = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'deshacer' });
  const btnRedo = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'rehacer' });
  const btnDelete = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'borrar' });
  const btnDuplicate = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'duplicar' });
  const btnPublish = createEl('button', { type: 'button', className: 'boton boton-pequeno', textContent: 'publicar mapa' });

  const toolbar = createEl('div', { className: 'juegoConfig__editor-toolbar' },
    btnSelect, btnPlace, btnSpawn, btnPaint,
    assetSlot, surfaceSlot,
    btnUndo, btnRedo, btnDelete, btnDuplicate,
    btnPublish,
  );

  const hint = createEl('p', { className: 'juegoConfig__editor-hint', textContent: 'cargando mapa…' });
  const issuesEl = createEl('p', { className: 'juegoConfig__editor-issues', role: 'status' });

  /* [297A-66] Select de superficie del pincel (suelo/agua), fijo; la vista
   * engancha onChange. */
  const surfaceSelect = createEl('select', {
    className: 'juegoConfig__editor-surface-select',
    'aria-label': 'superficie del pincel',
  });
  for (const [key, value] of Object.entries(TERRAIN_SURFACE_VALUES)) {
    const option = createEl('option', {
      value: String(value),
      textContent: TERRAIN_SURFACE_LABEL[key as keyof typeof TERRAIN_SURFACE_LABEL] ?? key,
    });
    surfaceSelect.appendChild(option);
  }
  surfaceSlot.appendChild(surfaceSelect);

  const refresh = (state: MapEditorState): void => {
    for (const [tool, button] of toolButtons) {
      button.classList.toggle('boton--activo', state.tool === tool);
    }
    surfaceSelect.value = String(state.activeSurface);
    btnUndo.disabled = state.undoStack.length === 0;
    btnRedo.disabled = state.redoStack.length === 0;
    btnDelete.disabled = state.selectedId === null;
    btnDuplicate.disabled = state.selectedId === null;
    const issues = getValidationIssues(state);
    const dirty = hasChanges(state);
    btnPublish.disabled = issues.length > 0;
    issuesEl.textContent = issues.length > 0
      ? `pendiente: ${issues[0].path} ${issues[0].message}`
      : dirty
        ? 'borrador con cambios · publicar crea una versión inmutable'
        : 'sin cambios desde la última publicación';
  };

  return {
    toolbar,
    hint,
    issuesEl,
    assetSlot,
    surfaceSelect,
    toolButtons,
    btnUndo,
    btnRedo,
    btnDelete,
    btnDuplicate,
    btnPublish,
    refresh,
  };
}
