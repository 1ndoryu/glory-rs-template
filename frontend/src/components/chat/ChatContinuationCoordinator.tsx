/* [267A-3] Consume el bearer token del fragmento, lo retira de la URL antes de
 * esperar red y monta después el widget con la identidad devuelta por backend.
 * [289A-1] El token es de un solo uso y StrictMode re-ejecuta este efecto en
 * dev: sin guard module-level la primera ejecución quema el token y la
 * limpieza invalida `active`, con lo que la identidad nunca se restaura.
 * `tokenEnProceso` canjea cada token una sola vez; la restauración corre
 * dentro de la promesa compartida (store global + toast son seguros aun si
 * el componente se desmontó; aquí no hay setState que proteger). */
import {useEffect} from 'react';

import {apiClaimChatContinuation} from '../../api/chat';
import {useChatStore} from '../../stores/chatStore';
import {toast} from '../../stores/toastStore';
import {restoreChatWidgetIdentity} from '../../utils/chatWidgetStorage';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {obtenerFragmento, obtenerRuta, reemplazarUrl} from '../../platform/navigation';

let tokenEnProceso: string | null = null;

export function ChatContinuationCoordinator() {
    useEffect(() => {
        if (obtenerRuta() !== '/continuar-chat') return;
        const params = new URLSearchParams(obtenerFragmento().slice(1));
        const token = params.get('token');
        reemplazarUrl('/');
        if (!token) {
            toast.error('El enlace para continuar la conversación no es válido.');
            return;
        }
        if (tokenEnProceso === token) return;
        tokenEnProceso = token;
        void apiClaimChatContinuation(token)
            .then(claim => {
                restoreChatWidgetIdentity(claim.visitor_id, claim.session_id);
                useChatStore.getState().abrir();
                toast.success('Conversación recuperada.');
            })
            .catch(() => {
                toast.error('El enlace expiró o ya fue utilizado.');
            })
            .finally(() => {
                if (tokenEnProceso === token) tokenEnProceso = null;
            });
    }, []);
    return null;
}
