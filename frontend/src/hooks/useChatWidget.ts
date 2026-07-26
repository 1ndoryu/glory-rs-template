/* [064A-29] Hook para el chat widget de visitantes.
 * Gestiona conexión WebSocket, persistencia de visitor_id/session_id en localStorage,
 * y carga de historial de mensajes al reconectar (via WS — el backend envia history).
 * [T-4] BroadcastChannel para sync entre pestañas del mismo origen.
 * Independiente del hook useChat (que es para staff). */

import {useState, useCallback, useRef, useEffect} from 'react';
import {buildVisitorWsUrl, apiUploadChatFile, type WsServerMessage, type ChatMessage} from '../api/chat';
import {useAuthStore} from '../stores/authStore';
import {handleChatWidgetServerMessage} from './chatWidgetServerMessages';
import {
    clearChatWidgetStorage,
    ensureChatStorageOwner,
    getOrCreateChatVisitorId,
    getSavedChatSessionId,
    loadPersistedChatMessages,
    saveChatSessionId,
} from '../utils/chatWidgetStorage';

const TYPING_THROTTLE_MS = 200;
const BC_CHANNEL_NAME = 'nakomi-chat';
/* [154A-3] Reconexión automática con backoff */
const WS_CONNECT_TIMEOUT_MS = 10_000;
const WS_MAX_RETRIES = 5;
const WS_BASE_BACKOFF_MS = 1_000;
const WS_MAX_BACKOFF_MS = 15_000;

export interface TypingIndicator {
    sender: string;
    content: string;
}

type ChatAuthUser = {
    userId: string;
    role: string;
    effectiveRole: string;
    impersonating: boolean;
} | null;

function buildChatOwnerKey(user: ChatAuthUser): string {
    if (!user) return 'anonymous';
    const impersonating = user.impersonating ? 'impersonating' : 'direct';
    return `user:${user.userId}:role:${user.role}:effective:${user.effectiveRole}:${impersonating}`;
}

