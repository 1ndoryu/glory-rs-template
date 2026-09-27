/* [044A-38 Fase 3] Sección de historial de pagos en el panel.
 * [035A-18] Rehecha como lista compacta de cards con imagen del servicio y modal
 * tipo factura para concentrar los detalles, 100% alineado al sistema visual.
 * [035A-23] Sub-componentes extraídos a SeccionPagos.parts.tsx para cumplir límite de 300 líneas. */

import { useMemo, useState } from 'react';
import { Loader2, AlertCircle, RotateCcw, Package, CreditCard } from 'lucide-react';
import { useOrdenes } from '../../hooks/useOrdenes';
import { usePagos } from '../../hooks/usePagos';
import { useRefundModal } from '../../hooks/useRefundModal';
import { useAuthStore } from '../../stores/authStore';
import { Textarea } from '../ui/Textarea';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import OptimizedImage from '../ui/OptimizedImage';
import { PAYMENT_MODE_LABELS, formatPrice } from '../../api/orders';
import { getServiceImage } from '../../utils/serviceImages';
import { PagoResumenCard, FacturaLinea } from './SeccionPagos.parts';
import './SeccionPagos.css';

/*
 * PagoResumenCard y FacturaLinea están en SeccionPagos.parts.tsx
 * para mantener este archivo bajo 300 líneas. */

