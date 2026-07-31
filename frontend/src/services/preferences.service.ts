/* wandori.us — Preferences Service
 * Cliente HTTP de preferencias privadas por cuenta.
 * No conoce themeStore ni decide conflictos: solo transporta el contrato API. */

import { api } from '../api/client';

export type AccountThemeMode = 'system' | 'claro' | 'oscuro';

export interface UserPreferences {
  readonly theme: AccountThemeMode;
  readonly revision: number;
  readonly updated_at: string;
}

export interface UpdateUserPreferencesRequest {
  readonly theme: AccountThemeMode;
  readonly expected_revision: number;
}

export const PreferencesService = {
  async get(): Promise<UserPreferences> {
    return api.get<UserPreferences>('/api/me/preferences');
  },

  async update(request: UpdateUserPreferencesRequest): Promise<UserPreferences> {
    return api.put<UserPreferences>('/api/me/preferences', request);
  },
};
