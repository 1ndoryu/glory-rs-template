/* wandori.us — Auth Service
 * Capa de servicio para autenticación.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.post en pages/login.ts.
 * API consistente con otros servicios: errores se propagan como ApiError. */

import { api } from '../api/client';
import { authStore } from '../store';

export interface MeResult {
  isAuthenticated: boolean;
  userId: string | null;
}

export const AuthService = {
  /** Iniciar sesión con email y contraseña.
   *  Lanza ApiError si las credenciales son inválidas (consistente con otros servicios). */
  async login(email: string, password: string): Promise<{ user_id: string }> {
    const res = await api.post<{ user_id: string }>('/api/auth/login', { email, password });
    authStore.set({ isAuthenticated: true, userId: res.user_id });
    return res;
  },

  /** Cerrar sesión. */
  async logout(): Promise<void> {
    try {
      await api.post<void>('/api/auth/logout', {});
    } catch {
      /* Error al cerrar sesión no crítico — limpiar estado igual */
    }
    authStore.set({ isAuthenticated: false, userId: null });
  },

  /** Verificar sesión actual. */
  async me(): Promise<MeResult> {
    try {
      const res = await api.get<{ user_id: string }>('/api/auth/me');
      authStore.set({ isAuthenticated: true, userId: res.user_id });
      return { isAuthenticated: true, userId: res.user_id };
    } catch {
      authStore.set({ isAuthenticated: false, userId: null });
      return { isAuthenticated: false, userId: null };
    }
  },
};
