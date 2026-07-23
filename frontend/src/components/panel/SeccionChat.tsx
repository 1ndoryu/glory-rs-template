/* [044A-38 Fase 5] Sección "Mensajes" del panel.
 * Lista de conversaciones + chat activo + panel info visitante (064A-72).
 * Staff: ve todas las sesiones. Cliente: solo sus chats de órdenes.
 * [074A-60] Info visitante solo visible para admin. */

import React, {useRef, useState} from 'react';
import {MessageCircle, Send, Bot, BotOff, ChevronLeft, XCircle, Info, Paperclip, AlertTriangle} from 'lucide-react';
import {SENDER_LABELS} from '../../api/chat';
import {useSeccionChat} from '../../hooks/useSeccionChat';
import {useAuthStore} from '../../stores/authStore';
import {ChatInfoPanel} from './ChatInfoPanel';
import {MessageBubble, resolveSenderToneClass} from './ChatBurbujaMessage';
import {resolveSessionTitle, SessionGroup} from './ChatSessionList';
import {Badge} from '../ui/Badge';
import {Button} from '../ui/Button';
import {Textarea} from '../ui/Textarea';
import './SeccionChat.css';
import './ChatBurbujas.css';

export const SeccionChat: React.FC = () => {
    const {
        activeSessionId,
        sessions,
        messages,
        cargandoSesiones,
        cargandoMensajes,
        enviando,
        input,
        messagesEndRef,
        typingMap,
        visitorOnlineMap,
        showingChat,
        hasOlderMessages,
        setInput,
        selectSession,
        clearActiveSession,
        loadOlderMessages,
        handleKeyDown,
        handleSend,
        handleCloseSession,
        handleUpload,
        uploading,
        closing,
        toggleAi,
        wsSessionAiEnabled,
    } = useSeccionChat();

    const [showInfo, setShowInfo] = useState(false);
    const fileInputRef = useRef<HTMLInputElement>(null);
    const activeSession = sessions.find(s => s.id === activeSessionId) ?? null;
    /* [237A-5] Las cerradas siguen disponibles como historial y su sesión activa es solo lectura. */
    const openSessions = sessions.filter(session => session.status !== 'closed');
    const closedSessions = sessions.filter(session => session.status === 'closed');
    const isActiveSessionClosed = activeSession?.status === 'closed';
    /* [104A-36] Info panel visible para admin y empleados (no solo admin) */
    const effectiveRole = useAuthStore(s => s.user?.effectiveRole);
    const isStaff = effectiveRole === 'admin' || effectiveRole === 'employee';

    /* [104A-40] Info de presencia del visitante en la sesión activa */
    const visitorStatus = activeSessionId ? (visitorOnlineMap[activeSessionId] ?? null) : null;

    if (cargandoSesiones) {
        return (
            <div className="chatLoading">
                <div className="chatSpinner" />
                <p>Cargando conversaciones...</p>
            </div>
        );
    }

    return (
        <div className="chatContenedor">
            {/* Lista de sesiones */}
            <div className={`chatListaSesiones ${showingChat ? 'chatListaOculta' : ''}`}>
                <div className="chatListaHeader">
                    <h3 className="chatListaTitulo">Conversaciones</h3>
                </div>
                {sessions.length === 0 ? (
                    <div className="chatVacio">
                        <MessageCircle size={32} strokeWidth={1.2} />
                        <p>Sin conversaciones</p>
                    </div>
                ) : (
                    <>
                        <SessionGroup
                            title="Activas" sessions={openSessions}
                            activeSessionId={activeSessionId}
                            onSelect={selectSession}
                            isStaff={isStaff}
                        />
                        <SessionGroup
                            title="Historial" sessions={closedSessions}
                            activeSessionId={activeSessionId}
                            onSelect={selectSession}
                            isStaff={isStaff}
                        />
                    </>
                )}
            </div>

            {/* Chat activo */}
            <div className={`chatAreaMensajes ${!showingChat ? 'chatAreaOculta' : ''}`}>
                {activeSessionId ? (
                    <>
                        <div className="chatAreaHeader">
                            <Button
                                className="chatBtnVolver"
                                onClick={clearActiveSession}
                                type="button"
                                variante="texto"
                                tamano="pequeno"
                            >
                                <ChevronLeft size={18} />
                            </Button>
                            <span className="chatAreaTitulo">
                                {activeSession
                                    ? resolveSessionTitle(activeSession, isStaff)
                                    : 'Seleccionar chat'}
                            </span>
                            {isActiveSessionClosed && <Badge label="Solo lectura" />}
                            {/* [104A-40] Indicador de presencia del visitante (staff only) */}
                            {isStaff && visitorStatus && (
                                <span className={`chatVisitorStatus ${visitorStatus.online ? 'chatVisitorOnline' : 'chatVisitorOffline'}`}
                                    title={visitorStatus.online
                                        ? 'Visitante en línea'
                                        : visitorStatus.lastConnectedAt
                                            ? `Última conexión: ${new Date(visitorStatus.lastConnectedAt).toLocaleString('es', {hour: '2-digit', minute: '2-digit', day: '2-digit', month: 'short'})}`
                                            : 'Desconectado'}
                                >
                                    <span className="chatVisitorDot" />
                                    {visitorStatus.online ? 'En línea' : 'Desconectado'}
                                </span>
                            )}
                            {/* [064A-72] Botón para abrir/cerrar panel de info
                             * [104A-36] Visible para admin y empleados — contiene IP, user-agent, notas */}
                            {isStaff && (
                                <Button
                                    className="chatBtnInfo"
                                    onClick={() => setShowInfo(prev => !prev)}
                                    type="button"
                                    variante="texto"
                                    tamano="pequeno"
                                    title="Info del visitante"
                                >
                                    <Info size={18} />
                                </Button>
                            )}
                            {/* [124A-CHAT1] Toggle IA — staff puede activar/desactivar la IA por sesión.
                             * Solo visible en chats generales (no de órdenes).
                             * wsSessionAiEnabled = estado live del WS para feedback inmediato.
                             * [124A-ESC] Cuando hay escalación: icono AlertTriangle antes del toggle
                             * y el botón muestra BotOff para indicar que la IA cedió el control. */}
                            {isStaff && activeSession && !activeSession.order_id && activeSession.status !== 'closed' && (
                                <>
                                    {activeSession.is_escalated && (
                                        <span className="chatEscaladoBadge" title="IA escaló — se requiere intervención humana">
                                            <AlertTriangle size={16} />
                                            Escalado
                                        </span>
                                    )}
                                    <Button
                                        className={`chatBtnToggleAi${(wsSessionAiEnabled ?? activeSession.ai_enabled) && !activeSession.is_escalated ? ' chatBtnToggleAiActivo' : ' chatBtnToggleAiInactivo'}`}
                                        onClick={() => {
                                            const current = wsSessionAiEnabled ?? activeSession.ai_enabled;
                                            toggleAi(activeSessionId!, !current);
                                        }}
                                        type="button"
                                        title={(wsSessionAiEnabled ?? activeSession.ai_enabled) ? 'Desactivar IA' : 'Activar IA'}
                                        variante="texto"
                                        tamano="pequeno"
                                    >
                                        {(wsSessionAiEnabled ?? activeSession.ai_enabled) && !activeSession.is_escalated
                                            ? <Bot size={18} />
                                            : <BotOff size={18} />}
                                    </Button>
                                </>
                            )}
                            {/* [104A-36] Cerrar sesión en BD (staff) — antes solo limpiaba UI.
                             * clearActiveSession desselecciona, handleCloseSession cierra via API. */}
                            {isStaff && activeSession?.status !== 'closed' && (
                                <Button
                                    className="chatBtnCerrar"
                                    onClick={() => void handleCloseSession()}
                                    disabled={closing}
                                    type="button"
                                    title="Cerrar conversación"
                                    variante="texto"
                                    tamano="pequeno"
                                >
                                    <XCircle size={18} />
                                </Button>
                            )}
                        </div>

                        <div className="chatMensajes">
                            {cargandoMensajes && (
                                <div className="chatLoading"><div className="chatSpinner" /></div>
                            )}
                            {/* [074A-43] Botón para cargar mensajes antiguos */}
                            {hasOlderMessages && !cargandoMensajes && (
                                <Button
                                    className="chatBtnCargarAnteriores"
                                    onClick={loadOlderMessages}
                                    type="button"
                                    variante="texto"
                                    tamano="pequeno"
                                >
                                    Cargar anteriores
                                </Button>
                            )}
                            {messages.map(m => (
                                <MessageBubble key={m.id} message={m} />
                            ))}
                            {/* [054A-3] Typing indicator desde WebSocket */}
                            {activeSessionId && typingMap[activeSessionId] && (
                                <div className="chatBurbuja chatBurbujaTyping">
                                    <div className="chatBurbujaHeader">
                                        <span className={resolveSenderToneClass(typingMap[activeSessionId].sender)}>
                                            {SENDER_LABELS[typingMap[activeSessionId].sender] || 'Visitante'} está escribiendo...
                                        </span>
                                    </div>
                                    <div className="chatBurbujaContenido chatTypingDots">
                                        <span /><span /><span />
                                    </div>
                                </div>
                            )}
                            <div ref={messagesEndRef} />
                        </div>

                        {!isActiveSessionClosed && <div className="chatInputArea">
                            {/* [114A-13] Botón adjuntar archivo (staff) */}
                            {isStaff && (
                                <>
                                    <input
                                        ref={fileInputRef}
                                        type="file"
                                        className="chatFileInput"
                                        onChange={e => {
                                            const file = e.target.files?.[0];
                                            if (file) void handleUpload(file);
                                            e.target.value = '';
                                        }}
                                    />
                                    <Button
                                        className="chatBtnAdjuntar"
                                        onClick={() => fileInputRef.current?.click()}
                                        disabled={uploading}
                                        type="button"
                                        variante="texto"
                                        tamano="pequeno"
                                        title="Adjuntar archivo"
                                    >
                                        <Paperclip size={16} />
                                    </Button>
                                </>
                            )}
                            <Textarea
                                className="chatInput"
                                value={input}
                                onChange={e => setInput(e.target.value)}
                                onKeyDown={handleKeyDown}
                                placeholder="Escribe un mensaje..."
                                rows={1}
                            />
                            <Button
                                className="chatBtnEnviar"
                                onClick={() => void handleSend()}
                                disabled={!input.trim() || enviando}
                                type="button"
                                variante="texto"
                                tamano="pequeno"
                            >
                                <Send size={16} />
                            </Button>
                        </div>}
                    </>
                ) : (
                    <div className="chatVacio chatVacioCentrado">
                        <MessageCircle size={48} strokeWidth={1.2} />
                        <p>Selecciona una conversación</p>
                    </div>
                )}
            </div>

            {/* [064A-72] Panel lateral de info del visitante
             * [104A-36] Admin y empleados pueden ver info (IP, user-agent, notas) */}
            {isStaff && showInfo && activeSession && (
                <ChatInfoPanel
                    session={activeSession}
                    onClose={() => setShowInfo(false)}
                />
            )}
        </div>
    );
};

/* [104A-32] MessageBubble, renderMessageContent y resolveSenderToneClass
 * extraidos a ChatBurbujaMessage.tsx para cumplir SRP (max 300 lineas).
 * [174A-2] resolveSessionTitle y SessionItem extraidos a ChatSessionList.tsx. */
