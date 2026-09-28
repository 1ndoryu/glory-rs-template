// Cliente dueña del agente (279A-2 F5): clientes, envío manual, uso y
// auditoría. Reutiliza `apiFetch` (JWT + 401); tipos en snake_case como los
// devuelve el backend (`chat_staff.rs`).

import { API_URL, ErrorApi, apiFetch, leerToken } from '../inmuebles/api';

/* Fila de `GET /api/admin/agent/clientes` (con nº de sesiones). */
export interface ClienteDuena {
  id: string;
  nombre: string | null;
  telefono: string;
  origen: string;
  interes: string | null;
  presupuesto: string | null;
  zona: string | null;
  notas: string | null;
  sesiones: number | null;
  created_at: string;
  updated_at: string;
}

/* Fila de `GET /api/admin/agent/clientes/:id/sesiones`. */
export interface SesionCliente {
  session_id: string;
  canal: string | null;
  telefono: string | null;
  modo: string | null;
  estado_atencion: string | null;
  status: string | null;
  ai_enabled: boolean | null;
  last_body: string | null;
  last_sender: string | null;
  last_at: string | null;
}

/* Fila de `GET /api/admin/agent/uso`. */
export interface UsoDia {
  dia: string | null;
  remitente: string | null;
  mensajes: number | null;
  tokens_est: number | null;
  tokens_in: number | null;
  tokens_out: number | null;
}

/* Fila de `GET /api/admin/agent/auditoria`. */
export interface AuditoriaFila {
  session_id: string;
  extracto: string | null;
  created_at: string;
  status: string | null;
  ai_enabled: boolean | null;
  estado_atencion: string | null;
  modo_atencion: string | null;
  nombre: string | null;
  telefono: string | null;
  ia: number | null;
  humano: number | null;
  visitante: number | null;
}

export function listarClientes(query?: string): Promise<ClienteDuena[]> {
  const q = query?.trim() ? `?query=${encodeURIComponent(query.trim())}&limit=100` : '?limit=100';
  return apiFetch<ClienteDuena[]>(`/api/admin/agent/clientes${q}`);
}

export function crearCliente(datos: { nombre?: string; telefono: string }): Promise<ClienteDuena> {
  return apiFetch('/api/admin/agent/clientes', { method: 'POST', body: JSON.stringify(datos) });
}

export function actualizarCliente(
  id: string,
  cambio: { nombre?: string; interes?: string; presupuesto?: string; zona?: string; notas?: string },
): Promise<ClienteDuena> {
  return apiFetch(`/api/admin/agent/clientes/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(cambio),
  });
}

export function sesionesDeCliente(id: string): Promise<SesionCliente[]> {
  return apiFetch<SesionCliente[]>(`/api/admin/agent/clientes/${encodeURIComponent(id)}/sesiones`);
}

/* Envío manual: con `cliente_id` abre hilo WhatsApp; con `telefono` registra
 * y abre; con `session_id` usa el hilo existente. Encola en outbox. */
export function enviarManual(datos: {
  cliente_id?: string;
  telefono?: string;
  session_id?: string;
  texto: string;
  media_url?: string;
}): Promise<{ ok: boolean; session_id: string; destino: string; outbox_id: string }> {
  return apiFetch('/api/admin/agent/enviar', { method: 'POST', body: JSON.stringify(datos) });
}

export function leerUso(dias = 7): Promise<UsoDia[]> {
  return apiFetch<UsoDia[]>(`/api/admin/agent/uso?dias=${dias}`);
}

export function leerAuditoria(limit = 50): Promise<AuditoriaFila[]> {
  return apiFetch<AuditoriaFila[]>(`/api/admin/agent/auditoria?limit=${limit}`);
}

/* Sesiones Baileys (289A-1): estado para vincular desde la consola.
 * `estado`: iniciando|esperando_qr|abierta|cerrada. */
export interface SesionWhatsapp {
  via: string;
  nombre: string;
  numero: string;
  estado: string;
}

export function sesionesWhatsapp(): Promise<SesionWhatsapp[]> {
  return apiFetch<SesionWhatsapp[]>('/api/admin/agent/whatsapp/sesiones');
}

/* QR pendiente como object URL (`<img src>` no puede mandar el JWT, así
 * que se baja como blob con `apiFetch` manual). 404 = ya vinculada o sin
 * QR pendiente (nunca imagen vacía). El llamador revoca con
 * `URL.revokeObjectURL` al recargar o desmontar. */
export async function descargarQr(canal: string): Promise<string> {
  const token = leerToken();
  const respuesta = await fetch(
    `${API_URL}/api/admin/agent/whatsapp/sesiones/${encodeURIComponent(canal)}/qr`,
    { headers: token ? { Authorization: `Bearer ${token}` } : {} },
  );
  if (respuesta.status === 404) return '';
  if (!respuesta.ok) throw new ErrorApi(`La API devolvió ${respuesta.status}.`, respuesta.status);
  return URL.createObjectURL(await respuesta.blob());
}
