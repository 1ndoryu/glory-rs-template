/* [304A-3] Sección Dominios del panel admin.
 * Muestra dominios registrados en Contabo para admin y checkout self-service para clientes.
 * [195A-1] Clientes pueden cotizar disponibilidad y pagar dominios con margen mínimo. */

import React, {useState} from 'react';
import {Globe, RefreshCw, Search} from 'lucide-react';
import {useQuery} from '@tanstack/react-query';
import {
    apiCheckDomainAvailability,
    apiCreateDomainCheckout,
    apiListDomainOrders,
    apiListDomains,
    type ContaboDomain,
    type DomainAvailability,
    type DomainOrder,
} from '../../api/hosting';
import {useAuthStore} from '../../stores/authStore';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {redirigir} from '../../platform/navigation';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import './SeccionDominios.css';

interface DomainUiState {
    search: string;
    quote: DomainAvailability | null;
    error: string;
    loading: boolean;
}

function formatMoney(cents: number): string {
    return `$${(cents / 100).toFixed(cents % 100 === 0 ? 0 : 2)}`;
}

function domainStatusLabel(status: string): string {
    switch (status) {
        case 'pending_payment': return 'Pendiente de pago';
        case 'paid_pending_registration': return 'Pagado, pendiente de registro';
        case 'registered': return 'Registrado';
        case 'cancelled': return 'Cancelado';
        default: return status;
    }
}

function DomainCard({domain}: {domain: ContaboDomain}) {
    const nombre = [domain.sld, domain.tld].filter(Boolean).join('.');
    const estado = domain.status ?? 'unknown';
    const vence = domain.paidUntil
        ? new Date(domain.paidUntil).toLocaleDateString('es')
        : null;

    return (
        <div className="dominioCard">
            <div className="dominioCardHeader">
                <Globe size={18} strokeWidth={1.4} className="dominioCardIcono" />
                <span className="dominioCombre">{nombre || '(sin nombre)'}</span>
                <span className={`dominioEstado dominioEstado--${estado.toLowerCase()}`}>
                    {estado}
                </span>
            </div>
            {vence && (
                <p className="dominioVence">Vence: {vence}</p>
            )}
            {domain.nameservers && domain.nameservers.length > 0 && (
                <div className="dominioNs">
                    {domain.nameservers.slice(0, 2).map((ns) => (
                        <span key={ns.hostname} className="dominioNsItem">{ns.hostname}</span>
                    ))}
                    {domain.nameservers.length > 2 && (
                        <span className="dominioNsItem">+{domain.nameservers.length - 2} más</span>
                    )}
                </div>
            )}
        </div>
    );
}

function DomainOrderCard({order}: {order: DomainOrder}) {
    return (
        <div className="dominioCard">
            <div className="dominioCardHeader">
                <Globe size={18} strokeWidth={1.4} className="dominioCardIcono" />
                <span className="dominioCombre">{order.domain}</span>
                <span className={`dominioEstado dominioEstado--${order.status}`}>
                    {domainStatusLabel(order.status)}
                </span>
            </div>
            <p className="dominioVence">Precio anual: {formatMoney(order.price_cents)}</p>
        </div>
    );
}

