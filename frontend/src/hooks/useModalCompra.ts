/* [044A-40] Hook para la lógica del modal de compra.
 * Maneja: pasos del modal, auth inline, creación de orden e inicio de pago.
 * Extraído de ModalCompra.tsx para cumplir SRP (max 3 useState en componente).
 * [064A-3] Flujo simplificado: solo pide email. Si el email ya existe, pide password.
 * [104A-25] Guard ref contra doble invocación de iniciarCompra (previene órdenes duplicadas).
 * [166A-2] Flujo checkout directo: Stripe PRIMERO, orden DESPUÉS del pago via webhook. */
import {useState, useRef} from 'react';
import {useAuthStore} from '../stores/authStore';
import {apiQuickRegister, apiLogin} from '../api/auth';
import type {PaymentMode} from '../api/orders';
import {apiSelfSubscribe, apiSelfSubscribeVps} from '../api/hosting';
import {apiCreateCheckoutIntent} from '../api/payments';
import {PANEL_TAB_KEY} from '../data/panel';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {redirigir} from '../platform/navigation';
import {navegar} from '../navegacionSPA';
import type {PlanServicio} from '../data/planes/tipos';

export type PasoModal = 'resumen' | 'auth' | 'procesando' | 'checkout' | 'error';

/* [166A-2] CheckoutPendiente simplificado: ya no tiene orderId ni orderNumber
 * porque la orden NO existe hasta que el pago se confirma via webhook. */
interface CheckoutPendiente {
    clientSecret: string;
    amountCents: number;
    currency: string;
}

interface UseModalCompraParams {
    plan: PlanServicio;
    servicioSlug: string;
    onClose: () => void;
}

function normalizeHostingPlanSlug(planId: string): string {
    return planId.trim().toLowerCase().replace(/^hosting-/, '');
}

function normalizeVpsTier(planId: string): string {
    return planId.trim().toLowerCase().replace(/^vps-/, '');
}

function getPurchaseErrorMessage(err: unknown, fallback: string): string {
    if (typeof err === 'object' && err !== null && 'response' in err) {
        const response = (err as {response?: {data?: {message?: unknown}}}).response;
        const message = response?.data?.message;
        if (typeof message === 'string' && message.trim()) {
            return message;
        }
    }

    if (err instanceof Error && !err.message.startsWith('Request failed with status code')) {
        return err.message;
    }

    return fallback;
}

