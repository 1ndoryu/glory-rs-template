/* wandori.us — Auth Service
 * Capa de servicio para autenticación.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.post en pages/login.ts.
 * API consistente con otros servicios: errores se propagan como ApiError. */

import { api } from '../api/client';
import { authStore, type AuthCapability } from '../store';
import { clearPreferencesSync, syncPreferencesForUser } from '../features/runtime/preferences-sync';
import { clearClipboard } from '../features/runtime/workspace/workspace-store';

export interface MeResult {
  isAuthenticated: boolean;
  userId: string | null;
  capability: AuthCapability;
}

interface MeResponse {
  id: string;
  role?: 'user' | 'admin';
}

function capabilityFromRole(role?: MeResponse['role']): AuthCapability {
  return role === 'admin' ? 'admin' : 'authenticated';
}

export const AuthService = {
  /** Crear cuenta: la sesión solo se habilita después de verificar el correo. */
  async register(email: string, password: string): Promise<{ message: string }> {
    return api.post<{ message: string }>('/api/auth/register', { email, password });
  },

  async verifyEmail(token: string): Promise<{ message: string }> {
    return api.post<{ message: string }>('/api/auth/verify-email', { token });
  },

  async requestPasswordReset(email: string): Promise<{ message: string }> {
    return api.post<{ message: string }>('/api/auth/password-reset', { email });
  },

  async resetPassword(token: string, password: string): Promise<void> {
    return api.post<void>('/api/auth/password-reset/confirm', { token, password });
  },

  /** Iniciar sesión con email y contraseña.
   *  Lanza ApiError si las credenciales son inválidas (consistente con otros servicios). */
  async login(email: string, password: string): Promise<void> {
    /* El endpoint de sesión responde 204; la capacidad se confirma en /me. */
    await api.post<void>('/api/auth/login', { email, password });
    const session = await this.me();
    if (!session.isAuthenticated) {
      throw new Error('La sesión no pudo confirmarse');
    }
  },

  /** Cerrar sesión. */
  async logout(): Promise<void> {
    try {
      await api.post<void>('/api/auth/logout', {});
    } catch (error) {
      /* La cookie puede haber expirado; el estado local se limpia igual, pero
       * el fallo queda observable para diagnóstico y no se silencia. */
      console.error('[auth] logout request failed', error);
    }
    clearPreferencesSync();
    clearClipboard();
    authStore.set({ isAuthenticated: false, userId: null, capability: 'public' });
  },

  /** Verificar sesión actual. */
  async me(): Promise<MeResult> {
    try {
      const res = await api.get<MeResponse>('/api/auth/me');
      const capability = capabilityFromRole(res.role);
      authStore.set({ isAuthenticated: true, userId: res.id, capability });
      await syncPreferencesForUser(res.id);
      return { isAuthenticated: true, userId: res.id, capability };
    } catch {
      clearPreferencesSync();
      authStore.set({ isAuthenticated: false, userId: null, capability: 'public' });
      return { isAuthenticated: false, userId: null, capability: 'public' };
    }
  },
};
