/* [044A-38 Fase 3] Modal de checkout con Stripe Elements.
 * Inicia PaymentIntent en el backend, muestra formulario de tarjeta,
 * y confirma el pago. El webhook procesa el resultado asíncronamente.
 * [064A-55] Migrado a <Modal> del sistema (focus trap, Escape, scroll lock).
 * [104A-15] Si recibe `clientSecret`, reutiliza el PaymentIntent ya creado
 * por el flujo publico y evita generar uno duplicado al entrar a Stripe.
 * [166A-2] orderId y orderNumber opcionales: en checkout directo la orden
 * no existe hasta que el pago se confirma via webhook. */

import { useCallback, useState } from 'react';
import {
    Elements,
    PaymentElement,
    useStripe,
    useElements,
} from '@stripe/react-stripe-js';
import { apiInitiatePayment } from '../../api/payments';
import { formatPrice } from '../../api/orders';
import { useStripeClient } from '../../hooks/useStripeClient';
import { Button } from '../ui/Button';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {obtenerOrigen} from '../../platform/navigation';
import { Modal } from '../ui/Modal';
import './CheckoutModal.css';

interface CheckoutModalProps {
    orderId?: string;
    orderNumber?: number;
    amountCents: number;
    currency: string;
    clientSecret?: string;
    phaseNumber?: number;
    onClose: () => void;
    onSuccess: () => void;
}

export default function CheckoutModal(props: CheckoutModalProps) {
    const { orderId, phaseNumber, onClose, onSuccess, clientSecret: initialClientSecret } = props;
    const [clientSecret, setClientSecret] = useState<string | null>(initialClientSecret ?? null);
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(false);
    const {stripePromise, cargando: cargandoStripe, configurado: stripeConfigurado} = useStripeClient();

    const iniciar = useCallback(async () => {
        if (clientSecret) {
            return;
        }

        /* [166A-2] Solo iniciar pago si hay orderId (flujo legacy).
         * En checkout directo, clientSecret siempre viene como prop. */
        if (!orderId) {
            setError('No se pudo iniciar el pago. Intenta de nuevo.');
            return;
        }

        setLoading(true);
        setError(null);
        try {
            const resp = await apiInitiatePayment(orderId, {
                phase_number: phaseNumber,
            });
            setClientSecret(resp.client_secret);
        } catch (err: unknown) {
            /* [074A-24] Extraer mensaje real del response body de Axios.
             * El backend devuelve { error: "...", message: "..." } en errores. */
            const axiosData = (err as { response?: { data?: { message?: string } } })?.response?.data;
            const msg = axiosData?.message
                ?? (err instanceof Error ? err.message : 'Error iniciando pago');
            setError(msg);
        } finally {
            setLoading(false);
        }
    }, [orderId, phaseNumber, clientSecret]);

    if (cargandoStripe) {
        return (
            <Modal abierto onCerrar={onClose} className="modalCompacto">
                <p className="checkoutMonto">Preparando Stripe...</p>
            </Modal>
        );
    }

    if (!stripeConfigurado || !stripePromise) {
        return (
            <Modal abierto onCerrar={onClose} className="modalCompacto">
                <p className="checkoutError">
                    Stripe no esta configurado en este entorno.
                </p>
            </Modal>
        );
    }

    return (
        <Modal abierto onCerrar={onClose} className="modalCompacto">
            <p className="checkoutMonto">
                {formatPrice(props.amountCents, props.currency)}
            </p>

            {error && <p className="checkoutError">{error}</p>}

            {!clientSecret && (
                <div className="modalAcciones">
                    <Button
                        className="checkoutBoton"
                        type="button"
                        variante="primario"
                        tamano="mediano"
                        onClick={iniciar}
                        disabled={loading}
                    >
                        {loading ? 'Preparando...' : 'Continuar al pago'}
                    </Button>
                </div>
            )}

            {clientSecret && (
                <Elements
                    stripe={stripePromise}
                    options={{ clientSecret, appearance: { theme: 'stripe' } }}
                >
                    <FormularioPago
                        onSuccess={onSuccess}
                        onError={setError}
                    />
                </Elements>
            )}
        </Modal>
    );
}

function FormularioPago({
    onSuccess,
    onError,
}: {
    onSuccess: () => void;
    onError: (msg: string) => void;
}) {
    const stripe = useStripe();
    const elements = useElements();
    const [procesando, setProcesando] = useState(false);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!stripe || !elements) return;

        setProcesando(true);
        const { error } = await stripe.confirmPayment({
            elements,
            confirmParams: {
                return_url: `${obtenerOrigen()}/panel`,
            },
            redirect: 'if_required',
        });

        if (error) {
            onError(error.message ?? 'Error al procesar el pago');
            setProcesando(false);
        } else {
            onSuccess();
        }
    };

    return (
        <form onSubmit={handleSubmit} className="checkoutForm">
            <PaymentElement />
            <div className="modalAcciones">
                <Button
                    type="submit"
                    tamano="pequeno"
                    className="checkoutBoton"
                    disabled={!stripe || procesando}
                >
                    {procesando ? 'Procesando...' : 'Pagar ahora'}
                </Button>
            </div>
        </form>
    );
}
