/* [064A-30] Detalle de orden rediseñado: cuadro de info (freelancer, servicio,
 * número, fecha inicio, precio, 3 puntos) + historial de fases con entregas y
 * revisiones. Clientes solicitan revisiones al recibir entrega.
 * Opciones: reportar, extensión de tiempo (empleado), cancelar.
 * custom toggle (<label> con CSS switch).
 * sentinel-disable-file limite-lineas: Componente orquestador de detalle de orden;
 * lógica principal ya extraída a useOrdenDetalle + sub-componentes.
 * sentinel-disable-file usestate-excesivo: 5 estados necesarios para flujos independientes
 * (fases, intermediario, modal asignar); ya se usa useOrdenDetalle para el estado principal. */
import React, {useState, useCallback} from 'react';
import {CreditCard, XCircle, ArrowLeft, AlertTriangle, Bot, ArrowRightLeft, UserCheck, UserX, Plus, DollarSign} from 'lucide-react';
import {
    ORDER_STATUS_LABELS,
    apiToggleAiIntermediary,
    apiAddOrderPhase,
    apiDeleteOrderPhase,
    type OrderResponse,
    type OrderPhaseResponse,
    type OrderStatus,
} from '../../api/orders';
import {ReviewPanel} from './ReviewPanel';
import {FaseCard} from './FaseCard';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import {MenuContextual, type MenuContextualItem} from '../ui/ContextMenu';
import CheckoutModal from './CheckoutModal';
import {OrderChat} from './OrderChat';
import {OrderDetailModals} from './OrderDetailModals';
import {OrderProjectDescription} from './OrderProjectDescription';
import {OrdenHistorialActividad} from './OrdenHistorialActividad';
import {CancellationBanner} from './CancellationBanner';
import {OrdenInfoGrid} from './OrdenInfoGrid';
import {ModalAsignar} from './ModalAsignar';
import {apiUnassignOrder} from '../../api/assignment';
import {useOrdenDetalle} from '../../hooks/useOrdenDetalle';
import {useCancellationRequest} from '../../hooks/useCancellationRequest';
import {useRefunds} from '../../hooks/useRefunds';
import {useChatStore} from '../../stores/chatStore';
import {useMutation, useQueryClient} from '@tanstack/react-query';
import './OrdenDetalle.css';

/* [104A-29] pending_payment ya no se usa a nivel de orden. payment_held es el estado inicial. */
const CANCELABLE_STATUSES: OrderStatus[] = ['payment_held', 'awaiting_assignment'];

const STATUS_CLASS: Record<OrderStatus, string> = {
    pending_payment: 'ordenBadge--paymentHeld',
    payment_held: 'ordenBadge--paymentHeld',
    awaiting_assignment: 'ordenBadge--awaiting',
    in_progress: 'ordenBadge--inProgress',
    under_review: 'ordenBadge--underReview',
    completed: 'ordenBadge--completed',
    cancelled: 'ordenBadge--cancelled',
    disputed: 'ordenBadge--disputed',
};

interface OrdenDetalleProps {
    order: OrderResponse;
    phases: OrderPhaseResponse[];
    effectiveRole: string;
    onVolver: () => void;
    onCancelar: (orderId: string, reason?: string) => Promise<void>;
    onAprobar: (orderId: string, phase: number) => Promise<void>;
    onRevision: (orderId: string, phase: number) => Promise<void>;
    onActualizarDescripcion: (orderId: string, projectDescription: string) => Promise<void>;
    onActualizarFase: (
        orderId: string,
        phase: number,
        req: {
            title?: string;
            description?: string;
            price_cents?: number;
            estimated_days?: number;
            max_revisions?: number;
        },
    ) => Promise<void>;
    onPagoExitoso: () => void;
    onIrADelegaciones?: () => void;
    cancelando: boolean;
    actualizandoDescripcion: boolean;
    actualizandoFase: boolean;
}

