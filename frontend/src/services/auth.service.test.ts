import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../api/client';
import { authStore } from '../store';
import { AuthService } from './auth.service';

describe('AuthService.login', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    authStore.set({ isAuthenticated: false, userId: null, capability: 'public' }, 'sync');
  });

  it('rechaza si login responde pero /auth/me no confirma la sesión', async () => {
    vi.spyOn(api, 'post').mockResolvedValue(undefined);
    vi.spyOn(api, 'get').mockRejectedValue(new Error('sesión no confirmada'));

    await expect(AuthService.login('user@example.com', 'secret'))
      .rejects.toThrow('La sesión no pudo confirmarse');
    expect(authStore.get().isAuthenticated).toBe(false);
  });
});
