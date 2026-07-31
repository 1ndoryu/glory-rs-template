import { beforeEach, describe, expect, it, vi } from 'vitest';
import { themeStore } from './theme-store';
import { PreferencesService } from '../../services/preferences.service';
import {
  clearPreferencesSync,
  preferencesSyncStore,
  resolvePreferencesConflict,
  syncPreferencesForUser,
} from './preferences-sync';
import { initPreferencesConflictUI } from './preferences-conflict-ui';

let stop: (() => void) | null = null;

beforeEach(() => {
  vi.restoreAllMocks();
  stop?.();
  stop = initPreferencesConflictUI();
  clearPreferencesSync();
  themeStore.set('claro', 'sync');
  document.body.innerHTML = '';
});

describe('preferences conflict UI', () => {
  it('muestra un único modal aunque el estado conflict se emita varias veces', () => {
    preferencesSyncStore.set({
      userId: 'user-a',
      revision: 4,
      remoteTheme: 'oscuro',
      status: 'conflict',
    }, 'sync');
    preferencesSyncStore.set({
      userId: 'user-a',
      revision: 4,
      remoteTheme: 'oscuro',
      status: 'conflict',
    }, 'sync');

    expect(document.querySelectorAll('.modal-overlay')).toHaveLength(1);
    expect(document.querySelector('.preferences-conflict__message')?.textContent)
      .toContain('preferencia diferente');
  });

  it('permite usar la preferencia remota y cierra el modal', async () => {
    vi.spyOn(PreferencesService, 'get').mockResolvedValue({
      theme: 'oscuro',
      revision: 4,
      updated_at: '2026-07-31T00:00:00.000Z',
    });
    themeStore.set('claro', 'user');
    await syncPreferencesForUser('user-a');

    const remoteButton = document.querySelector<HTMLButtonElement>(
      '[aria-label="Usar la preferencia de la cuenta"]',
    );
    remoteButton?.click();

    expect(themeStore.get()).toBe('oscuro');
    expect(preferencesSyncStore.get().status).toBe('ready');
    expect(document.querySelector('.modal-overlay')).toBeNull();
  });

  it('reemplaza el conflicto cuando cambia la cuenta aunque la revisión coincida', () => {
    preferencesSyncStore.set({
      userId: 'user-a',
      revision: 4,
      remoteTheme: 'oscuro',
      status: 'conflict',
    }, 'sync');

    preferencesSyncStore.set({
      userId: 'user-b',
      revision: 4,
      remoteTheme: 'system',
      status: 'conflict',
    }, 'sync');

    expect(document.querySelectorAll('.modal-overlay')).toHaveLength(1);
    expect(document.querySelector('.preferences-conflict__values')?.textContent)
      .toContain('sistema');
  });

  it('cierra la decisión pendiente al limpiar la cuenta', () => {
    preferencesSyncStore.set({
      userId: 'user-a',
      revision: 4,
      remoteTheme: 'oscuro',
      status: 'conflict',
    }, 'sync');

    clearPreferencesSync();

    expect(document.querySelector('.modal-overlay')).toBeNull();
    expect(preferencesSyncStore.get().status).toBe('idle');
  });

  it('conservar el dispositivo delega en el resolver y mantiene el modal hasta resolver la red', async () => {
    const get = vi.spyOn(PreferencesService, 'get').mockResolvedValue({
      theme: 'oscuro',
      revision: 4,
      updated_at: '2026-07-31T00:00:00.000Z',
    });
    vi.spyOn(PreferencesService, 'update').mockResolvedValue({
      theme: 'claro',
      revision: 5,
      updated_at: '2026-07-31T00:00:00.000Z',
    });
    themeStore.set('claro', 'user');
    await syncPreferencesForUser('user-a');

    expect(get).toHaveBeenCalledOnce();
    expect(preferencesSyncStore.get().status).toBe('conflict');
    const localButton = document.querySelector<HTMLButtonElement>(
      '[aria-label="Conservar la preferencia de este dispositivo"]',
    );
    localButton?.click();

    await vi.waitFor(() => expect(preferencesSyncStore.get().status).toBe('ready'));
    expect(themeStore.get()).toBe('claro');
    expect(document.querySelector('.modal-overlay')).toBeNull();
    resolvePreferencesConflict('remote');
  });
});
