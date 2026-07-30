/* wandori.us — Analytics Service
 * Capa de servicio para analytics y estadísticas.
 * [Auditoría v4 §4.1] — Rompe acoplamiento a api.post en tracker.ts. */

import { api } from '../api/client';
import type { AnalyticsStats } from '../api/types';

export const AnalyticsService = {
  /** Enviar eventos de analytics (batch). */
  async trackEvents(events: Array<{
    event_type: string;
    target_type?: string;
    target_id?: string;
    metadata?: Record<string, unknown>;
  }>): Promise<void> {
    await api.post('/api/analytics/events', { events });
  },

  /** Obtener estadísticas (admin). */
  async getStats(): Promise<AnalyticsStats> {
    return api.get<AnalyticsStats>('/api/admin/analytics/stats');
  },
};
