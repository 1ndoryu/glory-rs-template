/* GAME-01 — Configuración del Bosque (vista dentro de la ventana del juego).
 * [297A-63] Reemplaza al modal 297A-62: el comando `game:settings` del
 * toolbar dispara un evento sobre la ventana del juego y esta alterna su
 * contenido — la escena se retira por un momento y aparece el panel de
 * configuración con TABS (personajes / assets / actividad) para organizar
 * la gestión de catálogos. El panel reutiliza los servicios admin existentes
 * sin duplicar contratos; los modales de alta/edición siguen siendo diálogos
 * puntuales del OS dentro de esta vista. Sin preview de modelos hasta 3D. */

import { safeRun } from '../../../../utils/safe-async';
import { tryCatch } from '../../../../utils/result';
import {
  GameCharacterAdminService,
  isValidAdminId,
  isValidAdminLabel,
  type GameCharacterAdminEntry,
} from '../../../../services/game-character-admin.service';
import {
  GameAssetAdminService,
  GAME_ASSET_CATEGORIES,
  isValidAdminAssetId,
  isValidAdminAssetLabel,
  type GameAssetAdminEntry,
} from '../../../../services/game-asset-admin.service';
import {
  GameAuditService,
  type GameAuditEventEntry,
} from '../../../../services/game-audit.service';
import { createEl } from '../../../../utils/dom';
import { createVacio } from '../../../../components/ui/empty-state';
import { createModal } from '../../../../components/ui/modal';
import { createInput } from '../../../../components/ui/input';
import { createSelect } from '../../../../components/ui/select';
import { createTabs } from '../../../../components/ui/tabs';
import { showToast } from '../../../../components/ui/toast';
import { showConfirm } from '../../../../components/ui/confirm';
import { createGameMapEditor } from './game-map-editor';
import { openAssetVersionsPanel } from './game-asset-versions';

const TONO_ETIQUETA: Record<string, string> = {
  ink: 'ink',
  middle: 'middle',
  paper: 'paper',
};

const ACCION_ETIQUETA: Record<string, string> = {
  'character.created': 'creado',
  'character.updated': 'actualizado',
  'map.published': 'publicado',
  'asset.created': 'creado',
  'asset.updated': 'actualizado',
};

const gameCharacterListGenerations = new WeakMap<HTMLElement, number>();
const gameAssetListGenerations = new WeakMap<HTMLElement, number>();
const gameMapEditorCleanups = new WeakMap<HTMLElement, () => void>();

function tonoLabel(entry: GameCharacterAdminEntry): string {
  return TONO_ETIQUETA[entry.bodyTone] ?? entry.bodyTone;
}

