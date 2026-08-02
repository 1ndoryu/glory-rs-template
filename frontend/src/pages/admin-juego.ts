/* wandori.us — Admin Juego (catálogo de personajes)
 * Tab del panel Admin que gestiona el catálogo allowlisted de personajes del
 * Bosque: lista activas e inactivas (el backend solo deja ver inactivas a
 * admin), alta, renombrado, cambio de tono y activación/desactivación.
 * [297A-53] Sigue el patrón de admin-notifications: WeakMap de guards de
 * generación + modal con el diálogo B&W del OS; sin CSS nuevo ni edición de
 * estilos ajenos. El contrato admin (isActive/createdAt) lo valida el
 * servicio con validadores estrictos, igual que el catálogo público. */

import { safeRun } from '../utils/safe-async';
import { tryCatch } from '../utils/result';
import {
  GameCharacterAdminService,
  isValidAdminId,
  isValidAdminLabel,
  type GameCharacterAdminEntry,
} from '../services/game-character-admin.service';
import { createEl } from '../utils/dom';
import { createVacio } from '../components/ui/empty-state';
import { createModal } from '../components/ui/modal';
import { createInput } from '../components/ui/input';
import { createSelect } from '../components/ui/select';
import { showToast } from '../components/ui/toast';
import { showConfirm } from '../components/ui/confirm';

const TONO_ETIQUETA: Record<string, string> = {
  ink: 'ink',
  middle: 'middle',
  paper: 'paper',
};

const gameCharacterListGenerations = new WeakMap<HTMLElement, number>();

/** Liberar el guard de generación de una lista antes de desmontar su ventana. */
export function disposeGameCharacterAdminList(container: HTMLElement): void {
  gameCharacterListGenerations.delete(container);
}

/** Liberar todas las listas del catálogo pertenecientes a una página Admin. */
export function disposeAdminGameCharacterLists(page: HTMLElement): void {
  page.querySelectorAll<HTMLElement>('.admin-lista').forEach(disposeGameCharacterAdminList);
}

function tonoLabel(entry: GameCharacterAdminEntry): string {
  return TONO_ETIQUETA[entry.bodyTone] ?? entry.bodyTone;
}

/** Renderiza el listado completo del catálogo (activas e inactivas) con guard
 * de generación: si la ventana se desmonta o cambia de tab, no toca el DOM. */
export async function renderGameCharacterAdminList(container: HTMLElement): Promise<void> {
  const generation = (gameCharacterListGenerations.get(container) ?? 0) + 1;
  gameCharacterListGenerations.set(container, generation);
  container.textContent = '';
  container.appendChild(createEl('p', { className: 'cargando', textContent: 'cargando...' }));

  const result = await tryCatch(GameCharacterAdminService.listAll());
  if (gameCharacterListGenerations.get(container) !== generation) return;
  container.textContent = '';
  if (!result.ok) {
    container.appendChild(createVacio('error al cargar el catálogo de personajes'));
    return;
  }

  const items = result.value;
  for (const item of items) {
    container.appendChild(renderAdminItem(item, container));
  }
  if (items.length === 0) {
    container.appendChild(createVacio('no hay personajes en el catálogo'));
  }
}

function renderAdminItem(entry: GameCharacterAdminEntry, container: HTMLElement): HTMLElement {
  const tag = createEl('span', {
    className: 'tag-estado',
    textContent: entry.isActive ? 'activo' : 'inactivo',
  });
  const info = createEl('div', {},
    createEl('span', { textContent: entry.displayName }),
    createEl('small', { className: 'ml-sm', textContent: ` — ${tonoLabel(entry)}` }),
  );

  const editButton = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: 'editar',
  });
  editButton.addEventListener('click', () => openEditarPersonajeModal(entry, () => {
    void renderGameCharacterAdminList(container);
  }));

  const toggleButton = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: entry.isActive ? 'desactivar' : 'reactivar',
  });
  toggleButton.addEventListener('click', () => {
    /* [297A-53] Desactivar pide confirmación (desaparece del juego); reactivar
     * es reversible y va directo. Ambas vías reutilizan el mismo update. */
    void safeRun((async () => {
      if (entry.isActive) {
        const confirmed = await showConfirm(`desactivar "${entry.displayName}"?`);
        if (!confirmed) return;
      }
      await GameCharacterAdminService.update(entry.id, {
        displayName: entry.displayName,
        bodyTone: entry.bodyTone,
        isActive: !entry.isActive,
      });
      showToast(entry.isActive ? 'personaje desactivado' : 'personaje reactivado');
      void renderGameCharacterAdminList(container);
    })(), 'no se pudo actualizar el estado del personaje');
  });

  const actions = createEl('div', { className: 'admin-acciones' }, tag, editButton, toggleButton);
  return createEl('div', { className: 'admin-item' }, info, actions);
}

