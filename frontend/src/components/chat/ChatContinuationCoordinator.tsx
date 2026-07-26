/* [267A-3] Consume el bearer token del fragmento, lo retira de la URL antes de
 * esperar red y monta después el widget con la identidad devuelta por backend. */
import {useEffect} from 'react';

import {apiClaimChatContinuation} from '../../api/chat';
import {useChatStore} from '../../stores/chatStore';
import {toast} from '../../stores/toastStore';
import {restoreChatWidgetIdentity} from '../../utils/chatWidgetStorage';

export function ChatContinuationCoordinator() {
    useEffect(() => {
        if (window.location.pathname !== '/continuar-chat') return;
        const params = new URLSearchParams(window.location.hash.slice(1));
        const token = params.get('token');
        window.history.replaceState({}, '', '/');
        if (!token) {
            toast.error('El enlace para continuar la conversación no es válido.');
            return;
        }
        let active = true;
        void apiClaimChatContinuation(token)
            .then(claim => {
                if (!active) return;
                restoreChatWidgetIdentity(claim.visitor_id, claim.session_id);
                useChatStore.getState().abrir();
                toast.success('Conversación recuperada.');
            })
            .catch(() => {
                if (active) toast.error('El enlace expiró o ya fue utilizado.');
            });
        return () => {
            active = false;
        };
    }, []);
    return null;
}
