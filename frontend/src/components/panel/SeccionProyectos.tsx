/* [259A-5] sentinel-disable-file menu-contextual-override-diseno: variantes por instancia via props API publica del DS; variantes canon al sistema en fase 5b. */
/* [044A-38 Fase 2] Sección "Mis Proyectos" del panel.
 * Muestra lista de órdenes con progreso visual + detalle con fases.
 * Vista lista ↔ detalle. Detalle extraído a OrdenDetalle.tsx (SRP).
 * [064A-50] Tabs Activas/Historial. Activas ordenadas por prioridad de status.
 * Canceladas y completadas van a Historial.
 * [084A-1] Admin: búsqueda, filtro por empleado, empleado en card footer.
 * sentinel-disable-file componente-sin-hook: la lógica son constantes de ordenamiento y filtros
 * que no son reutilizables; data ya está en useOrdenes.
 * sentinel-disable-file usestate-excesivo: 4 useState — tab, búsqueda, filtroEmpleado,
 * menuAbierto son estados UI independientes del componente lista. */
import React, {useMemo} from 'react';
import {FolderOpen, Search, ChevronDown, MessageSquare} from 'lucide-react';
import {useQuery} from '@tanstack/react-query';
import {useSeccionProyectos} from '../../hooks/useSeccionProyectos';
import {OrdenDetalle} from './OrdenDetalle';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import {MenuContextual} from '../ui/ContextMenu';
import OptimizedImage from '../ui/OptimizedImage';
import {apiListChatSessions} from '../../api/chat';
import {
    ORDER_STATUS_LABELS,
    PAYMENT_MODE_LABELS,
    formatPrice,
    type OrderResponse,
} from '../../api/orders';
import {getServiceImage} from '../../utils/serviceImages';
import './SeccionProyectos.css';

/* Mapa status → clase CSS (evita inline style) */
const STATUS_CLASS: Record<string, string> = {
    pending_payment: 'ordenBadge--paymentHeld',
    payment_held: 'ordenBadge--paymentHeld',
    awaiting_assignment: 'ordenBadge--awaiting',
    in_progress: 'ordenBadge--inProgress',
    under_review: 'ordenBadge--underReview',
    completed: 'ordenBadge--completed',
    cancelled: 'ordenBadge--cancelled',
    disputed: 'ordenBadge--disputed',
};

export const SeccionProyectos: React.FC = () => {
    const {
        effectiveRole,
        isAdmin,
        tabActiva,
        setTabActiva,
        busqueda,
        setBusqueda,
        filtroEmpleado,
        setFiltroEmpleado,
        empleadoMenuAbierto,
        setEmpleadoMenuAbierto,
        ordenes, cargando, detalle, cargandoDetalle,
        ordenSeleccionada, error, handleSelectOrder, recargar,
        cancelando, actualizandoDescripcion, actualizandoFase,
        activas, historial, empleadosUnicos, listaActual,
        handleVolver, handleCancelar, handleAprobar, handleRevision,
        handleActualizarDescripcion, handleActualizarFase,
    } = useSeccionProyectos();

    /* [154A-13] Reutilizar cache de chat-sessions (ChatBell lo pollea cada 15s)
     * para detectar órdenes con mensajes sin leer y mostrar badge en tarjetas. */
    const {data: chatSessions = []} = useQuery({
        queryKey: ['chat-sessions'],
        queryFn: apiListChatSessions,
        refetchInterval: 15_000,
    });

    const ordenesConNoLeidos = useMemo(() => {
        const ids = new Set<string>();
        for (const s of chatSessions) {
            if (!s.order_id || !s.last_message_at) continue;
            if (!s.last_viewed_at || new Date(s.last_message_at) > new Date(s.last_viewed_at)) {
                ids.add(s.order_id);
            }
        }
        return ids;
    }, [chatSessions]);

    if (cargando) {
        return <div className="proyectosLoading"><div className="proyectosSpinner" /><p>Cargando proyectos...</p></div>;
    }
    if (error) {
        return <div className="proyectosError">{error}</div>;
    }

    /* Vista detalle */
    if (ordenSeleccionada && detalle) {
        return (
            <OrdenDetalle
                order={detalle.order}
                phases={detalle.phases}
                effectiveRole={effectiveRole}
                onVolver={handleVolver}
                onCancelar={handleCancelar}
                onAprobar={handleAprobar}
                onRevision={handleRevision}
                onActualizarDescripcion={handleActualizarDescripcion}
                onActualizarFase={handleActualizarFase}
                onPagoExitoso={recargar}
                cancelando={cancelando}
                actualizandoDescripcion={actualizandoDescripcion}
                actualizandoFase={actualizandoFase}
            />
        );
    }
    if (ordenSeleccionada && cargandoDetalle) {
        return <div className="proyectosLoading"><div className="proyectosSpinner" /><p>Cargando detalle...</p></div>;
    }

    /* Lista vacía (todas las órdenes, no solo la tab) */
    if (ordenes.length === 0) {
        return <EstadoVacioProyectos variante="sinOrdenes" />;
    }

    /* Lista de órdenes con tabs */
    return (
        <div className="proyectosContenedor">
            {/* [084A-1] Barra de búsqueda y filtro — solo admin */}
            {isAdmin && (
                <div className="proyectosFiltros">
                    <div className="proyectosBusqueda">
                        <Search size={16} className="proyectosBusquedaIcono" />
                        <Input
                            type="text"
                            className="proyectosBusquedaInput"
                            placeholder="Buscar por servicio..."
                            value={busqueda}
                            onChange={e => setBusqueda(e.target.value)}
                        />
                    </div>
                    {empleadosUnicos.length > 0 && (
                        <MenuContextual
                            abierto={empleadoMenuAbierto}
                            onToggle={() => setEmpleadoMenuAbierto(prev => !prev)}
                            onCerrar={() => setEmpleadoMenuAbierto(false)}
                            ariaLabel="Filtrar por freelancer"
                            triggerClassName="proyectosFiltroEmpleado"
                            triggerVariante="outline"
                            triggerTamano="pequeno"
                            triggerContent={<>{filtroEmpleado ? empleadosUnicos.find(e => e.id === filtroEmpleado)?.nombre ?? 'Freelancer' : 'Todos los freelancers'} <ChevronDown size={14} /></>}
                            items={[
                                {id: 'all', label: 'Todos los freelancers', onSelect: () => setFiltroEmpleado('')},
                                ...empleadosUnicos.map(emp => ({
                                    id: emp.id,
                                    label: emp.nombre,
                                    onSelect: () => setFiltroEmpleado(emp.id),
                                })),
                            ]}
                        />
                    )}
                </div>
            )}

            <div className="proyectosTabs">
                <Button
                    type="button"
                    variante="texto"
                    className={`proyectosTab ${tabActiva === 'activas' ? 'proyectosTab--activa' : ''}`}
                    onClick={() => setTabActiva('activas')}
                >
                    Activas ({activas.length})
                </Button>
                <Button
                    type="button"
                    variante="texto"
                    className={`proyectosTab ${tabActiva === 'historial' ? 'proyectosTab--activa' : ''}`}
                    onClick={() => setTabActiva('historial')}
                >
                    Historial ({historial.length})
                </Button>
            </div>

            {listaActual.length === 0 ? (
                <EstadoVacioProyectos
                    variante={tabActiva === 'activas' ? 'sinActivas' : 'sinHistorial'}
                />
            ) : (
                <div className="proyectosLista">
                    {listaActual.map(orden => (
                        <OrdenCard
                            key={orden.id}
                            orden={orden}
                            onClick={() => handleSelectOrder(orden.id)}
                            mostrarEmpleado={isAdmin}
                            tieneNoLeidos={ordenesConNoLeidos.has(orden.id)}
                        />
                    ))}
                </div>
            )}
        </div>
    );
};

