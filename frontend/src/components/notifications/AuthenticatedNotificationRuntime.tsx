/* [237A-7d] Runtime global de notificaciones autenticadas.
 * Componente sin UI que monta la conexión WebSocket de notificaciones
 * una sola vez para toda la sesión autenticada (público + panel).
 * Extraído de NotificationBell para que funcione fuera del panel también.
 *
 * Se monta en DeferredGlobalWidgets (App.tsx) — solo cuando hay token. */

import { useNotificationWs } from '../../hooks/useNotificationWs';
import { useAuthStore } from '../../stores/authStore';

export function AuthenticatedNotificationRuntime() {
  const token = useAuthStore((s) => s.token);
  /* Solo conectar si hay usuario autenticado */
  if (!token) return null;
  return <NotificationWsConnector />;
}

function NotificationWsConnector() {
  useNotificationWs();
  return null;
}
