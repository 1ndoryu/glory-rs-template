/* [044A-38 Fase 9] Campana de notificaciones con dropdown.
 * Badge de no leídas, dropdown con lista, marcar leídas, navegar a link. */

import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Bell } from 'lucide-react';

import { NOTIF_TYPES, type NotificationType } from '../../api/notifications';
import { useNotifications } from '../../hooks/useNotifications';
import { buildPanelNotificationTarget } from '../../utils/panelUrlState';
import { Button } from '../ui/Button';
import { MenuContextual } from '../ui/ContextMenu';
import './NotificationBell.css';

export default function NotificationBell() {
  const {
    notifications,
    unreadCount,
    marcarTodasLeidas,
  } = useNotifications();

  /* [237A-7d] WebSocket push ahora se monta globalmente en App.tsx via
   * AuthenticatedNotificationRuntime. NotificationBell solo consume la cache. */
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();

  /* [084A-27] Al abrir el dropdown, marcar todas como leídas automáticamente */
  const toggleDropdown = useCallback(() => {
    setOpen((prev) => {
      const willOpen = !prev;
      if (willOpen && unreadCount > 0) {
        marcarTodasLeidas();
      }
      return willOpen;
    });
  }, [unreadCount, marcarTodasLeidas]);

  const closeDropdown = useCallback(() => {
    setOpen(false);
  }, []);

  const handleNotificationClick = useCallback((target: string | null) => {
    if (!target) return;
    setOpen(false);
    navigate(target);
  }, [navigate]);

  /* Formatear fecha relativa simple */
  const formatTime = (iso: string): string => {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'ahora';
    if (mins < 60) return `hace ${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `hace ${hours}h`;
    const days = Math.floor(hours / 24);
    return `hace ${days}d`;
  };

  return (
    <MenuContextual
      abierto={open}
      onToggle={toggleDropdown}
      onCerrar={closeDropdown}
      ariaLabel={`Notificaciones${unreadCount > 0 ? ` (${unreadCount} sin leer)` : ''}`}
      className="notificationBell"
      triggerClassName="notificationBell__trigger"
      panelClassName="notificationBell__dropdown"
      triggerContent={(
        <>
          <Bell size={20} />
          {unreadCount > 0 && (
            <span className="notificationBell__badge">
              {unreadCount > 99 ? '99+' : unreadCount}
            </span>
          )}
        </>
      )}
    >
      <div className="notificationBell__header">
        <h3 className="notificationBell__title">Notificaciones</h3>
      </div>

      <div className="notificationBell__list">
        {notifications.length === 0 ? (
          <p className="notificationBell__empty">Sin notificaciones</p>
        ) : (
          notifications.map((n) => {
            const typeInfo = NOTIF_TYPES[n.notification_type as NotificationType];
            const target = buildPanelNotificationTarget(n);
            return (
              <Button
                key={n.id}
                type="button"
                variante="texto"
                className={`notificationBell__item ${!n.read ? 'notificationBell__item--unread' : ''}`}
                onClick={() => handleNotificationClick(target)}
                disabled={!target}
              >
                <div className="notificationBell__itemContent">
                  <span className="notificationBell__itemTitle">
                    {typeInfo?.label ?? n.notification_type}: {n.title}
                  </span>
                  {n.body && (
                    <span className="notificationBell__itemBody">{n.body}</span>
                  )}
                  <span className="notificationBell__itemTime">
                    {formatTime(n.created_at)}
                  </span>
                </div>

              </Button>
            );
          })
        )}
      </div>
    </MenuContextual>
  );
}
