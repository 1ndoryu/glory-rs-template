/* [174A-2] Lista de sesiones de chat y resolución de títulos.
 * Extraído de SeccionChat.tsx para cumplir SRP (max 300 líneas). */


import {Bot, User} from 'lucide-react';
import {SESSION_STATUS_LABELS, type ChatSession} from '../../api/chat';
import {Button} from '../ui/Button';

/* [154A-14] Resuelve el título de una sesión de chat mostrando el nombre del
 * otro participante en vez de "Orden #N". Staff ve el nombre del cliente,
 * clientes ven el nombre del empleado. Fallback a order_number si no hay nombre.
 * [164A-13] Chat general → Visitante #{id} para diferenciar anónimos. */
export function resolveSessionTitle(s: ChatSession, isStaff: boolean): string {
    if (s.order_id) {
        const name = isStaff ? s.client_name : s.employee_name;
        if (name) return name;
        return `Orden #${s.order_number ?? '...'}`;
    }
    if (s.visitor_name) return s.visitor_name;
    return `Visitante #${s.id.slice(-4).toUpperCase()}`;
}

export function SessionItem({
    session,
    active,
    onClick,
    isStaff,
}: {
    session: ChatSession;
    active: boolean;
    onClick: () => void;
    isStaff: boolean;
}) {
    /* [154A-14] Avatar del otro participante (staff → avatar del cliente, cliente → avatar del empleado) */
    const avatarUrl = isStaff ? session.client_avatar_url : session.employee_avatar_url;

    return (
        <Button
            className={`chatSesionItem ${active ? 'chatSesionActiva' : ''}`}
            onClick={onClick}
            type="button"
            variante="texto"
            tamano="pequeno"
        >
            <div className="chatSesionIcono">
                {avatarUrl
                    ? <img src={avatarUrl} alt="" className="chatSesionAvatar" />
                    : session.ai_enabled ? <Bot size={18} /> : <User size={18} />}
            </div>
            <div className="chatSesionInfo">
                <div className="chatSesionTitulo">
                    {resolveSessionTitle(session, isStaff)}
                </div>
                <div className="chatSesionPreview">
                    {session.last_message || 'Sin mensajes'}
                </div>
            </div>
            <div className="chatSesionMeta">
                <span className="chatSesionEstado">
                    {SESSION_STATUS_LABELS[session.status] || session.status}
                </span>
            </div>
        </Button>
    );
}

/* [237A-5] Agrupa conversaciones sin duplicar la receta visual de cada sesión.
 * Las cerradas conservan acceso desde Historial, pero su lectura se controla en SeccionChat. */
export function SessionGroup({
    title,
    sessions,
    activeSessionId,
    onSelect,
    isStaff,
}: {
    title: string;
    sessions: ChatSession[];
    activeSessionId: string | null;
    onSelect: (sessionId: string) => void;
    isStaff: boolean;
}) {
    if (sessions.length === 0) return null;

    return (
        <section aria-label={title}>
            <div className="chatListaHeader">
                <h4 className="chatListaTitulo">{title}</h4>
            </div>
            {sessions.map(session => (
                <SessionItem
                    key={session.id}
                    session={session}
                    active={session.id === activeSessionId}
                    onClick={() => onSelect(session.id)}
                    isStaff={isStaff}
                />
            ))}
        </section>
    );
}
