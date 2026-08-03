/* Tests del comando admin-only 'game:settings' [297A-62].
 * Importar el módulo registra el comando (side effect). Verifica:
 * - Oculto para no-admin (fail-closed).
 * - Visible/ejecutable para admin.
 * - Abre el panel de configuración del Bosque (módulo lazy mockeado). */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { CommandRegistry } from '../command-registry';

const openGameSettings = vi.fn();
vi.mock('../../desktop/apps/game-playable/game-settings', () => ({
  openGameSettings,
}));

import './toolbar-commands';

describe('game:settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('está registrado en el registry', () => {
    const cmd = CommandRegistry.get('game:settings');
    expect(cmd).toBeDefined();
    expect(cmd?.label).toBe('Configuración del Bosque');
    expect(cmd?.contexts).toContain('toolbar');
  });

  it('está oculto para invitados y autenticados no-admin (adminOnly)', () => {
    expect(CommandRegistry.isAvailable('game:settings', { capability: 'public' }).state)
      .toBe('hidden');
    expect(CommandRegistry.isAvailable('game:settings', { capability: 'authenticated' }).state)
      .toBe('hidden');
  });

  it('está habilitado para admin', () => {
    expect(CommandRegistry.isAvailable('game:settings', { capability: 'admin' }).state)
      .toBe('enabled');
  });

  it('abre el panel de configuración del Bosque para admin', async () => {
    const result = await CommandRegistry.execute('game:settings', { capability: 'admin' });
    expect(result).toEqual({ status: 'success' });
    expect(openGameSettings).toHaveBeenCalledTimes(1);
  });

  it('no se ejecuta para no-admin aunque el panel esté disponible', async () => {
    const result = await CommandRegistry.execute('game:settings', { capability: 'public' });
    expect(result.status).toBe('failure');
    expect(openGameSettings).not.toHaveBeenCalled();
  });
});