export const SeccionDominios: React.FC = () => {
    const effectiveRole = useAuthStore(s => s.user?.effectiveRole) || 'client';
    const isAdmin = effectiveRole === 'admin';
    const [ui, setUi] = useState<DomainUiState>({search: '', quote: null, error: '', loading: false});

    const {data: dominios = [], isLoading, isError, refetch, isFetching} = useQuery({
        queryKey: ['admin-domains'],
        queryFn: apiListDomains,
        enabled: isAdmin,
        staleTime: 2 * 60 * 1000, /* 2 minutos — Contabo API es lenta */
    });
    const {data: orders = [], refetch: refetchOrders} = useQuery({
        queryKey: ['domain-orders', effectiveRole],
        queryFn: apiListDomainOrders,
        staleTime: 60 * 1000,
    });

    const handleCheck = async () => {
        const domain = ui.search.trim();
        if (!domain) return;
        setUi(prev => ({...prev, loading: true, error: '', quote: null}));
        try {
            const quote = await apiCheckDomainAvailability(domain);
            setUi(prev => ({...prev, loading: false, quote}));
        } catch (error) {
            const message = error instanceof Error ? error.message : 'No se pudo consultar el dominio.';
            setUi(prev => ({...prev, loading: false, error: message}));
        }
    };

    const handleCheckout = async () => {
        if (!ui.quote?.available || ui.loading) return;
        setUi(prev => ({...prev, loading: true, error: ''}));
        try {
            const response = await apiCreateDomainCheckout(ui.quote.domain);
            await refetchOrders();
            redirigir(response.checkout_url);
        } catch (error) {
            const message = error instanceof Error ? error.message : 'No se pudo iniciar el checkout del dominio.';
            setUi(prev => ({...prev, loading: false, error: message}));
        }
    };

    return (
        <div className="dominiosContenedor">
            <div className="dominiosCompra">
                <div className="dominiosCompraHeader">
                    <Globe size={20} strokeWidth={1.4} />
                    <div>
                        <h2 className="dominiosTitulo">Comprar dominio</h2>
                        <p className="dominiosSubtitulo">Precio anual con margen operativo mínimo incluido.</p>
                    </div>
                </div>
                <div className="dominiosBuscador">
                    <Input
                        type="text"
                        value={ui.search}
                        onChange={event => setUi(prev => ({...prev, search: event.target.value}))}
                        placeholder="tudominio.com"
                    />
                    <Button type="button" variante="outline" tamano="pequeno" onClick={handleCheck} disabled={ui.loading}>
                        <Search size={14} />
                        Buscar
                    </Button>
                </div>
                {ui.error && <p className="dominiosErrorTexto">{ui.error}</p>}
                {ui.quote && (
                    <div className="dominiosQuote">
                        <span>{ui.quote.domain}</span>
                        <strong>{ui.quote.available ? formatMoney(ui.quote.price_cents) : 'No disponible'}</strong>
                        <Button
                            type="button"
                            variante="primario"
                            tamano="pequeno"
                            onClick={handleCheckout}
                            disabled={!ui.quote.available || ui.loading}
                        >
                            Comprar dominio
                        </Button>
                    </div>
                )}
            </div>

            {orders.length > 0 && (
                <div className="dominiosLista">
                    {orders.map(order => <DomainOrderCard key={order.id} order={order} />)}
                </div>
            )}

            {isAdmin && (
                <>
            <div className="dominiosAcciones">
                <Button
                    type="button"
                    variante="texto"
                    tamano="pequeno"
                    onClick={() => refetch()}
                    disabled={isFetching}
                    title="Recargar dominios desde Contabo"
                >
                    <RefreshCw size={14} className={isFetching ? 'dominiosSpinning' : ''} />
                </Button>
            </div>

            {isLoading && (
                <div className="dominiosCargando">
                    <p>Cargando dominios desde Contabo…</p>
                </div>
            )}

            {isError && (
                <div className="dominiosError">
                    <p>Error al cargar dominios. Verifica que Contabo API esté configurada.</p>
                    <Button type="button" variante="outline" tamano="pequeno" onClick={() => refetch()}>
                        Reintentar
                    </Button>
                </div>
            )}

            {!isLoading && !isError && dominios.length === 0 && (
                <div className="dominiosVacio">
                    <Globe size={40} strokeWidth={1.2} />
                    <p>No hay dominios registrados en Contabo.</p>
                </div>
            )}

            {!isLoading && dominios.length > 0 && (
                <div className="dominiosLista">
                    {dominios.map((d, i) => (
                        <DomainCard key={`${d.sld}-${d.tld}-${i}`} domain={d} />
                    ))}
                </div>
            )}
                </>
            )}
        </div>
    );
};
