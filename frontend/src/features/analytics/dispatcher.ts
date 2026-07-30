/* wandori.us — Analytics Dispatcher
 * Catálogo tipado de eventos del OS.
 * Wrapper sobre el tracker existente que garantiza contratos tipados.
 * No emite por cada pointermove; eventos críticos reservados al backend. */

import { trackPageView } from './tracker';

/** Catálogo de eventos del OS. Cada tipo tiene metadata tipada. */
export type TrackEvent =
  | { type: 'page_view'; path: string }
  | { type: 'app_opened'; appId: string }
  | { type: 'app_closed'; appId: string }
  | { type: 'window_focused'; appId: string }
  | { type: 'session_start'; userId: string | null }
  | { type: 'session_end' };

/** Cola interna para batching futuro. */
const queue: TrackEvent[] = [];
const MAX_QUEUE_SIZE = 50;

/**
 * Despacha un evento tipado al sistema de analytics.
 * Por ahora delega al tracker existente; en 297A-16 se conectará al pipeline batch.
 */
export function dispatchEvent(event: TrackEvent): void {
  /* Evitar cola infinita */
  if (queue.length >= MAX_QUEUE_SIZE) {
    queue.splice(0, queue.length - MAX_QUEUE_SIZE + 1);
  }
  queue.push(event);

  /* Delegar al tracker existente según tipo */
  switch (event.type) {
    case 'page_view':
      trackPageView(event.path);
      break;
    case 'app_opened':
    case 'app_closed':
    case 'window_focused':
    case 'session_start':
    case 'session_end':
      /* Placeholder: en 297A-16 se conectará al pipeline batch.
       * Por ahora solo acumula en cola para no perder eventos. */
      break;
  }
}

/** Obtener eventos acumulados (para debug/testing). */
export function getQueuedEvents(): readonly TrackEvent[] {
  return queue;
}

/** Limpiar cola (para testing). */
export function clearQueue(): void {
  queue.length = 0;
}
