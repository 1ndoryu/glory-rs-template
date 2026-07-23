import type React from 'react';
import type {ChatMessage, WsServerMessage} from '../api/chat';
import {playNotificationSound} from '../utils/notificationSound';
import {loadPersistedChatMessages, savePersistedChatMessages} from '../utils/chatWidgetStorage';

export interface ChatWidgetServerMessageDeps {
    chatOwnerKey: string;
    setActiveSessionId: (sessionId: string) => void;
    setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>;
    setTyping: React.Dispatch<React.SetStateAction<{sender: string; content: string} | null>>;
    typingTimerRef: React.MutableRefObject<ReturnType<typeof setTimeout> | null>;
    resetLocalChatState: (closeSocket: boolean, ownerKey?: string) => void;
}

export function handleChatWidgetServerMessage(
    deps: ChatWidgetServerMessageDeps,
    msg: WsServerMessage,
): void {
    switch (msg.type) {
        case 'message':
            handleMessageEvent(deps, msg);
            break;
        case 'typing':
            handleTypingEvent(deps, msg);
            break;
        case 'session_new':
            if (msg.session?.id) {
                const nextSessionId = String(msg.session.id);
                deps.setMessages(prev => (
                    prev.length > 0 && prev.every(m => m.session_id === nextSessionId)
                        ? prev
                        : loadPersistedChatMessages(nextSessionId)
                ));
                deps.setActiveSessionId(nextSessionId);
            }
            break;
        case 'session_closed':
            break;
        case 'reset':
            deps.resetLocalChatState(true, deps.chatOwnerKey);
            break;
        case 'error':
            console.warn('[ChatWidget] Server error:', msg.message);
            break;
    }
}

function handleMessageEvent(deps: ChatWidgetServerMessageDeps, msg: WsServerMessage): void {
    if (!msg.id || !msg.session_id || !msg.content) return;
    const incomingSessionId = msg.session_id;
    deps.setActiveSessionId(incomingSessionId);
    deps.setMessages(prev => {
        const base = prev.every(m => m.session_id === incomingSessionId) ? prev : [];
        if (base.some(m => m.id === msg.id)) return base;
        const next = [
            ...base,
            {
                id: msg.id!,
                session_id: incomingSessionId,
                sender_type: msg.sender || 'unknown',
                sender_id: msg.sender_id ?? null,
                content: msg.content!,
                created_at: msg.created_at || new Date().toISOString(),
                sender_avatar_url: null,
                sender_display_name: null,
                message_type: msg.message_type ?? null,
                metadata: msg.metadata ?? null,
            },
        ];
        savePersistedChatMessages(incomingSessionId, next);
        return next;
    });
    deps.setTyping(null);
    if (deps.typingTimerRef.current) {
        clearTimeout(deps.typingTimerRef.current);
        deps.typingTimerRef.current = null;
    }
    /* [237A-6b] Sonido solo para delivery=live (no historial en reconexión) y remitente no local */
    const isHistory = msg.delivery && msg.delivery !== 'live';
    if (!isHistory && msg.sender !== 'visitor' && msg.sender !== 'client') {
        playNotificationSound();
    }
}

function handleTypingEvent(deps: ChatWidgetServerMessageDeps, msg: WsServerMessage): void {
    if (!msg.sender || msg.sender === 'visitor' || msg.sender === 'client') return;
    deps.setTyping({sender: msg.sender, content: msg.content || ''});
    if (deps.typingTimerRef.current) {
        clearTimeout(deps.typingTimerRef.current);
    }
    deps.typingTimerRef.current = setTimeout(() => {
        deps.setTyping(null);
        deps.typingTimerRef.current = null;
    }, 3000);
}
