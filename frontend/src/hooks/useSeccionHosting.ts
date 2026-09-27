/* [074A-63] Hook de estado para SeccionHosting.
 * [094A-2] Ahora incluye selectedHostingId para navegación lista→detalle.
 * [094A-3] Mutations extraídas a useHostingMutations para cumplir límite 120 líneas.
 * 3 useState: showCreateModal, tabActiva, selectedHostingId. */

import {useState, useMemo, useEffect, useCallback} from 'react';
import {useQuery} from '@tanstack/react-query';
import {useLocation} from 'react-router-dom';
import {apiListHostingSubscriptions, apiListVpsSubscriptions} from '../api/hosting';
import {toast} from '../stores/toastStore';
import {useAuthStore} from '../stores/authStore';
import {useHostingMutations} from './useHostingMutations';
import {useVpsMutations} from './useVpsMutations';
import {useBillingItems} from './useBillingItems';
import {getPanelHostingIdFromUrl, syncPanelHostingInUrl} from '../utils/panelUrlState';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {leerParametrosBusqueda, obtenerHref, reemplazarUrl} from '../platform/navigation';

const ACTIVE_STATUSES = new Set(['pending', 'provisioning', 'active']);

export function useSeccionHosting() {
    const [showCreateModal, setShowCreateModal] = useState(false);
    /* [304A-1] 'deployments' y 'servidores' eliminados — movidos a SeccionInfraestructura */
    const [tabActiva, setTabActiva] = useState<'activos' | 'inactivos' | 'vps'>('activos');
    const [selectedHostingId, setSelectedHostingId] = useState<string | null>(() => getPanelHostingIdFromUrl());
    const location = useLocation();

    const effectiveRole = useAuthStore(s => s.user?.effectiveRole) ?? 'client';
    const isAdmin = effectiveRole === 'admin';

    /* [084A-24] Detectar retorno de Stripe Checkout (?hosting=success/cancelled).
     * [245A-11] El bypass de test vuelve con `hosting=test-bypass&subscription_id=...`;
     * mostrar feedback y dejar que panelUrlState canonice a `hostingId`. */
    useEffect(() => {
        const params = leerParametrosBusqueda();
        const hostingResult = params.get('hosting');
        if (hostingResult === 'success') {
            toast.success('¡Pago completado! Tu hosting se activará en breve.');
        } else if (hostingResult === 'test-bypass') {
            toast.success('Hosting de prueba activado. Ya puedes revisar sus accesos y estado.');
        } else if (hostingResult === 'cancelled') {
            toast.warning('Checkout cancelado. Puedes intentarlo de nuevo.');
        }
        const vpsResult = params.get('vps');
        if (vpsResult === 'success') {
            toast.success('Pago VPS confirmado. Tu solicitud quedó pendiente de aprobación.');
        } else if (vpsResult === 'cancelled') {
            toast.warning('Checkout VPS cancelado. Puedes intentarlo de nuevo.');
        }
        if (hostingResult) {
            const url = new URL(obtenerHref());
            url.searchParams.delete('hosting');
            url.searchParams.delete('session_id');
            reemplazarUrl(url.pathname + url.search);
        }
        if (vpsResult) {
            const url = new URL(obtenerHref());
            url.searchParams.delete('vps');
            url.searchParams.delete('session_id');
            reemplazarUrl(url.pathname + url.search);
        }
    }, []);

    useEffect(() => {
        const hostingId = getPanelHostingIdFromUrl();
        if (hostingId !== selectedHostingId) {
            setSelectedHostingId(hostingId);
        }
    }, [location.search, selectedHostingId]);

    useEffect(() => {
        const currentSection = new URLSearchParams(location.search).get('seccion');
        if (currentSection && currentSection !== 'hosting') {
            return;
        }
        /* [155A-10] Mantener URL y estado sincronizados solo despues de que los handlers
         * limpian/escriben la URL primero; evita el parpadeo detalle -> lista -> detalle. */
        syncPanelHostingInUrl(selectedHostingId);
    }, [selectedHostingId, location.search]);

    const handleSelectHosting = useCallback((hostingId: string) => {
        syncPanelHostingInUrl(hostingId);
        setSelectedHostingId(hostingId);
    }, []);

    const handleVolverHosting = useCallback(() => {
        syncPanelHostingInUrl(null);
        setSelectedHostingId(null);
    }, []);

    const hostingKey = ['hosting-subscriptions', effectiveRole] as const;
    const vpsKey = ['vps-subscriptions', effectiveRole] as const;

    const {data: subscriptions = [], isLoading: hostingLoading} = useQuery({
        queryKey: hostingKey,
        queryFn: apiListHostingSubscriptions,
    });

    const {data: vpsSubscriptions = [], isLoading: vpsLoading} = useQuery({
        queryKey: vpsKey,
        queryFn: apiListVpsSubscriptions,
    });
    const {
        billingItems,
        isLoading: billingLoading,
        checkoutMutation: billingCheckoutMutation,
    } = useBillingItems();

    const activos = useMemo(
        () => subscriptions.filter(s => ACTIVE_STATUSES.has(s.status)),
        [subscriptions],
    );
    const inactivos = useMemo(
        () => subscriptions.filter(s => !ACTIVE_STATUSES.has(s.status)),
        [subscriptions],
    );
    const listaActual = tabActiva === 'activos' ? activos : inactivos;
    const isLoading = hostingLoading || vpsLoading;

    const onCreateSuccess = useCallback(() => setShowCreateModal(false), []);
    const mutations = useHostingMutations(hostingKey, onCreateSuccess);
    const vpsMutations = useVpsMutations(vpsKey);

    return {
        subscriptions,
        vpsSubscriptions,
        isLoading,
        billingItems,
        billingLoading,
        billingCheckoutMutation,
        isAdmin,
        tabActiva,
        setTabActiva,
        activos,
        inactivos,
        listaActual,
        showCreateModal,
        setShowCreateModal,
        selectedHostingId,
        handleSelectHosting,
        handleVolverHosting,
        ...mutations,
        ...vpsMutations,
    };
}
