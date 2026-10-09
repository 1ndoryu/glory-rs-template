// Chats del asistente Marketplace (07AA-7): hilos con borradores por chat.
// Reutiliza `apiFetch` (JWT + 401); tipos en snake_case como los devuelve
// el backend (`services::marketplace::{ChatResumen, ChatFila}`).

import { apiFetch } from '../inmuebles/api';

/* Fila de `GET /api/admin/marketplace/chats`. */
export interface ChatResumen {
  thread_id: string;
  borradores: number;
  usos: number;
  corregidas: number;
  ultimo: string;
  /* [09AA-19 F7c] Fuente de verdad futura del vínculo (el backend aún no la
   * envía): ausente = sin dato (la vista de huérfanos los muestra todos con
   * aviso). Cuando exista, `true` = hilo con ficha, `false` = huérfano.
   * [09AA-23] El backend ya la envía (siempre presente en filas nuevas);
   * se conserva opcional por compatibilidad con respuestas viejas. */
  aviso_conocido?: boolean;
  /* [09AA-23] Título del inmueble vinculado (ID exacto o título emparejado);
   * ausente/null = huérfano («Sin ficha»). */
  inmueble_vinculado?: string | null;
}

/* Fila de `GET /api/admin/marketplace/chats/:thread`. */
export interface ChatFila {
  excerpt_texto: string;
  respuesta: string;
  usos: number;
  corregida: boolean;
  valida_hasta: string;
}

export function listarChats(): Promise<ChatResumen[]> {
  return apiFetch<ChatResumen[]>('/api/admin/marketplace/chats');
}

export function leerChat(thread: string): Promise<ChatFila[]> {
  return apiFetch<ChatFila[]>(`/api/admin/marketplace/chats/${encodeURIComponent(thread)}`);
}

/* [08AA-39] Limpieza total del panel: borra toda la caché de borradores.
 * [09AA-4] Sin botón en el panel (un solo «Regenerar», lo pidió ella
 * 2026-10-09): queda como cliente del endpoint admin `DELETE /chats`. */
export function limpiarChats(): Promise<{ borrados: number }> {
  return apiFetch<{ borrados: number }>('/api/admin/marketplace/chats', { method: 'DELETE' });
}

/* [09AA-3] Regeneración masiva: una pasada en serie por cada fila con
 * borrador (ver `handlers::marketplace::regenerar_todo`). Salta
 * correcciones de la dueña y filas solo-foto. Tarda ~15s por chat. */
export interface RegenerarTodoFila {
  thread_id: string;
  fuente: string;
}

export interface RegenerarTodoResumen {
  candidatos: number;
  regenerados: number;
  en_reserva: number;
  omitidos: number;
  detalle: RegenerarTodoFila[];
}

export function regenerarTodo(): Promise<RegenerarTodoResumen> {
  return apiFetch<RegenerarTodoResumen>('/api/admin/marketplace/regenerar-todo', { method: 'POST' });
}