type EstadoVacioProyectosVariante = 'sinOrdenes' | 'sinActivas' | 'sinHistorial';

function EstadoVacioProyectos({variante}: {variante: EstadoVacioProyectosVariante}) {
    const contenido: Record<EstadoVacioProyectosVariante, {titulo: string; texto: string}> = {
        sinOrdenes: {
            titulo: 'Sin proyectos aún',
            texto: 'Cuando contrates un servicio, aparecerá aquí con seguimiento en tiempo real.',
        },
        sinActivas: {
            titulo: 'Sin proyectos activos',
            texto: 'Cuando una orden avance más allá del historial, aparecerá aquí para que sigas su estado.',
        },
        sinHistorial: {
            titulo: 'Sin historial todavía',
            texto: 'Las órdenes completadas o canceladas aparecerán aquí cuando ya no estén activas.',
        },
    };

    return (
        <div className="proyectosVacio">
            <FolderOpen size={48} className="proyectosVacioIcono" strokeWidth={1.2} />
            <h3 className="proyectosVacioTitulo">{contenido[variante].titulo}</h3>
            <p className="proyectosVacioTexto">{contenido[variante].texto}</p>
        </div>
    );
}

/* Sub-componente: card de orden en la lista */
/* [054A-6] Rediseño: quita numeración, plan_name, barra de progreso.
 * Agrega imagen del servicio resuelta por slug.
 * [084A-1] Vista admin muestra empleado responsable en footer.
 * [074A-59] Admin/empleado ven client_name en footer. */
function OrdenCard({orden, onClick, mostrarEmpleado, tieneNoLeidos}: {orden: OrderResponse; onClick: () => void; mostrarEmpleado?: boolean; tieneNoLeidos?: boolean}) {
    const imgSrc = getServiceImage(orden.service_slug);

    return (
        <Button className="ordenCard" onClick={onClick} type="button" variante="texto">
            <div className="ordenCardImagenWrapper">
                <OptimizedImage
                    className="ordenCardImagen"
                    src={imgSrc}
                    alt={orden.service_title}
                    loading="lazy"
                />
                {/* [154A-13] Badge de mensajes no leídos en la tarjeta */}
                {tieneNoLeidos && (
                    <span className="ordenCardChatBadge" title="Mensajes sin leer">
                        <MessageSquare size={14} />
                    </span>
                )}
            </div>
            <div className="ordenCardBody">
                <div className="ordenCardHeader">
                    <h3 className="ordenCardTitulo">{orden.service_title}</h3>
                    <span className={`ordenCardBadge ${STATUS_CLASS[orden.status] || ''}`}>
                        {ORDER_STATUS_LABELS[orden.status]}
                    </span>
                </div>
                <div className="ordenCardFooter">
                    <span className="ordenCardPrecio">{formatPrice(orden.final_price_cents, orden.currency)}</span>
                    <span className="ordenCardModo">{PAYMENT_MODE_LABELS[orden.payment_mode]}</span>
                    {mostrarEmpleado && orden.client_name && (
                        <span className="ordenCardCliente">{orden.client_name}</span>
                    )}
                    {mostrarEmpleado && (
                        <span className="ordenCardEmpleado">
                            {orden.assigned_employee_name ?? 'Sin asignar'}
                        </span>
                    )}
                    <span className="ordenCardFecha">{new Date(orden.created_at).toLocaleDateString('es')}</span>
                </div>
            </div>
        </Button>
    );
}
