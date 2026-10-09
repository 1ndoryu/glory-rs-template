// Logs del puente Marketplace (09AA-5): eventos estructurados del flujo
// borrador/regenerar/IA (`handlers::mp_logs::LogEvento`). Reutiliza
// `apiFetch` (JWT + 401). Sin PII: el hilo viaja como hash-8.

import { apiFetch } from '../inmuebles/api';

export type NivelLog = 'info' | 'warn' | 'error';

/* Evento de `GET /api/admin/marketplace/logs` (recientes-primero). */
export interface LogEvento {
  id: number;
  ts: string;
  nivel: NivelLog;
  evento: string;
  estado: string;
  mensaje: string;
  campos: Record<string, unknown>;
}

export function listarLogs(nivel?: NivelLog, limite = 200): Promise<LogEvento[]> {
  const q = new URLSearchParams({ limite: String(limite) });
  if (nivel) q.set('nivel', nivel);
  return apiFetch<{ eventos: LogEvento[] }>(`/api/admin/marketplace/logs?${q.toString()}`).then(
    (r) => r.eventos,
  );
}
