import {useMutation, useQuery, useQueryClient} from '@tanstack/react-query';

import {
    apiCreateBillingCheckout,
    apiListBillingItems,
    type BillingCheckoutMode,
} from '../api/billing';
import {toast} from '../stores/toastStore';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {redirigir} from '../platform/navigation';

export const BILLING_ITEMS_KEY = ['billing-items'] as const;

export function useBillingItems() {
    const queryClient = useQueryClient();
    const query = useQuery({
        queryKey: BILLING_ITEMS_KEY,
        queryFn: apiListBillingItems,
        staleTime: 30_000,
    });

    const checkoutMutation = useMutation({
        mutationFn: ({itemIds, mode}: {itemIds?: string[]; mode: BillingCheckoutMode}) =>
            apiCreateBillingCheckout({item_ids: itemIds, mode}),
        onSuccess: (checkoutUrl) => {
            void queryClient.invalidateQueries({queryKey: BILLING_ITEMS_KEY});
            redirigir(checkoutUrl);
        },
        onError: () => toast.error('No se pudo iniciar el pago pendiente'),
    });

    return {
        billingItems: query.data ?? [],
        isLoading: query.isLoading,
        error: query.error,
        checkoutMutation,
    };
}