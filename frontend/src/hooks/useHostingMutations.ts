/* [094A-3] Mutations de hosting extraídas de useSeccionHosting para cumplir límite 120 líneas.
 * Contiene todas las mutaciones CRUD + checkout + self-service subscribe.
 * [304A-3] Agrega assignMutation (admin asigna hosting a cliente por email) y
 *          adminCheckoutMutation (genera link de pago sin redirigir al admin). */

import {useMutation, useQueryClient} from '@tanstack/react-query';
import {
    apiCreateHostingSubscription,
    apiUpdateHostingStatus,
    apiUpdateHostingSubscription,
    apiDeleteHostingSubscription,
    apiRequestCancelHosting,
    apiCreateHostingCheckout,
    apiAssignHostingToUser,
    apiSelfSubscribe,
    apiProvisionHosting,
    apiRestartHosting,
    apiStopHosting,
    apiStartHosting,
    apiAdminTestSubscribe,
    type CreateHostingRequest,
    type UpdateHostingRequest,
    type SelfSubscribeRequest,
} from '../api/hosting';
import {toast} from '../stores/toastStore';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {redirigir} from '../platform/navigation';

export function useHostingMutations(
    hostingKey: readonly string[],
    onCreateSuccess: () => void,
) {
    const queryClient = useQueryClient();

    const createMutation = useMutation({
        mutationFn: (req: CreateHostingRequest) => apiCreateHostingSubscription(req),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Suscripción creada');
            onCreateSuccess();
        },
        onError: () => toast.error('Error al crear suscripción'),
    });

    const statusMutation = useMutation({
        mutationFn: ({id, status}: {id: string; status: string}) =>
            apiUpdateHostingStatus(id, status),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Status actualizado');
        },
        onError: () => toast.error('Error al actualizar status'),
    });

    const updateMutation = useMutation({
        mutationFn: ({id, req}: {id: string; req: UpdateHostingRequest}) =>
            apiUpdateHostingSubscription(id, req),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Suscripción actualizada');
        },
        onError: () => toast.error('Error al actualizar suscripción'),
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => apiDeleteHostingSubscription(id),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Suscripción eliminada');
        },
        onError: () => toast.error('Error al eliminar suscripción'),
    });

    const cancelMutation = useMutation({
        mutationFn: (id: string) => apiRequestCancelHosting(id),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Suscripción cancelada');
        },
        onError: () => toast.error('Error al cancelar suscripción'),
    });

    const checkoutMutation = useMutation({
        mutationFn: (id: string) => apiCreateHostingCheckout(id),
        onSuccess: (checkoutUrl) => {
            redirigir(checkoutUrl);
        },
        onError: () => toast.error('Error al iniciar checkout'),
    });

    const subscribeMutation = useMutation({
        mutationFn: (req: SelfSubscribeRequest) => apiSelfSubscribe(req),
        onSuccess: (resp) => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Suscripción creada. Redirigiendo al pago…');
            redirigir(resp.checkout_url);
        },
        onError: () => toast.error('Error al contratar hosting'),
    });

    /* [154A-11] Provisionar hosting real en Coolify (admin only) */
    const provisionMutation = useMutation({
        mutationFn: (id: string) => apiProvisionHosting(id),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Hosting provisionado correctamente');
        },
        onError: () => toast.error('Error al provisionar hosting'),
    });

    /* [154A-9] Control de servicio */
    const restartMutation = useMutation({
        mutationFn: (id: string) => apiRestartHosting(id),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('WordPress reiniciado');
        },
        onError: () => toast.error('Error al reiniciar WordPress'),
    });

    const stopMutation = useMutation({
        mutationFn: (id: string) => apiStopHosting(id),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('WordPress detenido');
        },
        onError: () => toast.error('Error al detener WordPress'),
    });

    const startMutation = useMutation({
        mutationFn: (id: string) => apiStartHosting(id),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('WordPress iniciado');
        },
        onError: () => toast.error('Error al iniciar WordPress'),
    });

    /* [154A-14] Admin test subscribe (sin Stripe) */
    const adminTestSubscribeMutation = useMutation({
        mutationFn: (req: SelfSubscribeRequest) => apiAdminTestSubscribe(req),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success('Suscripción de prueba creada');
            onCreateSuccess();
        },
        onError: () => toast.error('Error al crear suscripción de prueba'),
    });

    /* [304A-3] Admin asigna hosting a cliente registrado por email */
    const assignMutation = useMutation({
        mutationFn: ({id, email}: {id: string; email: string}) =>
            apiAssignHostingToUser(id, email),
        onSuccess: (updated) => {
            queryClient.invalidateQueries({queryKey: hostingKey});
            toast.success(`Hosting asignado a ${updated.client_email}`);
        },
        onError: (err: unknown) => {
            const msg = err instanceof Error ? err.message : 'Error al asignar hosting';
            toast.error(msg);
        },
    });

    /* [304A-3] Admin genera link de pago: copia URL al portapapeles en vez de redirigir */
    const adminCheckoutMutation = useMutation({
        mutationFn: (id: string) => apiCreateHostingCheckout(id),
        onSuccess: (checkoutUrl) => {
            navigator.clipboard.writeText(checkoutUrl).catch(() => {/* silent fallback */});
            toast.success('Link de pago copiado al portapapeles');
        },
        onError: () => toast.error('Error al generar link de pago'),
    });

    return {
        createMutation,
        statusMutation,
        updateMutation,
        deleteMutation,
        cancelMutation,
        checkoutMutation,
        subscribeMutation,
        provisionMutation,
        restartMutation,
        stopMutation,
        startMutation,
        adminTestSubscribeMutation,
        assignMutation,
        adminCheckoutMutation,
    };
}