/** Modal de alta: id (slug allowlisted) + etiqueta + tono; nace activo. */
export function openNuevoPersonajeModal(onCreated: () => void): void {
  let id = '';
  let displayName = '';
  let bodyTone = 'ink';

  const idField = createInput({
    label: 'id (a-z, 0-9, guiones)',
    placeholder: 'forest-ranger',
    required: true,
    onInput: (v) => { id = v; },
  });
  const nameField = createInput({
    label: 'etiqueta visible',
    placeholder: 'Guardabosques',
    required: true,
    onInput: (v) => { displayName = v; },
  });
  const toneField = createSelect({
    label: 'tono de cuerpo',
    options: [
      { value: 'ink', label: 'ink' },
      { value: 'middle', label: 'middle' },
      { value: 'paper', label: 'paper' },
    ],
    value: bodyTone,
    onChange: (v) => { bodyTone = v; },
  });
  const feedback = createEl('p', { className: 'modal-feedback', role: 'status' });

  const btnCancelar = createEl('button', { type: 'button', className: 'boton', textContent: 'cancelar' });
  const btnCrear = createEl('button', { type: 'button', className: 'boton', textContent: 'crear personaje' });
  const acciones = createEl('div', { className: 'modal-acciones' }, btnCancelar, btnCrear);

  const modal = createModal({
    titulo: 'nuevo personaje',
    contenido: [idField, nameField, toneField, feedback, acciones],
    ancho: '420px',
  });

  btnCancelar.addEventListener('click', () => modal.close());
  btnCrear.addEventListener('click', () => {
    const cleanId = id.trim();
    const cleanName = displayName.trim();
    if (!isValidAdminId(cleanId)) {
      feedback.textContent = 'id no válido: solo minúsculas, dígitos y guiones (máx 32).';
      return;
    }
    if (!isValidAdminLabel(cleanName)) {
      feedback.textContent = 'etiqueta no válida: entre 1 y 48 caracteres, sin saltos de línea.';
      return;
    }
    btnCrear.disabled = true;
    feedback.textContent = 'guardando...';
    void safeRun(
      GameCharacterAdminService.create({
        id: cleanId,
        displayName: cleanName,
        bodyTone: bodyTone as GameCharacterAdminEntry['bodyTone'],
      }),
      'no se pudo crear el personaje',
    ).then((result) => {
      btnCrear.disabled = false;
      if (!result.ok) {
        feedback.textContent = 'no se pudo crear el personaje (¿id duplicado?).';
        return;
      }
      showToast('personaje creado');
      modal.close();
      onCreated();
    });
  });
}

/** Modal de edición: etiqueta + tono + estado activo (el id es inmutable). */
export function openEditarPersonajeModal(entry: GameCharacterAdminEntry, onSaved: () => void): void {
  let displayName = entry.displayName;
  let bodyTone = entry.bodyTone;
  let isActive = entry.isActive;

  const nameField = createInput({
    label: 'etiqueta visible',
    value: entry.displayName,
    required: true,
    onInput: (v) => { displayName = v; },
  });
  const toneField = createSelect({
    label: 'tono de cuerpo',
    options: [
      { value: 'ink', label: 'ink' },
      { value: 'middle', label: 'middle' },
      { value: 'paper', label: 'paper' },
    ],
    value: entry.bodyTone,
    onChange: (v) => { bodyTone = v as GameCharacterAdminEntry['bodyTone']; },
  });
  const stateField = createSelect({
    label: 'estado',
    options: [
      { value: 'true', label: 'activo' },
      { value: 'false', label: 'inactivo' },
    ],
    value: String(entry.isActive),
    onChange: (v) => { isActive = v === 'true'; },
  });
  const feedback = createEl('p', { className: 'modal-feedback', role: 'status' });

  const btnCancelar = createEl('button', { type: 'button', className: 'boton', textContent: 'cancelar' });
  const btnGuardar = createEl('button', { type: 'button', className: 'boton', textContent: 'guardar' });
  const acciones = createEl('div', { className: 'modal-acciones' }, btnCancelar, btnGuardar);

  const modal = createModal({
    titulo: `editar ${entry.id}`,
    contenido: [nameField, toneField, stateField, feedback, acciones],
    ancho: '420px',
  });

  btnCancelar.addEventListener('click', () => modal.close());
  btnGuardar.addEventListener('click', () => {
    const cleanName = displayName.trim();
    if (!isValidAdminLabel(cleanName)) {
      feedback.textContent = 'etiqueta no válida: entre 1 y 48 caracteres, sin saltos de línea.';
      return;
    }
    btnGuardar.disabled = true;
    feedback.textContent = 'guardando...';
    void safeRun(
      GameCharacterAdminService.update(entry.id, {
        displayName: cleanName,
        bodyTone,
        isActive,
      }),
      'no se pudo guardar el personaje',
    ).then((result) => {
      btnGuardar.disabled = false;
      if (!result.ok) {
        feedback.textContent = 'no se pudo guardar el personaje.';
        return;
      }
      showToast('personaje actualizado');
      modal.close();
      onSaved();
    });
  });
}