export function useModalCompra({plan, servicioSlug, onClose}: UseModalCompraParams) {
    const logueado = useAuthStore(s => s.logueado);
    const login = useAuthStore(s => s.login);
    const user = useAuthStore(s => s.user);

    const [paso, setPaso] = useState<PasoModal>('resumen');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    /* [064A-3] Si el email ya existe, mostramos campo password */
    const [emailExiste, setEmailExiste] = useState(false);
    const [errorMsg, setErrorMsg] = useState('');
    /* [064A-60] Modo de pago seleccionable: full (20% desc), half_half (10%), phased (0%) */
    const [paymentMode, setPaymentMode] = useState<PaymentMode>('full');
    /* [104A-16] Hosting ya no pasa por órdenes genéricas.
     * El dominio opcional se envía al flujo self-service real de suscripciones. */
    const [hostingDomain, setHostingDomain] = useState('');
    const [checkoutPendiente, setCheckoutPendiente] = useState<CheckoutPendiente | null>(null);
    const isHosting = servicioSlug === 'hosting';
    const isVps = servicioSlug === 'vps';
    /* [104A-25] Ref guard: previene doble invocación de iniciarCompra.
     * useState es async y no previene race conditions entre renders. */
    const compraEnCurso = useRef(false);

    /* [20CA-1] Tras pago exitoso: autenticar al usuario si no lo está.
     * quick_register es idempotente para usuarios sin contraseña: retorna JWT
     * sin crear duplicado. Si el usuario ya tiene contraseña (409), solo navegar. */
    const navegarAlPanelPendiente = async () => {
        localStorage.setItem(PANEL_TAB_KEY, isHosting || isVps ? 'hosting' : 'proyectos');

        if (!logueado && email.trim()) {
            try {
                const authResp = await apiQuickRegister(email.trim());
                login(authResp.token, authResp.user_id, authResp.email, authResp.role, authResp.effective_role, authResp.needs_password);
            } catch {
                /* 409 = usuario con contraseña; puede loguearse manualmente */
            }
        }

        navegar('/panel');
        onClose();
    };

    const crearHostingYRedirigir = async () => {
        setPaso('procesando');
        try {
            const response = await apiSelfSubscribe({
                plan: normalizeHostingPlanSlug(plan.id),
                domain: hostingDomain.trim() || undefined,
            });
            localStorage.setItem(PANEL_TAB_KEY, 'hosting');
            redirigir(response.checkout_url);
        } catch (err: unknown) {
            setPaso('error');
            setErrorMsg(getPurchaseErrorMessage(err, 'Error al iniciar el checkout de hosting.'));
        }
    };

    const crearVpsYRedirigir = async () => {
        setPaso('procesando');
        try {
            const response = await apiSelfSubscribeVps({
                tier: normalizeVpsTier(plan.id),
                hostname: hostingDomain.trim() || undefined,
            });
            localStorage.setItem(PANEL_TAB_KEY, 'hosting');
            redirigir(response.checkout_url);
        } catch (err: unknown) {
            setPaso('error');
            setErrorMsg(getPurchaseErrorMessage(err, 'Error al iniciar el checkout de VPS.'));
        }
    };

    /* [166A-2] Flujo nuevo: crear PaymentIntent directo (sin crear orden).
     * [20CA-1] El email se envía al backend para guardarlo en metadata de Stripe.
     * La orden Y el usuario se crean via webhook cuando Stripe confirma el pago.
     * Esto garantiza que NO se crea cuenta ni pedido sin pago confirmado. */
    const crearOrdenYPagar = async () => {
        /* El email viene del campo del modal (no-auth) o del usuario logueado */
        const checkoutEmail = email.trim() || user?.email || '';
        if (!checkoutEmail || !checkoutEmail.includes('@')) {
            setPaso('error');
            setErrorMsg('Introduce un email válido para continuar.');
            return;
        }

        setPaso('procesando');
        try {
            const checkoutIntent = await apiCreateCheckoutIntent({
                service_slug: servicioSlug,
                plan_slug: plan.id,
                payment_mode: paymentMode,
                email: checkoutEmail,
            });
            localStorage.setItem(PANEL_TAB_KEY, 'proyectos');
            setCheckoutPendiente({
                clientSecret: checkoutIntent.client_secret,
                amountCents: checkoutIntent.amount_cents,
                currency: checkoutIntent.currency,
            });
            setPaso('checkout');
        } catch (err: unknown) {
            setPaso('error');
            setErrorMsg(getPurchaseErrorMessage(err, 'Error al iniciar el pago. Intenta de nuevo.'));
        }
    };

    const iniciarCompra = async () => {
        /* [104A-25] Guard contra doble invocación: si ya hay una compra en curso, ignorar */
        if (compraEnCurso.current) return;
        compraEnCurso.current = true;
        try {
            if (isHosting) {
                await crearHostingYRedirigir();
                return;
            }
            if (isVps) {
                await crearVpsYRedirigir();
                return;
            }
            await crearOrdenYPagar();
        } finally {
            compraEnCurso.current = false;
        }
    };

    /* Paso 1: usuario confirma que quiere continuar */
    const handleContinuar = () => {
        /* [035A-5] El backend de ordenes permite comprar solo como client/admin.
         * En local es comun quedar logueado como employee por pruebas del panel;
         * sin este guard el modal intenta crear la orden y recibe 403. */
        if (logueado && user?.effectiveRole === 'employee') {
            setErrorMsg('La compra publica solo se puede iniciar con una sesion de cliente. Si estas probando con un freelancer, cambia a cliente o usa una cuenta cliente.');
            return;
        }

        setErrorMsg('');
        if (logueado) {
            void iniciarCompra();
        } else {
            setPaso('auth');
        }
    };

    /* [064A-3] Paso 2: login si el usuario ya existe.
     * [20CA-1] Para usuarios nuevos, NO se llama apiQuickRegister antes del pago.
     * En vez de crear la cuenta inmediatamente, el email se pasa al checkout intent
     * y el usuario se crea en el webhook DESPUÉS del pago confirmado.
     * Esto evita cuentas huérfanas cuando el usuario abandona Stripe. */
    const handleAuth = async (e: React.FormEvent) => {
        e.preventDefault();
        setErrorMsg('');

        /* Si ya sabemos que el email existe, hacer login con password */
        if (emailExiste) {
            setPaso('procesando');
            try {
                const authResp = await apiLogin(email, password);
                login(authResp.token, authResp.user_id, email, authResp.role, authResp.effective_role, authResp.needs_password);
                await iniciarCompra();
            } catch (err: unknown) {
                setPaso('auth');
                const msg = err instanceof Error ? err.message : 'Credenciales inválidas.';
                setErrorMsg(msg);
            }
            return;
        }

        /* [20CA-1] Usuario nuevo: NO crear cuenta todavía.
         * Guardar email y proceder directamente al checkout de Stripe.
         * La cuenta se creará en el webhook cuando el pago se confirme. */
        setPaso('procesando');
        try {
            await iniciarCompra();
        } catch (err: unknown) {
            setPaso('error');
            setErrorMsg(getPurchaseErrorMessage(err, 'Error al procesar. Intenta de nuevo.'));
        }
    };

    const reintentar = () => {
        setPaso('resumen');
        setEmailExiste(false);
        setErrorMsg('');
        setCheckoutPendiente(null);
    };

    return {
        paso,
        email,
        setEmail,
        password,
        setPassword,
        emailExiste,
        errorMsg,
        paymentMode,
        setPaymentMode,
        hostingDomain,
        setHostingDomain,
        checkoutPendiente,
        isHosting,
        isVps,
        navegarAlPanelPendiente,
        handleContinuar,
        handleAuth,
        reintentar
    };
}
