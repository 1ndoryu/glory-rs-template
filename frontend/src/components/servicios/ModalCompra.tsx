/* [044A-40] Modal de compra de servicio.
 * Flujo: muestra resumen minimo del plan → si no logueado pide email → crea cuenta → orden → pago.
 * [064A-3] Simplificado: solo pide email. Si ya existe, pide password.
 * [035A-4] El brief del proyecto sale del modal inicial; el checkout no pide discovery previo.
 * Se abre al hacer click en CTA de un plan (SeccionPlanesServicio).
 * Usa componente <Modal> base y hook useModalCompra para la lógica. */
import React from 'react';
import {useTranslation} from 'react-i18next';
import CheckoutModal from '../panel/CheckoutModal';
import {Modal, ModalBody} from '../ui/Modal';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import {useModalCompra} from '../../hooks/useModalCompra';
import {useQuery} from '@tanstack/react-query';
import {apiCheckFirstOrderDiscount} from '../../api/orders';
import {PAYMENT_MODE_LABELS, type PaymentMode} from '../../api/orders';
import type {PlanServicio} from '../../data/planes/tipos';
import './ModalCompra.css';

/* [064A-60] Info de descuento por modo de pago. Alineado con backend discount_for_mode */
const PAYMENT_MODE_DISCOUNT: Record<PaymentMode, number> = {
    full: 20,
    half_half: 10,
    phased: 0,
};

/* [064A-60] Descripciones breves de cada modo */
const PAYMENT_MODE_DESC: Record<PaymentMode, string> = {
    full: 'Un solo pago al confirmar',
    half_half: '50% al inicio, 50% al entregar',
    phased: 'Paga cada fase por separado',
};

/* [084A-12] Parsea precio string ("$100", "$350") a cents.
 * Retorna null si no es numérico ("A medida", "Contactar"). */
function parsePrecioCents(precio: string): number | null {
    const match = precio.replace(/,/g, '').match(/\$?\s*([\d.]+)/);
    return match ? Math.round(parseFloat(match[1]) * 100) : null;
}

/* [084A-12] Formatea cents a precio visible */
function formatPrecioDescontado(cents: number): string {
    const val = cents / 100;
    return val % 1 === 0 ? `$${val}` : `$${val.toFixed(2)}`;
}

interface ModalCompraProps {
    plan: PlanServicio;
    servicioSlug: string;
    abierto: boolean;
    onCerrar: () => void;
}

