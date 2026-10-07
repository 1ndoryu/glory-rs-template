// Cliente staff del chat con IA (169A-5): bandeja, hilo, responder,
// tomar/soltar IA y config. Reutiliza `apiFetch` (JWT + 401) y el tipo
// `MensajeServidor` del cliente visitante: nada duplicado.

import { apiFetch } from '../inmuebles/api';
import type { MensajeServidor } from './cliente-chat';

export type EstadoSesionChat = 'open' | 'escalated' | 'closed';

/* Fila de `GET /api/admin/agent/sesiones` (snake_case del backend). */
export interface ResumenSesion {
  id: string;
  visitor_name: string | null;
  contact: string | null;
  status: string;
  ai_enabled: boolean;
  /* [289A-2] Número del cliente (`canal_sesiones.telefono`): la bandeja y el
   * hilo lo muestran. Null en web sin hilo o sin vincular. */
  telefono: string | null;
  last_body: string | null;
  last_sender: string | null;
  last_at: string | null;
  alertas: number | null;
  updated_at: string;
}

/* Allowlist del backend (`CLAVES_CONFIG` en `chat_staff.rs`; [07AA-1 F4]
 * todo-controlable: números A/B, ventana, autorizados, tope, corte y tono). */
export const CLAVES_CONFIG = [
  'prompt_extra',
  'contacto_telefono',
  'whatsapp_admin',
  'wa_numero_a',
  'wa_numero_b',
  'ai_enabled_global',
  'tools_deshabilitadas',
  'ventana_retraso_min',
  'whatsapp_autorizados',
  'ia_tope_tokens_dia',
  'corte_whatsapp',
  'whatsapp_acuse_texto',
  'whatsapp_fallback_texto',
  'whatsapp_aviso_asesor_texto',
] as const;
export type ClaveConfig = (typeof CLAVES_CONFIG)[number];
export type MapaConfig = Record<ClaveConfig, string | null>;

export function listarSesiones(estado?: EstadoSesionChat): Promise<ResumenSesion[]> {
  const q = estado ? `?estado=${estado}&limit=100` : '?limit=100';
  return apiFetch<ResumenSesion[]>(`/api/admin/agent/sesiones${q}`);
}

/* [309A-3] Página del hilo: `beforeSeq` trae lo anterior a esa secuencia
 * (scroll arriba); sin cursor trae lo último. */
export function historialSesion(id: string, opts?: { limit?: number; beforeSeq?: number }): Promise<MensajeServidor[]> {
  const q = new URLSearchParams();
  q.set('limit', String(opts?.limit ?? 100));
  if (opts?.beforeSeq !== undefined) q.set('before_seq', String(opts.beforeSeq));
  return apiFetch<MensajeServidor[]>(`/api/admin/agent/sesiones/${encodeURIComponent(id)}/historial?${q.toString()}`);
}

/* Responder toma el hilo (backend apaga la IA y marca `escalated`). */
export function responderSesion(id: string, texto: string): Promise<{ ok: boolean; sequence_num: number }> {
  return apiFetch(`/api/admin/agent/sesiones/${encodeURIComponent(id)}/mensajes`, {
    method: 'POST',
    body: JSON.stringify({ body: texto }),
  });
}

export function actualizarSesion(
  id: string,
  cambio: { aiEnabled?: boolean; status?: EstadoSesionChat },
): Promise<ResumenSesion> {
  return apiFetch(`/api/admin/agent/sesiones/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(cambio),
  });
}

export function leerConfig(): Promise<MapaConfig> {
  return apiFetch<MapaConfig>('/api/admin/agent/config');
}

export function guardarConfig(valores: Partial<Record<ClaveConfig, string>>): Promise<{ ok: boolean }> {
  return apiFetch('/api/admin/agent/config', { method: 'PUT', body: JSON.stringify(valores) });
}