export function SeccionPagos() {
    const { ordenes, cargando, error } = useOrdenes();
    const [ordenSeleccionada, setOrdenSeleccionada] = useState<string | null>(null);
    const { pagos, cargandoPagos, errorPagos } = usePagos(ordenSeleccionada);
    const {
        refundOrderId, refundRazon, refundEnCurso,
        setRefundRazon, abrirModal, cerrarModal, enviarSolicitud,
    } = useRefundModal();
    const effectiveRole = useAuthStore(s => s.user?.effectiveRole) || 'client';
    
    const ordenActiva = useMemo(
        () => ordenes.find(orden => orden.id === ordenSeleccionada) ?? null,
        [ordenes, ordenSeleccionada],
    );

    const ordenesOrdenadas = useMemo(
        () => [...ordenes].sort(
            (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
        ),
        [ordenes],
    );

    const totales = useMemo(() => {
        if (!pagos.length) return null;
        const pagado = pagos
            .filter(p => p.status === 'released' || p.status === 'held')
            .reduce((sum, p) => sum + p.amount_cents, 0);
        const reembolsado = pagos
            .filter(p => p.status === 'refunded')
            .reduce((sum, p) => sum + p.amount_cents, 0);
        const pendiente = pagos
            .filter(p => p.status === 'pending')
            .reduce((sum, p) => sum + p.amount_cents, 0);
        const currency = pagos[0]?.currency || 'USD';
        return { pagado, reembolsado, pendiente, currency };
    }, [pagos]);

    const puedeSolicitarReembolso = effectiveRole === 'client' && 
        pagos.some(p => p.status === 'held' || p.status === 'released');

    if (cargando) {
        return (
            <div className="pagosVacio">
                <Loader2 className="pagosSpinner" size={32} />
            </div>
        );
    }

    if (error) {
        return (
            <div className="pagosError">
                <AlertCircle size={20} />
                <span>{error}</span>
            </div>
        );
    }

    return (
        <div className="pagosContenedor">
            {ordenesOrdenadas.length === 0 ? (
                <div className="pagosVacioDetalle">
                    <Package size={32} />
                    <p className="pagosVacioTexto">No tienes órdenes con historial de pago</p>
                </div>
            ) : (
                <div className="pagosListaCompacta">
                    {ordenesOrdenadas.map(orden => (
                        <PagoResumenCard
                            key={orden.id}
                            order={orden}
                            onOpen={() => setOrdenSeleccionada(orden.id)}
                        />
                    ))}
                </div>
            )}

            <Modal
                abierto={!!ordenActiva}
                onCerrar={() => setOrdenSeleccionada(null)}
                className="pagosFacturaModal"
            >
                {ordenActiva && (
                    <div className="pagosFactura">
                        {/* Header Minimalista */}
                        <div className="pagosFacturaHeader">
                            <div className="pagosFacturaServicio">
                                <OptimizedImage
                                    className="pagosFacturaImagen"
                                    src={getServiceImage(ordenActiva.service_slug)}
                                    alt={ordenActiva.service_title}
                                    loading="lazy"
                                />
                                {/* sentinel-disable-next-line modal-con-titulo -- [259A-5] factura: h3 con className canon modalTitulo como titulo de servicio, no titulo de modal */}
                                <div className="pagosFacturaServicioInfo">
                                    <h3 className="modalTitulo">{ordenActiva.service_title}</h3>
                                    <p className="modalTexto">
                                        Orden #{ordenActiva.order_number}
                                    </p>
                                </div>
                            </div>
                            <div className="pagosFacturaTotalesBloque">
                                <span className="pagosFacturaTotalEyebrow">Total Contratado</span>
                                <strong className="pagosFacturaTotalValor">
                                    {formatPrice(ordenActiva.final_price_cents, ordenActiva.currency)}
                                </strong>
                            </div>
                        </div>

                        {/* Metadatos en linea */}
                        <div className="pagosFacturaMetaList">
                            <div className="pagosFacturaMetaGroup">
                                <span className="pagosFacturaMetaLabel">Moneda</span>
                                <span className="pagosFacturaMetaValue">{ordenActiva.currency}</span>
                            </div>
                            <div className="pagosFacturaMetaGroup">
                                <span className="pagosFacturaMetaLabel">Modalidad</span>
                                <span className="pagosFacturaMetaValue">{PAYMENT_MODE_LABELS[ordenActiva.payment_mode]}</span>
                            </div>
                            <div className="pagosFacturaMetaGroup">
                                <span className="pagosFacturaMetaLabel">Fecha Contratación</span>
                                <span className="pagosFacturaMetaValue">
                                    {new Date(ordenActiva.created_at).toLocaleDateString('es-ES', {
                                        day: 'numeric', month: 'long', year: 'numeric'
                                    })}
                                </span>
                            </div>
                        </div>

                        {/* Lineas de Transacciones */}
                        {/* sentinel-disable-next-line modal-con-titulo -- [259A-5] factura: h4 de seccion "Movimientos de pago", jerarquia de factura, no titulo de modal */}
                        <div className="pagosFacturaLineasBloque">
                            <h4 className="pagosFacturaSubtitulo">
                                Movimientos de pago
                            </h4>
                            
                            {cargandoPagos && (
                                <div className="pagosVacioDetalle">
                                    <Loader2 className="pagosSpinner" size={24} />
                                    <p className="pagosVacioTexto">Cargando...</p>
                                </div>
                            )}

                            {errorPagos && (
                                <p className="pagosError"><AlertCircle size={16} /> {errorPagos}</p>
                            )}

                            {!cargandoPagos && !errorPagos && pagos.length === 0 && (
                                <div className="pagosVacioDetalle">
                                    <CreditCard size={32} />
                                    <p className="pagosVacioTexto">No hay movimientos registrados.</p>
                                </div>
                            )}

                            {!cargandoPagos && pagos.length > 0 && (
                                <>
                                    <div className="pagosFacturaLineas">
                                        {pagos.map(pago => (
                                            <FacturaLinea key={pago.id} payment={pago} />
                                        ))}
                                    </div>

                                    {totales && (
                                        <div className="pagosFacturaResumenTotales">
                                            {totales.pagado > 0 && (
                                                <div className="pagosFacturaResumenItem">
                                                    <span className="pagosFacturaMetaLabel">Pagado</span>
                                                    <span className="pagosFacturaMetaValue pagosFacturaResumenPagado">
                                                        {formatPrice(totales.pagado, totales.currency)}
                                                    </span>
                                                </div>
                                            )}
                                            {totales.pendiente > 0 && (
                                                <div className="pagosFacturaResumenItem">
                                                    <span className="pagosFacturaMetaLabel">Pendiente</span>
                                                    <span className="pagosFacturaMetaValue pagosFacturaResumenPendiente">
                                                        {formatPrice(totales.pendiente, totales.currency)}
                                                    </span>
                                                </div>
                                            )}
                                            {totales.reembolsado > 0 && (
                                                <div className="pagosFacturaResumenItem">
                                                    <span className="pagosFacturaMetaLabel">Reembolsado</span>
                                                    <span className="pagosFacturaMetaValue pagosFacturaResumenReembolsado">
                                                        {formatPrice(totales.reembolsado, totales.currency)}
                                                    </span>
                                                </div>
                                            )}
                                        </div>
                                    )}
                                </>
                            )}
                        </div>

                        {/* sentinel-disable-next-line modal-acciones-no-canonico -- [259A-5] factura: el div YA lleva modalAcciones canon; pagosFacturaAcciones solo anade layout local */}
                        {puedeSolicitarReembolso && (
                            <div className="modalAcciones pagosFacturaAcciones">
                                <Button
                                    variante="outline"
                                    tamano="pequeno"
                                    onClick={() => {
                                        setOrdenSeleccionada(null);
                                        abrirModal(ordenActiva.id);
                                    }}
                                >
                                    <RotateCcw size={14} className="pagosReembolsoIcon" />
                                    Solicitar Reembolso
                                </Button>
                            </div>
                        )}
                    </div>
                )}
            </Modal>

            {/* Modal de solicitud de reembolso */}
            <Modal abierto={!!refundOrderId} onCerrar={cerrarModal}>
                <div className="pagoModalContenido">
                    <p className="modalTexto pagosReembolsoIntro">
                        Describe el motivo de tu solicitud de reembolso. Un administrador la revisará.
                    </p>
                    <Textarea
                        className="pagoModalTextarea"
                        placeholder="Motivo del reembolso..."
                        value={refundRazon}
                        onChange={(e) => setRefundRazon(e.target.value)}
                        rows={4}
                    />
                    <div className="modalAcciones">
                        <Button
                            variante="outline"
                            onClick={cerrarModal}
                            disabled={refundEnCurso}
                        >
                            Cancelar
                        </Button>
                        <Button
                            onClick={() => void enviarSolicitud()}
                            disabled={refundEnCurso || !refundRazon.trim()}
                        >
                            {refundEnCurso ? 'Enviando...' : 'Enviar solicitud'}
                        </Button>
                    </div>
                </div>
            </Modal>
        </div>
    );
}