/* wandori.us — Auth Service
 * Capa de servicio para autenticación.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.post en pages/login.ts.
 * API consistente con otros servicios: errores se propagan como ApiError. */

import { api } from '../api/client';
import { authStore, type AuthCapability } from '../store';
import { clearPreferencesSync, syncPreferencesForUser } from '../features/runtime/preferences-sync';

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
  /** Iniciar sesión con email y contraseña.
   *  Lanza ApiError si las credenciales son inválidas (consistente con otros servicios). */
  async login(email: string, password: string): Promise<void> {
    /* El endpoint de sesión responde 204; la capacidad se confirma en /me. */
    await api.post<void>('/api/auth/login', { email, password });
    await this.me();
  },

  /** Cerrar sesión. */
  async logout(): Promise<void> {
    try {
      await api.post<void>('/api/auth/logout', {});
    } catch {
      /* Error al cerrar sesión no crítico — limpiar estado igual */
    }
    clearPreferencesSync();
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