export function useChatWidget() {
    const authToken = useAuthStore(s => s.token);
    const authUser = useAuthStore(s => s.user);
    const chatOwnerKey = buildChatOwnerKey(authUser);
    ensureChatStorageOwner(chatOwnerKey);
    const initialSessionId = getSavedChatSessionId();
    const [connected, setConnected] = useState(false);
    const [connecting, setConnecting] = useState(false);
    const [messages, setMessages] = useState<ChatMessage[]>(() => loadPersistedChatMessages(initialSessionId));
    const [typing, setTyping] = useState<TypingIndicator | null>(null);
    const [sessionId, setSessionId] = useState<string | null>(initialSessionId);
    const sessionIdRef = useRef<string | null>(initialSessionId);
    const wsRef = useRef<WebSocket | null>(null);
    const typingTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const lastTypingSentRef = useRef<number>(0);
    /* [T-4] BroadcastChannel para sync entre pestañas */
    const bcRef = useRef<BroadcastChannel | null>(null);
    /* [154A-3] Reconexión automática */
    const retriesRef = useRef(0);
    const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const connectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const intentionalCloseRef = useRef(false);
    const connectInnerRef = useRef<(visitorName?: string, context?: string | null) => void>(() => undefined);
    const lastOwnerKeyRef = useRef(chatOwnerKey);
    /* Guardar últimos args de connect para reconexión */
    const lastConnectArgsRef = useRef<{visitorName?: string; context?: string | null}>({});

    const setActiveSessionId = useCallback((nextSessionId: string) => {
        if (sessionIdRef.current === nextSessionId) return;
        sessionIdRef.current = nextSessionId;
        setSessionId(nextSessionId);
        saveChatSessionId(nextSessionId);
    }, []);

    const resetLocalChatState = useCallback((closeSocket: boolean, ownerKey?: string) => {
        clearChatWidgetStorage(ownerKey);
        sessionIdRef.current = null;
        setMessages([]);
        setSessionId(null);
        setTyping(null);
        setConnecting(false);
        if (reconnectTimerRef.current) {
            clearTimeout(reconnectTimerRef.current);
            reconnectTimerRef.current = null;
        }
        if (connectTimeoutRef.current) {
            clearTimeout(connectTimeoutRef.current);
            connectTimeoutRef.current = null;
        }
        if (typingTimerRef.current) {
            clearTimeout(typingTimerRef.current);
            typingTimerRef.current = null;
        }
        if (closeSocket && wsRef.current) {
            intentionalCloseRef.current = true;
            const ws = wsRef.current;
            ws.onclose = null;
            ws.onerror = null;
            ws.onmessage = null;
            ws.close();
            wsRef.current = null;
            setConnected(false);
        }
    }, []);

    /* [T-4] Procesar un WsServerMessage (reutilizado por WS onmessage y BroadcastChannel) */
    const handleServerMessage = useCallback((msg: WsServerMessage) => {
        if (msg.type === 'session_closed') {
            setConnected(false);
            return;
        }
        handleChatWidgetServerMessage({
            chatOwnerKey,
            setActiveSessionId,
            setMessages,
            setTyping,
            typingTimerRef,
            resetLocalChatState,
        }, msg);
    }, [chatOwnerKey, resetLocalChatState, setActiveSessionId]);

    /* [154A-3] Intento de reconexión con backoff exponencial */
    const scheduleReconnect = useCallback(() => {
        if (intentionalCloseRef.current) return;
        if (retriesRef.current >= WS_MAX_RETRIES) return;
        const delay = Math.min(
            WS_BASE_BACKOFF_MS * Math.pow(2, retriesRef.current),
            WS_MAX_BACKOFF_MS,
        );
        retriesRef.current += 1;
        reconnectTimerRef.current = setTimeout(() => {
            reconnectTimerRef.current = null;
            const args = lastConnectArgsRef.current;
            connectInnerRef.current(args.visitorName, args.context);
        }, delay);
    }, []);

    /* Implementación interna de connect, separada para reutilizar en reconnect */
    const connectInner = useCallback((visitorName?: string, context?: string | null) => {
        if (wsRef.current) return;
        setConnecting(true);

        const visitorId = getOrCreateChatVisitorId(chatOwnerKey);
        /* [T-9][095A-20] Enviar JWT actual si el usuario está autenticado. */
        const url = buildVisitorWsUrl(
            visitorId,
            visitorName,
            authToken,
            context,
            sessionIdRef.current,
        );
        const ws = new WebSocket(url);
        wsRef.current = ws;

        /* [154A-3] Timeout: si no conecta en 10s, abortar e intentar de nuevo */
        connectTimeoutRef.current = setTimeout(() => {
            if (ws.readyState !== WebSocket.OPEN) {
                ws.close();
                wsRef.current = null;
                setConnecting(false);
                scheduleReconnect();
            }
        }, WS_CONNECT_TIMEOUT_MS);

        ws.onopen = () => {
            if (connectTimeoutRef.current) {
                clearTimeout(connectTimeoutRef.current);
                connectTimeoutRef.current = null;
            }
            retriesRef.current = 0;
            setConnected(true);
            setConnecting(false);
            /* [064A-29] El backend envia historial de mensajes automaticamente
             * al reconectar (via WsServerMessage::Message). El hook los recibe
             * en onmessage y deduplica por ID. */
        };

        ws.onclose = () => {
            if (connectTimeoutRef.current) {
                clearTimeout(connectTimeoutRef.current);
                connectTimeoutRef.current = null;
            }
            setConnected(false);
            setConnecting(false);
            wsRef.current = null;
            /* [154A-3] Reconexión automática en cierre inesperado */
            scheduleReconnect();
        };

        ws.onerror = () => {
            /* onclose se dispara después de onerror, la reconexión se maneja ahí */
        };

        ws.onmessage = (event) => {
            try {
                const msg: WsServerMessage = JSON.parse(event.data);
                handleServerMessage(msg);

                /* [T-4] Relayar a BroadcastChannel para otras pestañas */
                if (bcRef.current) {
                    bcRef.current.postMessage(msg);
                }
            } catch {
                /* Mensaje no JSON, ignorar */
            }
        };
    }, [authToken, chatOwnerKey, handleServerMessage, scheduleReconnect]);

    useEffect(() => {
        connectInnerRef.current = connectInner;
    }, [connectInner]);

    useEffect(() => {
        if (lastOwnerKeyRef.current === chatOwnerKey) return;
        lastOwnerKeyRef.current = chatOwnerKey;

        /* [095A-20] Una cuenta/rol efectivo distinto no puede heredar visitor_id,
         * session_id ni mensajes locales de otra identidad del mismo navegador. */
        const hadActiveConnection = Boolean(wsRef.current) || connected || connecting;
        const args = lastConnectArgsRef.current;
        resetLocalChatState(true, chatOwnerKey);
        retriesRef.current = 0;
        intentionalCloseRef.current = false;

        if (hadActiveConnection) {
            connectInnerRef.current(args.visitorName, args.context);
        }
    }, [chatOwnerKey, connected, connecting, resetLocalChatState]);

    const connect = useCallback((visitorName?: string, context?: string | null) => {
        intentionalCloseRef.current = false;
        retriesRef.current = 0;
        lastConnectArgsRef.current = {visitorName, context};
        connectInner(visitorName, context);
    }, [connectInner]);

    const disconnect = useCallback(() => {
        intentionalCloseRef.current = true;
        if (reconnectTimerRef.current) {
            clearTimeout(reconnectTimerRef.current);
            reconnectTimerRef.current = null;
        }
        if (connectTimeoutRef.current) {
            clearTimeout(connectTimeoutRef.current);
            connectTimeoutRef.current = null;
        }
        if (wsRef.current) {
            wsRef.current.close();
            wsRef.current = null;
        }
        if (typingTimerRef.current) {
            clearTimeout(typingTimerRef.current);
            typingTimerRef.current = null;
        }
        setConnected(false);
        setConnecting(false);
        setTyping(null);
    }, []);

    const sendMessage = useCallback((content: string) => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({type: 'message', content}));
            if (content.trim().toLowerCase() === '/reset') {
                intentionalCloseRef.current = true;
                resetLocalChatState(false, chatOwnerKey);
                bcRef.current?.postMessage({type: 'reset'} satisfies WsServerMessage);
            }
        }
    }, [chatOwnerKey, resetLocalChatState]);

    /* [054A-8] Throttle typing a 200ms para no saturar el WS */
    const sendTyping = useCallback((content: string) => {
        const now = Date.now();
        if (now - lastTypingSentRef.current < TYPING_THROTTLE_MS) return;
        lastTypingSentRef.current = now;

        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({type: 'typing', content}));
        }
    }, []);

    /* [T-5] Upload de archivo via REST (multipart). El backend crea el mensaje rico
     * y procesa IA en background (Vision, Whisper, PDF extract). */
    const [uploading, setUploading] = useState(false);
    const uploadFile = useCallback(async (file: File) => {
        if (!sessionId) {
            console.warn('[ChatWidget] Upload ignorado: sesión aún no confirmada');
            return;
        }
        setUploading(true);
        try {
            await apiUploadChatFile(sessionId, file);
            /* El mensaje aparece via WS broadcast — no necesitamos actualizar state local */
        } catch (err) {
            console.error('[ChatWidget] Upload error:', err);
        } finally {
            setUploading(false);
        }
    }, [sessionId]);

    /* Limpiar al desmontar */
    useEffect(() => () => disconnect(), [disconnect]);

    /* [T-4] BroadcastChannel: recibir mensajes de otras pestañas.
     * Cada pestaña abre su propio WS, pero las que aún no se conectaron
     * reciben mensajes por BC. La deduplicación por ID evita duplicados. */
    useEffect(() => {
        if (typeof BroadcastChannel === 'undefined') return;
        const bc = new BroadcastChannel(BC_CHANNEL_NAME);
        bcRef.current = bc;
        bc.onmessage = (event) => {
            try {
                handleServerMessage(event.data as WsServerMessage);
            } catch {
                /* datos inesperados, ignorar */
            }
        };
        return () => {
            bc.close();
            bcRef.current = null;
        };
    }, [handleServerMessage]);

    return {
        connected,
        connecting,
        messages,
        typing,
        sessionId,
        uploading,
        connect,
        disconnect,
        sendMessage,
        sendTyping,
        uploadFile,
    };
}
