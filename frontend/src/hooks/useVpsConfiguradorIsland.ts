/* [195A-1] Hook del configurador VPS.
 * Centraliza auth inline + checkout para que la island quede como vista declarativa. */
import {useState} from 'react';
import type React from 'react';
import {apiLogin, apiQuickRegister, extraerMensajeError} from '../api/auth';
import {apiSelfSubscribeVps} from '../api/hosting';
import {PANEL_TAB_KEY} from '../data/panel';
import {useAuthStore} from '../stores/authStore';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {redirigir} from '../platform/navigation';
import {useVpsCatalog} from './useVpsCatalog';

interface ConfigForm {
    selectedTier: string;
    hostname: string;
    email: string;
    password: string;
    selectedStorage: string;
    selectedRegion: string;
    selectedOs: string;
    serverPassword: string;
}

interface ConfigStatus {
    submitting: boolean;
    error: string;
}

const DEFAULT_FORM: ConfigForm = {
    selectedTier: '',
    hostname: '',
    email: '',
    password: '',
    selectedStorage: '',
    selectedRegion: 'EU',
    selectedOs: 'Ubuntu 24.04 LTS',
    serverPassword: '',
};

/* Comisión Stripe México: 3.6% + 1.5% (internacional) + 1% (divisa) = 6.1% + $3 MXN.
 * Se muestra como cargo adicional en el resumen de pago. */
const STRIPE_FEE_RATE = 0.061;
const STRIPE_FEE_FIXED_CENTS = 15; /* $3 MXN ≈ $0.15 USD al TC ~20 */

function stripeFeeCents(baseCents: number): number {
    return Math.round(baseCents * STRIPE_FEE_RATE) + STRIPE_FEE_FIXED_CENTS;
}

export function useVpsConfiguradorIsland(initialTier?: string) {
    const {plans, isLoading} = useVpsCatalog();
    const logueado = useAuthStore(s => s.logueado);
    const login = useAuthStore(s => s.login);
    const [form, setForm] = useState<ConfigForm>({...DEFAULT_FORM, selectedTier: initialTier ?? ''});
    const [emailExiste, setEmailExiste] = useState(false);
    const [status, setStatus] = useState<ConfigStatus>({submitting: false, error: ''});

    const selectedTier = form.selectedTier || initialTier || plans[0]?.tier_name || '';
    const selectedPlan = plans.find(plan => plan.tier_name === selectedTier) ?? plans[0];
    const storageKey = form.selectedStorage || selectedPlan?.storage_options[0] || '';
    const storageExtraCents = (selectedPlan?.storage_extra_cents as Record<string, number> | undefined)?.[storageKey] ?? 0;
    const regionExtraCents = (selectedPlan?.region_extra_cents as Record<string, number> | undefined)?.[form.selectedRegion] ?? 0;
    const monthlyTotal = selectedPlan
        ? selectedPlan.monthly_price_cents + storageExtraCents + regionExtraCents
        : 0;
    const subtotalFirstPayment = selectedPlan
        ? monthlyTotal + selectedPlan.setup_fee_cents
        : 0;
    const processingFeeCents = selectedPlan ? stripeFeeCents(subtotalFirstPayment) : 0;
    const dueToday = subtotalFirstPayment + processingFeeCents;

    const updateField = (field: keyof ConfigForm, value: string) => {
        if (field === 'selectedTier') {
            /* Al cambiar plan, resetear storage a la primera opción del nuevo plan */
            const newPlan = plans.find(p => p.tier_name === value);
            const newStorage = newPlan?.storage_options[0] ?? '';
            setForm(prev => ({...prev, selectedTier: value, selectedStorage: newStorage}));
            return;
        }
        setForm(prev => ({...prev, [field]: value}));
    };

    const submitCheckout = async () => {
        if (!selectedPlan || status.submitting) return;
        setStatus({submitting: true, error: ''});
        try {
            const response = await apiSelfSubscribeVps({
                tier: selectedPlan.tier_name,
                hostname: form.hostname.trim() || undefined,
                storage_preference: form.selectedStorage || selectedPlan.storage_options[0] || undefined,
                region_preference: form.selectedRegion || undefined,
                os_preference: form.selectedOs || undefined,
                server_password: form.serverPassword.trim() || undefined,
            });
            localStorage.setItem(PANEL_TAB_KEY, 'hosting');
            redirigir(response.checkout_url);
        } catch (error) {
            setStatus({submitting: false, error: extraerMensajeError(error)});
        }
    };

    const handleSubmit = async (event: React.FormEvent) => {
        event.preventDefault();
        if (!selectedPlan) return;

        if (logueado) {
            await submitCheckout();
            return;
        }

        setStatus({submitting: true, error: ''});
        try {
            const auth = emailExiste
                ? await apiLogin(form.email, form.password)
                : await apiQuickRegister(form.email);
            login(auth.token, auth.user_id, auth.email, auth.role, auth.effective_role, auth.needs_password);
            await submitCheckout();
        } catch (error) {
            const isConflict = typeof error === 'object' && error !== null && 'response' in error
                && (error as {response?: {status?: number}}).response?.status === 409;
            if (isConflict) {
                setEmailExiste(true);
                setStatus({submitting: false, error: 'Ya tienes cuenta. Introduce tu contraseña para continuar.'});
                return;
            }
            setStatus({submitting: false, error: extraerMensajeError(error)});
        }
    };

    return {
        plans,
        isLoading,
        selectedPlan,
        form,
        emailExiste,
        status,
        dueToday,
        monthlyTotal,
        processingFeeCents,
        storageExtraCents,
        regionExtraCents,
        logueado,
        updateField,
        handleSubmit,
    };
}
