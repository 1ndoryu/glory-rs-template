/* wandori.us — Preferences Conflict UI
 * Adaptador de presentación para el estado conflict del sync de preferencias.
 * No conoce HTTP ni modifica el store directamente: solo presenta la decisión
 * y delega en resolvePreferencesConflict(). */

import { createModal, type ModalOptions } from '../../components/ui/modal';
import { createEl } from '../../utils/dom';
import { themeStore } from './theme-store';
import {
  preferencesSyncStore,
  resolvePreferencesConflict,
  type PreferencesSyncState,
} from './preferences-sync';

let activeModal: { close: () => void } | null = null;
let activeUserId: string | null = null;
let activeRevision: number | null = null;
let stopSubscription: (() => void) | null = null;
let sharedCleanup: (() => void) | null = null;

function themeLabel(theme: PreferencesSyncState['remoteTheme']): string {
  if (theme === 'claro') return 'claro';
  if (theme === 'oscuro') return 'oscuro';
  return 'sistema';
}

function closeActiveModal(): void {
  activeModal?.close();
  activeModal = null;
  activeUserId = null;
  activeRevision = null;
}

function openConflictModal(state: PreferencesSyncState): void {
  if (state.remoteTheme === null || state.revision === null) return;
  if (activeModal && activeUserId === state.userId && activeRevision === state.revision) return;

  closeActiveModal();

  const localTheme = themeStore.get();
  const title = createEl('h2', {
    className: 'preferences-conflict__title',
    textContent: 'preferencia actualizada',
  });
  const message = createEl('p', {
    className: 'preferences-conflict__message',
    textContent: 'Esta cuenta tiene una preferencia diferente. Elige cuál conservar.',
  });
  const values = createEl('dl', { className: 'preferences-conflict__values' });
  values.append(
    createEl('dt', { textContent: 'en este dispositivo' }),
    createEl('dd', { textContent: themeLabel(localTheme) }),
    createEl('dt', { textContent: 'en tu cuenta' }),
    createEl('dd', { textContent: themeLabel(state.remoteTheme) }),
  );

  const keepLocal = createEl('button', {
    className: 'boton preferences-conflict__action',
    type: 'button',
    textContent: 'conservar dispositivo',
    'aria-label': 'Conservar la preferencia de este dispositivo',
  });
  const useRemote = createEl('button', {
    className: 'boton preferences-conflict__action',
    type: 'button',
    textContent: 'usar preferencia de cuenta',
    'aria-label': 'Usar la preferencia de la cuenta',
  });
  const actions = createEl('div', { className: 'preferences-conflict__actions' }, keepLocal, useRemote);
  const content = createEl('section', {
    className: 'preferences-conflict',
  }, title, message, values, actions);
  title.id = 'preferences-conflict-title';

  const options: ModalOptions = {
    contenido: content,
    ancho: '440px',
    closeOnBackdrop: false,
    closeOnEscape: false,
    ariaLabelledby: 'preferences-conflict-title',
  };
  activeUserId = state.userId;
  activeRevision = state.revision;
  activeModal = createModal(options);

  keepLocal.addEventListener('click', () => {
    resolvePreferencesConflict('local');
  });
  useRemote.addEventListener('click', () => {
    resolvePreferencesConflict('remote');
  });
  keepLocal.focus();
}

function render(state: PreferencesSyncState): void {
  if (state.status === 'conflict') {
    openConflictModal(state);
  } else {
    closeActiveModal();
  }
}

/** Montar una única superficie visual para conflictos de preferencias. */
export function initPreferencesConflictUI(): () => void {
  if (sharedCleanup) return sharedCleanup;

  stopSubscription = preferencesSyncStore.subscribe((state) => {
    render(state);
  });

  let cleaned = false;
  const cleanup = (): void => {
    if (cleaned) return;
    cleaned = true;
    stopSubscription?.();
    stopSubscription = null;
    sharedCleanup = null;
    closeActiveModal();
  };
  sharedCleanup = cleanup;
  return cleanup;
}