export const ModalCompra: React.FC<ModalCompraProps> = ({plan, servicioSlug, abierto, onCerrar}) => {
    const {t} = useTranslation();
    const {
        paso, email, setEmail, password, setPassword,
        emailExiste, errorMsg, paymentMode, setPaymentMode,
        hostingDomain, setHostingDomain, checkoutPendiente, isHosting, isVps,
        navegarAlPanelPendiente, handleContinuar, handleAuth, reintentar
    } = useModalCompra({plan, servicioSlug, onClose: onCerrar});

    /* [277A-10] Verificar si califica para descuento 50% primer pedido */
    const {data: firstOrderDiscount} = useQuery({
        queryKey: ['first-order-discount'],
        queryFn: apiCheckFirstOrderDiscount,
        staleTime: 10 * 60 * 1000,
        retry: 0,
        enabled: !isHosting && !isVps,
    });
    const qualifiesFirstOrder = firstOrderDiscount?.qualifies && !isHosting && !isVps;

    /* [084A-12] Precio dinámico según modo de pago o meses seleccionados */
    const baseCents = parsePrecioCents(plan.precio);
    const descuento = isHosting ? 0 : PAYMENT_MODE_DISCOUNT[paymentMode];
    const totalCents = baseCents != null
        ? Math.round(baseCents * (1 - descuento / 100))
        : null;
    const precioFinal = totalCents != null ? formatPrecioDescontado(totalCents) : plan.precio;
    const tieneDescuento = baseCents != null && descuento > 0;

    if (paso === 'checkout' && checkoutPendiente) {
        return (
            <CheckoutModal
                amountCents={checkoutPendiente.amountCents}
                currency={checkoutPendiente.currency}
                clientSecret={checkoutPendiente.clientSecret}
                onClose={navegarAlPanelPendiente}
                onSuccess={navegarAlPanelPendiente}
            />
        );
    }

    return (
            <Modal abierto={abierto} onCerrar={onCerrar}>
            {/* [035A-4] Resumen minimo: sin wrapper extra ni brief previo a la compra. */}
            {(paso === 'resumen' || paso === 'auth') && (
                <p className="modalTexto">{plan.descripcion}</p>
            )}

            {/* Paso resumen: selector de modo de pago + botón continuar */}
            {paso === 'resumen' && (
                <div className="modalCompraPaso">
                    {/* [104A-16] Hosting: el flujo publico usa la suscripcion real.
                     * Pedimos dominio opcional y dejamos claro que el cobro es mensual recurrente. */}
                    {isHosting ? (
                        <div className="modalCompraBrief">
                            <span className="modalCompraBriefLabel">
                                Dominio para tu hosting
                            </span>
                            <Input
                                type="text"
                                value={hostingDomain}
                                onChange={e => setHostingDomain(e.target.value)}
                                placeholder="ejemplo.com (opcional)"
                            />
                            <p className="modalCompraAviso">
                                Stripe abrirá una suscripción mensual. Puedes dejar el dominio vacío y configurarlo después desde el panel.
                            </p>
                        </div>
                    ) : isVps ? (
                        <div className="modalCompraBrief">
                            <span className="modalCompraBriefLabel">
                                Hostname solicitado
                            </span>
                            <Input
                                type="text"
                                value={hostingDomain}
                                onChange={e => setHostingDomain(e.target.value)}
                                placeholder="cliente-vps-01 (opcional)"
                            />
                            <p className="modalCompraAviso">
                                Usaremos este nombre como referencia interna y para el bootstrap inicial al provisionar el servidor.
                            </p>
                        </div>
                    ) : (
                        /* [064A-60] Servicios: selector de modo de pago */
                        <div className="modalCompraModos" role="radiogroup" aria-label="Modo de pago">
                            {(['full', 'half_half', 'phased'] as PaymentMode[]).map(mode => (
                                <div
                                    key={mode}
                                    role="radio"
                                    aria-checked={paymentMode === mode}
                                    tabIndex={0}
                                    className={`modalCompraModo ${paymentMode === mode ? 'modalCompraModoActivo' : ''}`}
                                    onClick={() => setPaymentMode(mode)}
                                    onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') setPaymentMode(mode); }}
                                >
                                    <span className="modalCompraModoNombre">{PAYMENT_MODE_LABELS[mode]}</span>
                                    <span className="modalCompraModoDesc">{PAYMENT_MODE_DESC[mode]}</span>
                                    {PAYMENT_MODE_DISCOUNT[mode] > 0 && (
                                        <span className="modalCompraModoDescuento">
                                            {PAYMENT_MODE_DISCOUNT[mode]}% descuento
                                        </span>
                                    )}
                                </div>
                            ))}
                        </div>
                    )}
                    {/* [277A-10] Banner 50% OFF primer pedido */}
                    {qualifiesFirstOrder && (
                        <div className="modalCompraPrimerPedido">
                            <span className="modalCompraPrimerPedidoBadge">50% OFF</span>
                            <span className="modalCompraPrimerPedidoTexto">Descuento en tu primer servicio</span>
                        </div>
                    )}
                    {/* [084A-12] Resumen de precio con descuento aplicado */}
                    {baseCents != null && (
                        <div className="modalCompraPrecioResumen">
                            {tieneDescuento || qualifiesFirstOrder ? (
                                <>
                                    <span className="modalCompraPrecioOriginal">{plan.precio}</span>
                                    <span className="modalCompraPrecioFinal">
                                        {qualifiesFirstOrder
                                            ? formatPrecioDescontado(Math.round(baseCents * 0.5))
                                            : precioFinal}
                                    </span>
                                    <span className="modalCompraPrecioAhorro">
                                        Ahorras {qualifiesFirstOrder ? 50 : descuento}%
                                    </span>
                                </>
                            ) : (
                                <span className="modalCompraPrecioFinal">{plan.precio}</span>
                            )}
                        </div>
                    )}
                    {errorMsg && <p className="modalCompraErrorMensaje">{errorMsg}</p>}
                    <Button variante="primario" tamano="mediano" onClick={handleContinuar} className="modalCompraCta">
                        {isHosting || isVps
                            ? t('purchase.continue_pay', 'Continuar al checkout')
                            : t('purchase.continue', 'Continuar')} ({precioFinal})
                    </Button>
                    <p className="modalCompraAviso">
                        {isHosting
                            ? 'El cargo se confirma dentro de Stripe Checkout.'
                            : isVps
                                ? 'El cargo se confirma dentro de Stripe Checkout y la provisión queda sujeta a aprobación manual.'
                            : t('purchase.no_charge_yet', 'Aún no se te cobrará')}
                    </p>
                </div>
            )}

            {/* [064A-3] Paso auth: solo email. Si email ya existe, muestra password */}
            {paso === 'auth' && (
                <ModalBody as="form" onSubmit={handleAuth}>
                    <p className="modalTexto modalCompraAuthNota">
                        {emailExiste
                            ? t('purchase.existing_account', 'Ya tienes cuenta. Introduce tu contraseña para continuar.')
                            : t('purchase.enter_email', 'Introduce tu email para continuar')}
                    </p>
                    {errorMsg && <p className="modalCompraErrorMensaje">{errorMsg}</p>}
                    <Input
                        type="email"
                        placeholder={t('auth.email_placeholder', 'tu@email.com')}
                        value={email}
                        onChange={e => setEmail(e.target.value)}
                        required
                        disabled={emailExiste}
                    />
                    {emailExiste && (
                        <Input
                            type="password"
                            placeholder={t('auth.password_placeholder', 'Contraseña')}
                            value={password}
                            onChange={e => setPassword(e.target.value)}
                            required
                            minLength={8}
                        />
                    )}
                    <Button variante="primario" tamano="mediano" type="submit" className="modalCompraCta">
                        {isHosting || isVps
                            ? t('purchase.continue_pay', 'Continuar al checkout')
                            : t('purchase.continue_pay', 'Continuar al pago')} ({precioFinal})
                    </Button>
                    <p className="modalCompraAviso">
                        {isHosting
                            ? 'El cargo se confirma dentro de Stripe Checkout.'
                            : isVps
                                ? 'El cargo se confirma dentro de Stripe Checkout y la provisión queda sujeta a aprobación manual.'
                            : t('purchase.no_charge_yet', 'Aún no se te cobrará')}
                    </p>
                </ModalBody>
            )}

            {/* Procesando */}
            {paso === 'procesando' && (
                <div className="modalCompraProcesando">
                    <p>{t('purchase.processing', 'Procesando tu orden...')}</p>
                </div>
            )}

            {/* Error */}
            {paso === 'error' && (
                <div className="modalCompraError">
                    <p className="modalCompraErrorMensaje">{errorMsg}</p>
                    <Button variante="outline" tamano="pequeno" onClick={reintentar}>
                        {t('common.retry', 'Intentar de nuevo')}
                    </Button>
                </div>
            )}
        </Modal>
    );
};
