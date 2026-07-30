/* wandori.us — Settings Service
 * Capa de servicio para configuración del sitio y perfil.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.get/post en 5+ archivos. */

import { api } from '../api/client';

export const SettingsService = {
  /** Obtener todas las configuraciones. */
  async getAll(): Promise<Record<string, string>> {
    return api.get<Record<string, string>>('/api/settings');
  },

  /** Guardar configuraciones parciales. */
  async save(settings: Record<string, string>): Promise<void> {
    return api.post<void>('/api/settings', { settings });
  },

  /** Obtener el contenido de About. */
  async getAboutContent(): Promise<string> {
    const s = await api.get<Record<string, string>>('/api/settings');
    return s.about_content || '';
  },

  /** Guardar el contenido de About. */
  async saveAboutContent(content: string): Promise<void> {
    return SettingsService.save({ about_content: content });
  },
};
