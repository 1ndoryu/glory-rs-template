/* [064A-31] Hook para chat dentro de un pedido.
 * [20CA-7] Conecta a WebSocket para recibir mensajes en tiempo real.
 * El polling REST (5s) se mantiene como fallback; el WS invalida la cache
 * para que React Query actualice la UI instantáneamente.
 * [20CA-9] Marca sesión como vista al abrir para tracking de no leídos. */

import {useState, useCallback, useRef, useEffect} from 'react';
import {useQuery, useMutation, useQueryClient} from '@tanstack/react-query';
import {
    apiCreateChatSession,
    apiGetMessages,
    apiSendMessage,
    apiMarkSessionViewed,
    buildVisitorWsUrl,
    type ChatSession,
    type WsServerMessage,
} from '../api/chat';
import {useAuthStore} from '../stores/authStore';

export function useOrderChat(orderId: string) {
    const queryClient = useQueryClient();
    const token = useAuthStore(s => s.token);
    /* [237A-5] userId es la identidad canónica persistida por authStore. */
    const userId = useAuthStore(s => s.user?.userId);
    const [sessionId, setSessionId] = useState<string | null>(null);
    const [session, setSession] = useState<ChatSession | null>(null);
    const [creando, setCreando] = useState(false);
    const inicializado = useRef(false);
    const wsRef = useRef<WebSocket | null>(null);

    /* Crear o recuperar sesión al montar */
    const iniciarSesion = useCallback(async () => {
        if (inicializado.current || creando) return;
        inicializado.current = true;
        setCreando(true);
        try {
            const s = await apiCreateChatSession(orderId);
            setSessionId(s.id);
            setSession(s);
            /* [20CA-9] Marcar como vista al iniciar */
            apiMarkSessionViewed(s.id).catch(() => {});
        } catch (err) {
            console.error('Error creando sesión de chat de orden:', err);
            inicializado.current = false;
        } finally {
            setCreando(false);
        }
    }, [orderId, creando]);

    /* [20CA-7] Conectar WebSocket cuando hay sesión activa */
    useEffect(() => {
        if (!sessionId || !token || !userId) return;
        /* Cerrar WS previo si existe */
        if (wsRef.current) {
            wsRef.current.close();
            wsRef.current = null;
        }
        const url = buildVisitorWsUrl(userId, undefined, token, `order:${orderId}`);
        const ws = new WebSocket(url);
        wsRef.current = ws;

        ws.onmessage = (event) => {
            try {
                const msg: WsServerMessage = JSON.parse(event.data);
                if (msg.type === 'message' && msg.session_id === sessionId) {
                    /* Invalidar cache para que React Query refresque */
                    queryClient.invalidateQueries({queryKey: ['order-chat-messages', sessionId]});
                }
            } catch { /* ignorar mensajes malformados */ }
        };

        ws.onclose = () => { wsRef.current = null; };
        return () => { ws.close(); wsRef.current = null; };
    }, [sessionId, token, userId, orderId, queryClient]);

    /* Polling de mensajes cada 5s como fallback (WS puede perder mensajes) */
    const {data: mensajes = []} = useQuery({
        queryKey: ['order-chat-messages', sessionId],
        queryFn: () => apiGetMessages(sessionId!, 100, 0),
        enabled: !!sessionId,
        refetchInterval: 5_000,
    });

    /* Enviar mensaje */
    const {mutateAsync: enviarMensajeAsync, isPending: enviando} = useMutation({
        mutationFn: (content: string) => apiSendMessage(sessionId!, content),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: ['order-chat-messages', sessionId]});
        },
    });

    const enviarMensaje = useCallback(
        async (content: string) => {
            if (!sessionId || !content.trim()) return;
            await enviarMensajeAsync(content.trim());
        },
        [sessionId, enviarMensajeAsync],
    );

    /* [20CA-9] Marcar como vista al hacer focus en la sesión */
    const marcarVista = useCallback(() => {
        if (sessionId) {
            apiMarkSessionViewed(sessionId).catch(() => {});
        }
    }, [sessionId]);

    return {
        sessionId,
        session,
        mensajes,
        enviando,
        creando,
        iniciarSesion,
        enviarMensaje,
        marcarVista,
    };
}