function formatFechaHora(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  const hora = `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
  return `${date.getDate()}/${date.getMonth() + 1}/${date.getFullYear()} ${hora}`;
}

/** Sección de actividad reciente (últimos eventos auditados), aislada: si la
 * auditoría falla, solo la sección lo indica y el catálogo sigue operativo. */
function renderActividad(
  result: { ok: true; value: GameAuditEventEntry[] } | { ok: false; error: string },
  titulo: string,
): HTMLElement {
  const seccion = createEl('section');
  seccion.appendChild(createEl('h4', { className: 'mt-md mb-sm', textContent: titulo }));
  if (!result.ok) {
    seccion.appendChild(createVacio('no se pudo cargar la actividad'));
    return seccion;
  }
  if (result.value.length === 0) {
    seccion.appendChild(createVacio('sin actividad reciente'));
    return seccion;
  }
  for (const event of result.value) {
    const label = ACCION_ETIQUETA[event.action] ?? event.action;
    const payload = event.payload;
    const nombre = typeof payload?.displayName === 'string' ? payload.displayName : event.entityId;
    const info = createEl('div', {},
      createEl('span', { textContent: `${label} · ${nombre}` }),
      createEl('small', { className: 'ml-sm', textContent: formatFechaHora(event.createdAt) }),
    );
    seccion.appendChild(createEl('div', { className: 'admin-item' }, info));
  }
  return seccion;
}

/** Renderiza el listado de personajes (activas e inactivas) con guard de
 * generación: si la vista se desmonta mientras carga, no toca el DOM. */
async function renderPersonajes(container: HTMLElement): Promise<void> {
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

  const auditResult = await tryCatch(GameAuditService.listCharacterEvents({ limit: 10 }));
  if (gameCharacterListGenerations.get(container) !== generation) return;

  for (const item of result.value) {
    container.appendChild(renderPersonajeItem(item, container));
  }
  if (result.value.length === 0) {
    container.appendChild(createVacio('no hay personajes en el catálogo'));
  }
  container.appendChild(renderActividad(auditResult, 'actividad'));
}

/** Renderiza el listado de assets (activas e inactivas) con guard propio y
 * actividad de assets aislada en paralelo. */
async function renderAssets(container: HTMLElement): Promise<void> {
  const generation = (gameAssetListGenerations.get(container) ?? 0) + 1;
  gameAssetListGenerations.set(container, generation);
  container.textContent = '';
  container.appendChild(createEl('p', { className: 'cargando', textContent: 'cargando...' }));

  const result = await tryCatch(GameAssetAdminService.listAll());
  if (gameAssetListGenerations.get(container) !== generation) return;
  container.textContent = '';
  if (!result.ok) {
    container.appendChild(createVacio('error al cargar el catálogo de assets'));
    return;
  }

  const auditResult = await tryCatch(GameAuditService.listAssetEvents({ limit: 10 }));
  if (gameAssetListGenerations.get(container) !== generation) return;

  for (const item of result.value) {
    container.appendChild(renderAssetItem(item, container));
  }
  if (result.value.length === 0) {
    container.appendChild(createVacio('no hay assets en el catálogo'));
  }
  container.appendChild(renderActividad(auditResult, 'actividad'));
}

function renderPersonajeItem(entry: GameCharacterAdminEntry, container: HTMLElement): HTMLElement {
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
    void renderPersonajes(container);
  }));

  const toggleButton = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: entry.isActive ? 'desactivar' : 'reactivar',
  });
  toggleButton.addEventListener('click', () => {
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
      void renderPersonajes(container);
    })(), 'no se pudo actualizar el estado del personaje');
  });

  const actions = createEl('div', { className: 'admin-acciones' }, tag, editButton, toggleButton);
  return createEl('div', { className: 'admin-item' }, info, actions);
}

function renderAssetItem(entry: GameAssetAdminEntry, container: HTMLElement): HTMLElement {
  const tag = createEl('span', {
    className: 'tag-estado',
    textContent: entry.isActive ? 'activo' : 'inactivo',
  });
  const info = createEl('div', {},
    createEl('span', { textContent: entry.displayName }),
    createEl('small', { className: 'ml-sm', textContent: ` — ${entry.category}` }),
  );

  const editButton = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: 'editar',
  });
  editButton.addEventListener('click', () => openEditarAssetModal(entry, () => {
    void renderAssets(container);
  }));

  const toggleButton = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: entry.isActive ? 'desactivar' : 'reactivar',
  });
  toggleButton.addEventListener('click', () => {
    void safeRun((async () => {
      if (entry.isActive) {
        const confirmed = await showConfirm(`desactivar "${entry.displayName}"?`);
        if (!confirmed) return;
      }
      await GameAssetAdminService.update(entry.id, {
        displayName: entry.displayName,
        category: entry.category,
        isActive: !entry.isActive,
      });
      showToast(entry.isActive ? 'asset desactivado' : 'asset reactivado');
      void renderAssets(container);
    })(), 'no se pudo actualizar el estado del asset');
  });

  /* [297A-73] Panel de versiones 3D (import GLB, preview, metadata, activar). */
  const versionsButton = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: 'versiones 3D',
  });
  versionsButton.addEventListener('click', () => {
    openAssetVersionsPanel(entry, () => {
      void renderAssets(container);
    });
  });

  const actions = createEl('div', { className: 'admin-acciones' }, tag, editButton, toggleButton, versionsButton);
  return createEl('div', { className: 'admin-item' }, info, actions);
}

/* === Alta y edición (formularios puntuales del OS dentro de la vista) === */

const CATEGORY_OPTIONS = GAME_ASSET_CATEGORIES.map((category) => ({ value: category, label: category }));
const TONE_OPTIONS = [
  { value: 'ink', label: 'ink' },
  { value: 'middle', label: 'middle' },
  { value: 'paper', label: 'paper' },
];

/** Modal de alta de personaje: id (slug allowlisted) + etiqueta + tono. */
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
    options: TONE_OPTIONS,
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

/** Modal de edición de personaje: etiqueta + tono + estado (id inmutable). */
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
    options: TONE_OPTIONS,
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

/** Modal de alta de asset: id + etiqueta + categoría; nace activo. */
export function openNuevoAssetModal(onCreated: () => void): void {
  let id = '';
  let displayName = '';
  let category = 'tree';

  const idField = createInput({
    label: 'id (a-z, 0-9, guiones)',
    placeholder: 'roble',
    required: true,
    onInput: (v) => { id = v; },
  });
  const nameField = createInput({
    label: 'etiqueta visible',
    placeholder: 'Roble',
    required: true,
    onInput: (v) => { displayName = v; },
  });
  const categoryField = createSelect({
    label: 'categoría',
    options: CATEGORY_OPTIONS,
    value: category,
    onChange: (v) => { category = v; },
  });
  const feedback = createEl('p', { className: 'modal-feedback', role: 'status' });

  const btnCancelar = createEl('button', { type: 'button', className: 'boton', textContent: 'cancelar' });
  const btnCrear = createEl('button', { type: 'button', className: 'boton', textContent: 'crear asset' });
  const acciones = createEl('div', { className: 'modal-acciones' }, btnCancelar, btnCrear);

  const modal = createModal({
    titulo: 'nuevo asset',
    contenido: [idField, nameField, categoryField, feedback, acciones],
    ancho: '420px',
  });

  btnCancelar.addEventListener('click', () => modal.close());
  btnCrear.addEventListener('click', () => {
    const cleanId = id.trim();
    const cleanName = displayName.trim();
    if (!isValidAdminAssetId(cleanId)) {
      feedback.textContent = 'id no válido: solo minúsculas, dígitos y guiones (máx 48).';
      return;
    }
    if (!isValidAdminAssetLabel(cleanName)) {
      feedback.textContent = 'etiqueta no válida: entre 1 y 64 caracteres, sin saltos de línea.';
      return;
    }
    btnCrear.disabled = true;
    feedback.textContent = 'guardando...';
    void safeRun(
      GameAssetAdminService.create({ id: cleanId, displayName: cleanName, category }),
      'no se pudo crear el asset',
    ).then((result) => {
      btnCrear.disabled = false;
      if (!result.ok) {
        feedback.textContent = 'no se pudo crear el asset (¿id duplicado?).';
        return;
      }
      showToast('asset creado');
      modal.close();
      onCreated();
    });
  });
}

/** Modal de edición de asset: etiqueta + categoría + estado (id inmutable). */
export function openEditarAssetModal(entry: GameAssetAdminEntry, onSaved: () => void): void {
  let displayName = entry.displayName;
  let category = entry.category;
  let isActive = entry.isActive;

  const nameField = createInput({
    label: 'etiqueta visible',
    value: entry.displayName,
    required: true,
    onInput: (v) => { displayName = v; },
  });
  const categoryField = createSelect({
    label: 'categoría',
    options: CATEGORY_OPTIONS,
    value: entry.category,
    onChange: (v) => { category = v; },
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
    contenido: [nameField, categoryField, stateField, feedback, acciones],
    ancho: '420px',
  });

  btnCancelar.addEventListener('click', () => modal.close());
  btnGuardar.addEventListener('click', () => {
    const cleanName = displayName.trim();
    if (!isValidAdminAssetLabel(cleanName)) {
      feedback.textContent = 'etiqueta no válida: entre 1 y 64 caracteres, sin saltos de línea.';
      return;
    }
    btnGuardar.disabled = true;
    feedback.textContent = 'guardando...';
    void safeRun(
      GameAssetAdminService.update(entry.id, { displayName: cleanName, category, isActive }),
      'no se pudo guardar el asset',
    ).then((result) => {
      btnGuardar.disabled = false;
      if (!result.ok) {
        feedback.textContent = 'no se pudo guardar el asset.';
        return;
      }
      showToast('asset actualizado');
      modal.close();
      onSaved();
    });
  });
}

export interface GameSettingsPanel {
  element: HTMLElement;
  destroy: () => void;
}

/* [297A-63] Vista de configuración: reemplaza al juego dentro de la ventana.
 * Tabs para organizar (personajes / assets / actividad); la actividad global
 * agrega también las publicaciones de mapas. Cada tab se monta bajo demanda
 * para no cargar todos los catálogos al abrir. */
export function createGameSettingsPanel(options: { onBack: () => void }): GameSettingsPanel {
  const personajesLista = createEl('div', { className: 'admin-lista' });
  const assetsLista = createEl('div', { className: 'admin-lista' });
  const actividadContenido = createEl('div', { className: 'admin-lista' });

  const btnNuevoPersonaje = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: '+ nuevo personaje',
  });
  btnNuevoPersonaje.addEventListener('click', () => openNuevoPersonajeModal(() => {
    void renderPersonajes(personajesLista);
  }));
  const btnNuevoAsset = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: '+ nuevo asset',
  });
  btnNuevoAsset.addEventListener('click', () => openNuevoAssetModal(() => {
    void renderAssets(assetsLista);
  }));

  const personajes = createEl('section', {},
    createEl('header', { className: 'admin-seccion' },
      createEl('h3', { className: 'mt-lg mb-sm', textContent: 'personajes' }),
      btnNuevoPersonaje,
    ),
    personajesLista,
  );
  const assets = createEl('section', {},
    createEl('header', { className: 'admin-seccion' },
      createEl('h3', { className: 'mt-lg mb-sm', textContent: 'assets' }),
      btnNuevoAsset,
    ),
    assetsLista,
  );

  /* Actividad global: personajes + assets + publicaciones de mapas, cada una
   * aislada (si una falla, las demás siguen). */
  const actividad = createEl('section', {},
    createEl('h3', { className: 'mt-lg mb-sm', textContent: 'actividad' }),
    actividadContenido,
  );

  /* [297A-64] Tab "mapa": editor 2D del Bosque dentro de la misma ventana.
   * Se monta bajo demanda y se destruye al salir del panel (teardown del
   * editor: listeners, ResizeObserver y cargas pendientes). */
  const mapa = createEl('section', {},
    createEl('h3', { className: 'mt-lg mb-sm', textContent: 'editor de mapa' }),
  );

  const paneles = new Map<string, HTMLElement>([
    ['personajes', personajes],
    ['assets', assets],
    ['actividad', actividad],
    ['mapa', mapa],
  ]);
  const activos = new Map<string, boolean>();

  const tabs = createTabs({
    tabs: [
      { id: 'personajes', label: 'personajes' },
      { id: 'assets', label: 'assets' },
      { id: 'actividad', label: 'actividad' },
      { id: 'mapa', label: 'mapa' },
    ],
    initial: 'personajes',
    onSwitch: (id) => {
      for (const [tabId, panel] of paneles) {
        panel.hidden = tabId !== id;
      }
      /* [297A-63] Carga bajo demanda: cada tab monta su contenido una sola vez. */
      if (id === 'personajes' && !activos.get('personajes')) {
        activos.set('personajes', true);
        void renderPersonajes(personajesLista);
      } else if (id === 'assets' && !activos.get('assets')) {
        activos.set('assets', true);
        void renderAssets(assetsLista);
      } else if (id === 'actividad' && !activos.get('actividad')) {
        activos.set('actividad', true);
        void renderActividadGlobal(actividadContenido);
      } else if (id === 'mapa' && !activos.get('mapa')) {
        activos.set('mapa', true);
        /* [297A-64] El editor destruye su runtime al salir del panel: el
         * teardown queda registrado en el WeakMap y se libera en destroy(). */
        const editor = createGameMapEditor(mapa);
        gameMapEditorCleanups.set(mapa, editor.destroy);
      }
    },
  });

  const btnVolver = createEl('button', {
    type: 'button',
    className: 'boton',
    textContent: 'volver al Bosque',
  });
  btnVolver.addEventListener('click', () => options.onBack());

  const header = createEl('header', { className: 'admin-seccion juegoConfig__header' },
    createEl('h2', { className: 'juegoConfig__titulo', textContent: 'configuración del Bosque' }),
    btnVolver,
  );

  const element = createEl('div', { className: 'juegoConfig' },
    header,
    tabs.el,
    ...paneles.values(),
  );

  /* Tab inicial visible desde el montaje. */
  for (const [tabId, panel] of paneles) panel.hidden = tabId !== 'personajes';
  void renderPersonajes(personajesLista);
  activos.set('personajes', true);

  return {
    element,
    /* [297A-63] destroy() también retira el panel del DOM: la vista del juego
     * oculta sus hijos originales y monta este panel como último hijo; al
     * volver, el elemento debe desaparecer o quedaría superpuesto al Bosque
     * rehidratado. El cleanup de generaciones evita que las cargas
     * pendientes toquen un DOM ya desmontado. */
    destroy: () => {
      gameCharacterListGenerations.delete(personajesLista);
      gameAssetListGenerations.delete(assetsLista);
      gameMapEditorCleanups.get(mapa)?.();
      gameMapEditorCleanups.delete(mapa);
      element.remove();
    },
  };
}

/** Carga aislada de las tres actividades en el tab "actividad". */
async function renderActividadGlobal(container: HTMLElement): Promise<void> {
  container.textContent = '';
  const [personajes, assets, mapas] = await Promise.all([
    tryCatch(GameAuditService.listCharacterEvents({ limit: 10 })),
    tryCatch(GameAuditService.listAssetEvents({ limit: 10 })),
    tryCatch(GameAuditService.listMapEvents({ limit: 10 })),
  ]);
  container.appendChild(renderActividad(personajes, 'personajes'));
  container.appendChild(renderActividad(assets, 'assets'));
  container.appendChild(renderActividadMapas(mapas));
}

/** Sección de publicaciones de mapas recientes (últimos eventos auditados). */
function renderActividadMapas(result: { ok: true; value: GameAuditEventEntry[] } | { ok: false; error: string }): HTMLElement {
  const seccion = createEl('section');
  seccion.appendChild(createEl('h4', { className: 'mt-md mb-sm', textContent: 'publicaciones de mapas' }));
  if (!result.ok) {
    seccion.appendChild(createVacio('no se pudo cargar la actividad de mapas'));
    return seccion;
  }
  if (result.value.length === 0) {
    seccion.appendChild(createVacio('sin publicaciones recientes'));
    return seccion;
  }
  for (const event of result.value) {
    const label = ACCION_ETIQUETA[event.action] ?? event.action;
    const version = typeof event.payload?.schemaVersion === 'number' ? ` · v${event.payload.schemaVersion}` : '';
    const info = createEl('div', {},
      createEl('span', { textContent: `${label} · ${event.entityId}${version}` }),
      createEl('small', { className: 'ml-sm', textContent: formatFechaHora(event.createdAt) }),
    );
    seccion.appendChild(createEl('div', { className: 'admin-item' }, info));
  }
  return seccion;
}