export const OrdenDetalle: React.FC<OrdenDetalleProps> = ({
    order, phases, effectiveRole, onVolver, onCancelar, onAprobar, onRevision, onPagoExitoso, cancelando,
    onActualizarDescripcion, onActualizarFase, actualizandoDescripcion, actualizandoFase, onIrADelegaciones,
}) => {
    const {
        checkout, menuAbierto, setMenuAbierto,
        modalCancelarAbierto, setModalCancelarAbierto,
        modalReportarAbierto, setModalReportarAbierto,
        confirmarCancelacion, enviarReporte, reportando, reportError, reportExito,
        cerrarReportar, abrirCheckout, cerrarCheckout,
    } = useOrdenDetalle(order, onCancelar);

    /* [035A-30] Gestión manual de fases (solo phased, solo employee/admin) */
    const [agregandoFase, setAgregandoFase] = useState(false);
    const [eliminandoFase, setEliminadoFase] = useState<number | null>(null);

    const handleAgregarFase = useCallback(async () => {
        setAgregandoFase(true);
        try {
            await apiAddOrderPhase(order.id);
            onPagoExitoso();
        } catch {
            /* toast de error ya manejado por axios interceptor */
        } finally {
            setAgregandoFase(false);
        }
    }, [order.id, onPagoExitoso]);

    const handleEliminarFase = useCallback(async (phaseNumber: number) => {
        setEliminadoFase(phaseNumber);
        try {
            await apiDeleteOrderPhase(order.id, phaseNumber);
            onPagoExitoso();
        } catch {
            /* toast de error ya manejado por axios interceptor */
        } finally {
            setEliminadoFase(null);
        }
    }, [order.id, onPagoExitoso]);

    /* [164A-9] Solicitud de cancelación: empleados crean, clientes aceptan/rechazan */
    const {
        pendingRequest,
        creating: creandoSolicitud,
        responding: respondiendoSolicitud,
        createRequest,
        respond: respondRequest,
    } = useCancellationRequest(order.id);

    /* [164A-9] Empleado: crea solicitud en vez de cancelar directo.
     * Cliente/admin: usa cancel directo (con wallet credit). */
    const handleEmployeeCancelRequest = useCallback(async (reason?: string) => {
        if (!reason || reason.trim().length < 5) return;
        await createRequest(reason);
        setModalCancelarAbierto(false);
    }, [createRequest, setModalCancelarAbierto]);

    /* [T-10] Toggle IA intermediaria: solo visible para admin/employee */
    const [intermediaryEnabled, setIntermediaryEnabled] = useState(order.ai_intermediary_enabled);
    const [togglingIntermediary, setTogglingIntermediary] = useState(false);

    const handleToggleIntermediary = useCallback(async () => {
        setTogglingIntermediary(true);
        try {
            const updated = await apiToggleAiIntermediary(order.id, !intermediaryEnabled);
            setIntermediaryEnabled(updated.ai_intermediary_enabled);
        } catch {
            /* Rollback visual si falla */
            setIntermediaryEnabled(prev => prev);
        } finally {
            setTogglingIntermediary(false);
        }
    }, [order.id, intermediaryEnabled]);

    const isEmployee = effectiveRole === 'employee';
    const isClient = effectiveRole === 'client' || effectiveRole === 'admin';
    const isAdmin = effectiveRole === 'admin';
    const isActiveOrder = order.status !== 'cancelled' && order.status !== 'completed';
    /* [20CA-5] Admin puede cancelar en cualquier estado activo. Empleado solo solicita en in_progress. */
    const canCancel = (isAdmin && isActiveOrder && !pendingRequest)
        || (CANCELABLE_STATUSES.includes(order.status) && !pendingRequest)
        || (isEmployee && order.status === 'in_progress' && !pendingRequest);
    /* [104A-29] payment_held es el estado inicial — el botón de pago aparece aquí */
    const needsPayment = order.status === 'payment_held';
    /* [064A-60] Tanto phased como half_half usan pagos por fase individuales */
    const isPerPhasePayment = order.payment_mode === 'phased' || order.payment_mode === 'half_half';
    /* [035A-15] project_description pasa a ser campo operativo del staff.
     * El cliente mantiene visibilidad, pero solo admin o empleado asignado deben editarlo. */
    const canEditDescription = (isEmployee || isAdmin)
        && order.status !== 'completed'
        && order.status !== 'cancelled';
    const canDefinePhases = order.payment_mode === 'phased' && (isEmployee || effectiveRole === 'admin');

    /* [T2-assignment] Modal de asignación admin */
    const [modalAsignarAbierto, setModalAsignarAbierto] = useState(false);
    const canAssign = isAdmin && (order.status === 'awaiting_assignment' || order.status === 'payment_held' || order.status === 'in_progress');

    /* [016A-5] Desasignar empleado (admin) */
    const queryClient = useQueryClient();
    const desasignar = useMutation({
        mutationFn: () => apiUnassignOrder(order.id),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['ordenes'] });
            queryClient.invalidateQueries({ queryKey: ['orden-detalle', order.id] });
        },
    });

    /* [20CA-5] Refund mutation para admin */
    const { solicitarReembolso } = useRefunds();

    /* [064A-31] Chat disponible cuando hay empleado asignado y la orden está activa */
    const canChat = !!order.assigned_employee_id && isActiveOrder;

    /* [104A-28] Menú contextual con acciones según rol */
    const menuItems: MenuContextualItem[] = [];

    /* Cancelar: cliente (estados iniciales) o empleado (solicitud, si no hay pendiente) */
    if (canCancel) {
        const isBusy = isEmployee ? creandoSolicitud : cancelando;
        menuItems.push({
            id: 'cancel-order',
            label: isBusy ? 'Cancelando...' : (isEmployee ? 'Solicitar cancelación' : 'Cancelar orden'),
            onSelect: () => setModalCancelarAbierto(true),
            disabled: isBusy,
            danger: true,
            icon: <XCircle size={16} />,
        });
    }

    /* [104A-33] Reportar: clientes abren chatbot con contexto de problema,
     * empleados usan el modal directo (reportan vía API). */
    const abrirChat = useChatStore(s => s.abrir);
    if (order.assigned_employee_id && isActiveOrder) {
        menuItems.push({
            id: 'report-order',
            label: 'Reportar problema',
            onSelect: isClient
                ? () => abrirChat('problem:' + order.id)
                : () => setModalReportarAbierto(true),
            icon: <AlertTriangle size={16} />,
        });
    }

    /* Delegar: solo empleados asignados en órdenes activas */
    if (isEmployee && order.assigned_employee_id && isActiveOrder && onIrADelegaciones) {
        menuItems.push({
            id: 'delegate-order',
            label: 'Delegar orden',
            onSelect: onIrADelegaciones,
            icon: <ArrowRightLeft size={16} />,
        });
    }

    /* [T2-assignment] Asignar/reasignar: admin siempre puede asignar en estados activos */
    if (canAssign) {
        menuItems.push({
            id: 'assign-order',
            label: order.assigned_employee_id ? 'Reasignar a freelancer' : 'Asignar a freelancer',
            onSelect: () => setModalAsignarAbierto(true),
            icon: <UserCheck size={16} />,
        });
    }

    /* [016A-5] Desasignar: admin puede quitar la asignación actual */
    if (isAdmin && order.assigned_employee_id && isActiveOrder) {
        menuItems.push({
            id: 'unassign-order',
            label: 'Desasignar freelancer',
            onSelect: () => desasignar.mutate(),
            disabled: desasignar.isPending,
            icon: <UserX size={16} />,
        });
    }

    /* [20CA-5] Solicitar reembolso: admin puede iniciar reembolso desde el detalle */
    if (isAdmin && isActiveOrder && order.status !== 'payment_held') {
        menuItems.push({
            id: 'refund-order',
            label: 'Solicitar reembolso',
            onSelect: () => {
                const motivo = prompt('Motivo del reembolso:');
                if (motivo && motivo.trim()) {
                    solicitarReembolso.mutate({ orderId: order.id, reason: motivo.trim() });
                }
            },
            disabled: solicitarReembolso.isPending,
            icon: <DollarSign size={16} />,
        });
    }

    return (
        <div className="ordenDetalle">
            <div className="ordenDetalleTopbar">
                <Button className="ordenDetalleVolver" onClick={onVolver} type="button" variante="texto" tamano="pequeno">
                    <ArrowLeft size={16} /> Volver
                </Button>
            </div>

            {/* [064A-30] Cuadro de información del pedido */}
            <div className="ordenInfoCard">
                <div className="ordenInfoCardHeader">
                    <span className={`ordenDetalleBadge ${STATUS_CLASS[order.status]}`}>
                        {ORDER_STATUS_LABELS[order.status]}
                    </span>
                    <div className="ordenInfoCardAcciones">
                        {/* [T-10] Toggle IA intermediaria — solo admin/employee */}
                        {!isClient && canChat && (
                            <label className={`ordenToggleIntermediary ${togglingIntermediary ? 'ordenToggleIntermediary--disabled' : ''}`}>
                                <Bot size={14} />
                                <span className="ordenToggleLabel">IA</span>
                                <Input
                                    type="checkbox"
                                    checked={intermediaryEnabled}
                                    onChange={() => void handleToggleIntermediary()}
                                    disabled={togglingIntermediary}
                                    className="ordenToggleInput"
                                />
                                <span className="ordenToggleSwitch" />
                            </label>
                        )}
                        {needsPayment && !isPerPhasePayment && (
                            <Button
                                onClick={() => abrirCheckout(order.final_price_cents)}
                                type="button"
                                variante="secundario"
                                tamano="pequeno"
                            >
                                <CreditCard size={16} /> Pagar
                            </Button>
                        )}
                        {menuItems.length > 0 && (
                            <MenuContextual
                                abierto={menuAbierto}
                                onToggle={() => setMenuAbierto(prev => !prev)}
                                onCerrar={() => setMenuAbierto(false)}
                                items={menuItems}
                ariaLabel="Opciones de la orden"
                variante="accion"
                            />
                        )}
                    </div>
                </div>

                <OrdenInfoGrid order={order} />

                <OrderProjectDescription
                    orderId={order.id}
                    projectDescription={order.project_description}
                    canEdit={canEditDescription}
                    isSaving={actualizandoDescripcion}
                    onSave={onActualizarDescripcion}
                />

                {/* [T-10] Resumen IA del pedido — solo visible para admin/employee */}
                {!isClient && order.ai_summary && (
                    <div className="ordenAiSummary">
                        <span className="ordenAiSummaryLabel">
                            <Bot size={14} /> Resumen IA
                        </span>
                        <p className="ordenAiSummaryText">{order.ai_summary}</p>
                    </div>
                )}
            </div>

            {/* [164A-9] Banner de solicitud de cancelación pendiente.
             * [20CA-6] Admin ve acciones extendidas (reassign). */}
            {pendingRequest && (
                <CancellationBanner
                    request={pendingRequest}
                    isClient={isClient}
                    isAdmin={isAdmin}
                    responding={respondiendoSolicitud}
                    onAccept={() => void respondRequest(pendingRequest.id, true)}
                    onReject={() => void respondRequest(pendingRequest.id, false)}
                    onReassign={isAdmin ? () => setModalAsignarAbierto(true) : undefined}
                />
            )}

            <OrderDetailModals
                orderNumber={order.order_number}
                isEmployee={isEmployee}
                modalCancelarAbierto={modalCancelarAbierto}
                modalReportarAbierto={modalReportarAbierto}
                cancelando={isEmployee ? creandoSolicitud : cancelando}
                reportando={reportando}
                reportError={reportError}
                reportExito={reportExito}
                onCerrarCancelar={() => setModalCancelarAbierto(false)}
                onConfirmarCancelacion={(reason) => {
                    if (isEmployee) {
                        void handleEmployeeCancelRequest(reason);
                    } else {
                        void confirmarCancelacion(reason);
                    }
                }}
                onCerrarReportar={cerrarReportar}
                onEnviarReporte={(reason) => void enviarReporte(reason)}
            />

            {checkout && (
                <CheckoutModal
                    orderId={order.id}
                    orderNumber={order.order_number}
                    amountCents={checkout.amountCents}
                    currency={order.currency}
                    phaseNumber={checkout.phaseNumber}
                    onClose={cerrarCheckout}
                    onSuccess={() => {
                        cerrarCheckout();
                        onPagoExitoso();
                    }}
                />
            )}

            {/* [064A-30] Cuadro de historial: fases con progreso, entregas, revisiones */}
            {phases.length > 0 && (
                <div className="ordenHistorialCard">
                    <h3 className="ordenHistorialTitulo">Fases del proyecto</h3>
                    <div className="fasesTimeline">
                        {phases.map((phase, idx) => (
                            <FaseCard
                                key={phase.phase_number}
                                phase={phase}
                                orderId={order.id}
                                isLast={idx === phases.length - 1}
                                isClient={isClient}
                                isEmployee={isEmployee}
                                isPhased={isPerPhasePayment}
                                canDefinePhase={canDefinePhases}
                                isUpdatingDefinition={actualizandoFase || eliminandoFase === phase.phase_number}
                                onAprobar={onAprobar}
                                onRevision={onRevision}
                                onActualizarFase={onActualizarFase}
                                onPagarFase={(pn, amount) => abrirCheckout(amount, pn)}
                                onEliminarFase={canDefinePhases ? handleEliminarFase : undefined}
                            />
                        ))}
                    </div>
                    {/* [035A-30] Agregar fase (solo phased, solo employee/admin) */}
                    {canDefinePhases && (
                        <Button
                            type="button"
                            variante="outline"
                            tamano="pequeno"
                            onClick={handleAgregarFase}
                            disabled={agregandoFase}
                            className="ordenAgregarFaseBtn"
                        >
                            <Plus size={14} />
                            {agregandoFase ? 'Agregando...' : 'Agregar fase'}
                        </Button>
                    )}
                </div>
            )}

            {/* [154A-15d] Timeline de actividad visible para todas las partes */}
            <OrdenHistorialActividad orderId={order.id} />

            {/* [084A-2] Chat siempre abierto, debajo del historial */}
            {canChat && (
                <OrderChat orderId={order.id} />
            )}

            {order.status === 'completed' && (
                <ReviewPanel orderId={order.id} effectiveRole={effectiveRole} />
            )}

            {/* [T2-assignment] Modal de asignación de orden a empleado */}
            {canAssign && (
                <ModalAsignar
                    orderId={order.id}
                    orderNumber={order.order_number}
                    abierto={modalAsignarAbierto}
                    onCerrar={() => setModalAsignarAbierto(false)}
                    onAsignado={() => {
                        setModalAsignarAbierto(false);
                        onPagoExitoso();
                    }}
                />
            )}
        </div>
    );
};
