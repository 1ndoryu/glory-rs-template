/* [259A-5] sentinel-disable-file menu-contextual-override-diseno: variantes por instancia via props API publica del DS; variantes canon al sistema en fase 5b. */
/* [104A-34] Botón de chat en header con dropdown de sesiones activas.
 * Al hacer clic en una sesión, navega al panel → sección mensajes → abre ese chat.
 * Usa sessionStorage para pasar el target a SeccionChat (evita race conditions con mount). */

import {useCallback, useState} from 'react';
import {MessageSquare, Bot, User} from 'lucide-react';
import {useQueryClient} from '@tanstack/react-query';

import {useChat} from '../../hooks/useChat';
import {apiMarkSessionViewed} from '../../api/chat';
import {useAuthStore} from '../../stores/authStore';
import {Button} from '../ui/Button';
import {MenuContextual} from '../ui/ContextMenu';
import './ChatBell.css';

const PANEL_CHAT_TARGET = 'PANEL_CHAT_TARGET';

export default function ChatBell() {
    const {sessions, cargandoSesiones} = useChat();
    const queryClient = useQueryClient();
    const [open, setOpen] = useState(false);
    /* [154A-14] Rol del usuario para resolver títulos de sesión */
    const effectiveRole = useAuthStore(s => s.user?.effectiveRole);
    const isStaff = effectiveRole === 'admin' || effectiveRole === 'employee';

    /* Sesiones activas (no cerradas/archivadas) ordenadas por último mensaje */
    const activeSessions = sessions
        .filter(s => s.status !== 'closed')
        .sort((a, b) => {
            const ta = a.last_message_at ?? a.created_at;
            const tb = b.last_message_at ?? b.created_at;
            return new Date(tb).getTime() - new Date(ta).getTime();
        })
        .slice(0, 15);

    /* [104A-39] Badge = sesiones con mensajes no leídos (last_message_at > last_viewed_at).
     * Si nunca se vio (last_viewed_at null) y tiene mensajes, cuenta como no leída. */
    const unreadCount = activeSessions.filter(s => {
        if (!s.last_message_at) return false;
        if (!s.last_viewed_at) return true;
        return new Date(s.last_message_at) > new Date(s.last_viewed_at);
    }).length;

    const activeCount = activeSessions.length;

    const toggleDropdown = useCallback(() => {
        setOpen(prev => !prev);
    }, []);

    const closeDropdown = useCallback(() => {
        setOpen(false);
    }, []);

    /* [164A-2] Navegar al panel → sección mensajes → chat seleccionado.
     * Marca la sesión como vista DIRECTAMENTE aquí para actualizar el badge inmediatamente,
     * sin depender del roundtrip vía SeccionChat que fallaba por timing. */
    const handleSelectSession = useCallback((sessionId: string) => {
        sessionStorage.setItem(PANEL_CHAT_TARGET, sessionId);
        window.dispatchEvent(new CustomEvent('panel-cambiar-tab', {detail: 'mensajes'}));
        setOpen(false);
        void apiMarkSessionViewed(sessionId)
            .then(() => queryClient.invalidateQueries({queryKey: ['chat-sessions']}))
            .catch(() => { /* no bloquear navegación si falla el mark-viewed */ });
    }, [queryClient]);

    /* Formatear fecha relativa */
    const formatTime = (iso: string | null): string => {
        if (!iso) return '';
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
            ariaLabel={`Chats${activeCount > 0 ? ` (${activeCount} activos)` : ''}`}
            className="chatBell"
            triggerClassName="chatBell__trigger"
            panelClassName="chatBell__dropdown"
            triggerContent={(
                <>
                    <MessageSquare size={20} />
                    {unreadCount > 0 && (
                        <span className="chatBell__badge">
                            {unreadCount > 99 ? '99+' : unreadCount}
                        </span>
                    )}
                </>
            )}
        >
            <div className="chatBell__header">
                <h3 className="chatBell__title">Conversaciones</h3>
            </div>

            <div className="chatBell__list">
                {cargandoSesiones ? (
                    <p className="chatBell__empty">Cargando...</p>
                ) : activeSessions.length === 0 ? (
                    <p className="chatBell__empty">Sin conversaciones activas</p>
                ) : (
                    activeSessions.map(s => (
                        <Button
                            key={s.id}
                            type="button"
                            variante="texto"
                            className="chatBell__item"
                            onClick={() => handleSelectSession(s.id)}
                        >
                            <span className="chatBell__itemIcon">
                                {s.ai_enabled ? <Bot size={16} /> : <User size={16} />}
                            </span>
                            <div className="chatBell__itemContent">
                                <span className="chatBell__itemTitle">
                                    {/* [154A-14] Mostrar nombre del participante en vez de "Orden #N" */}
                                    {s.order_id
                                        ? (isStaff ? s.client_name : s.employee_name) || (s.order_number ? `Orden #${s.order_number}` : 'Chat')
                                        : s.visitor_name || `Visitante #${s.id.slice(-4).toUpperCase()}`}
                                </span>
                                {s.last_message && (
                                    <span className="chatBell__itemPreview">{s.last_message}</span>
                                )}
                                <span className="chatBell__itemTime">
                                    {formatTime(s.last_message_at ?? s.created_at)}
                                </span>
                            </div>
                        </Button>
                    ))
                )}
            </div>
        </MenuContextual>
    );
}
