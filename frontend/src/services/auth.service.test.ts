import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as generatedAuth from '../api/generated/auth/auth';
import { authStore } from '../store';
import { AuthService } from './auth.service';

describe('AuthService.login', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    authStore.set({ isAuthenticated: false, userId: null, capability: 'public' }, 'sync');
  });

  it('rechaza si login responde pero /auth/me no confirma la sesión', async () => {
    /* [018A-35] El test verifica el boundary generado, no el cliente manual. */
    vi.spyOn(generatedAuth, 'login').mockResolvedValue({
      data: undefined,
      status: 200,
      headers: new Headers(),
    });
    vi.spyOn(generatedAuth, 'me').mockRejectedValue(new Error('sesión no confirmada'));

    await expect(AuthService.login('user@example.com', 'secret'))
      .rejects.toThrow('La sesión no pudo confirmarse');
    expect(authStore.get().isAuthenticated).toBe(false);
  });
});
