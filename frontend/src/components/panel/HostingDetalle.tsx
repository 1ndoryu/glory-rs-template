/* [094A-2] Página de detalle individual por hosting.
 * Muestra toda la info de una suscripción de hosting como un panel real:
 * General, Recursos, Dominio & SSL, Acceso SSH, Facturación, Eventos.
 * Patrón: igual que OrdenDetalle (back button + tabs + contenido).
 * Tabs extraídas a HostingDetalleTabs.tsx para cumplir límite 300 líneas. */

import React from 'react';
import {
    ArrowLeft, Globe, Server, Terminal,
    CreditCard, Clock, ExternalLink, Copy, Zap, MessageSquare, HardDrive, Mail,
} from 'lucide-react';
import {useHostingDetalle, type HostingDetalleTab} from '../../hooks/useHostingDetalle';
import {
    HOSTING_PLAN_LABELS,
    HOSTING_STATUS_LABELS,
    HOSTING_STATUS_CLASS,
} from '../../api/hosting';
import {Button} from '../ui/Button';
import {useChatStore} from '../../stores/chatStore';
import {
    TabGeneral, TabRecursos, TabEventos,
} from './HostingDetalleTabs';
/* [higiene] TabDominio/TabAcceso se importan de su módulo propio: HostingDetalleTabs
 * ya no los re-exporta (mezclaba re-export y lógica ejecutable: mixed-barrel-logic). */
import {TabDominio, TabAcceso} from './HostingDetalleAccess';
import {TabFacturacion} from './TabFacturacion';
import {TabBackups} from './TabBackups';
import {TabCorreo} from './TabCorreo';
import './HostingDetalle.css';

const TABS: {key: HostingDetalleTab; label: string; icon: React.ReactNode}[] = [
    {key: 'general', label: 'General', icon: <Server size={16} />},
    {key: 'recursos', label: 'Recursos', icon: <Zap size={16} />},
    {key: 'dominio', label: 'Dominio & SSL', icon: <Globe size={16} />},
    {key: 'acceso', label: 'Acceso', icon: <Terminal size={16} />},
    {key: 'facturacion', label: 'Facturación', icon: <CreditCard size={16} />},
    {key: 'backups', label: 'Respaldos', icon: <HardDrive size={16} />},
    {key: 'correo', label: 'Correo', icon: <Mail size={16} />},
    {key: 'eventos', label: 'Eventos', icon: <Clock size={16} />},
];

/* [094A-2] Copiar texto al clipboard con feedback visual.
 * Exportado para reutilizar en HostingDetalleTabs. */
export function CopyButton({text}: {text: string}) {
    const [copied, setCopied] = React.useState(false);
    const handleCopy = () => {
        navigator.clipboard.writeText(text).catch(() => {
            /* Fallback silencioso — clipboard no disponible en algunos contextos */
        });
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };
    return (
        <Button
            type="button"
            variante="texto"
            tamano="pequeno"
            className="hostingDetalleCopyBtn"
            onClick={handleCopy}
            title="Copiar"
        >
            <Copy size={14} />
            {copied && <span className="hostingDetalleCopied">Copiado</span>}
        </Button>
    );
}

/* [094A-2] Fila de info reutilizable: label + valor + acción (copiar, link).
 * Exportado para reutilizar en HostingDetalleTabs. */
export function InfoRow({label, value, copyable, link}: {
    label: string;
    value: string;
    copyable?: boolean;
    link?: string;
}) {
    return (
        <div className="hostingDetalleInfoRow">
            <span className="hostingDetalleInfoLabel">{label}</span>
            <span className="hostingDetalleInfoValue">
                {link ? (
                    <a href={link} target="_blank" rel="noopener noreferrer" className="hostingDetalleLink">
                        {value} <ExternalLink size={12} />
                    </a>
                ) : value}
                {copyable && <CopyButton text={value} />}
            </span>
        </div>
    );
}

export function HostingDetalle({
    hostingId,
    isAdmin,
    onVolver,
    onPlanChange,
    planChangeLoading,
    onProvision,
    provisionLoading,
    onRestart,
    restartLoading,
    onStop,
    stopLoading,
    onStart,
    startLoading,
}: {
    hostingId: string;
    isAdmin: boolean;
    onVolver: () => void;
    onPlanChange?: (plan: string, domain?: string) => void;
    planChangeLoading?: boolean;
    onProvision?: () => void;
    provisionLoading?: boolean;
    onRestart?: () => void;
    restartLoading?: boolean;
    onStop?: () => void;
    stopLoading?: boolean;
    onStart?: () => void;
    startLoading?: boolean;
}) {
    const {
        subscription: sub,
        isLoading,
        tabActiva,
        setTabActiva,
        sshInfo,
        domainInfo,
    } = useHostingDetalle(hostingId);

    if (isLoading || !sub) {
        return (
            <div className="hostingDetalleLoading">
                <Server size={32} strokeWidth={1.2} />
                <p>Cargando hosting...</p>
            </div>
        );
    }

    return (
        <div className="hostingDetalle">
            <div className="hostingDetalleHeader">
                <Button
                    type="button"
                    variante="texto"
                    tamano="pequeno"
                    onClick={onVolver}
                    className="hostingDetalleBack"
                >
                    <ArrowLeft size={18} /> Volver
                </Button>
                <div className="hostingDetalleHeaderInfo">
                    <h2 className="hostingDetalleTitulo">
                        {sub.domain || sub.client_name || HOSTING_PLAN_LABELS[sub.plan]}
                    </h2>
                    <div className="hostingDetalleHeaderMeta">
                        <span className={`hostingStatus ${HOSTING_STATUS_CLASS[sub.status] || ''}`}>
                            {HOSTING_STATUS_LABELS[sub.status] || sub.status}
                        </span>
                        <span className="hostingDetalleHeaderPlan">
                            {HOSTING_PLAN_LABELS[sub.plan] || sub.plan} · ${(sub.monthly_price_cents / 100).toFixed(2)}/mes
                        </span>
                        {/* [094A-7] Contactar soporte: abre el chat con contexto del hosting
                         * [084A-28] Pasa hosting:{id} como contexto para que la IA sepa de qué hosting se trata */}
                        <Button
                            type="button"
                            variante="outline"
                            tamano="pequeno"
                            className="hostingDetalleSoporte"
                            /* [165A-20] Soporte abre el ChatWidget contextual sin sacar al usuario del detalle. */
                            onClick={() => {
                                useChatStore.getState().abrir(`hosting:${sub.id}`);
                            }}
                        >
                            <MessageSquare size={14} /> Soporte
                        </Button>
                    </div>
                </div>
            </div>

            <div className="hostingDetalleTabs">
                {TABS.map(tab => (
                    <Button
                        key={tab.key}
                        type="button"
                        variante="texto"
                        className={`hostingDetalleTab ${tabActiva === tab.key ? 'hostingDetalleTab--activa' : ''}`}
                        onClick={() => setTabActiva(tab.key)}
                    >
                        {tab.icon}
                        <span className="hostingDetalleTabLabel">{tab.label}</span>
                    </Button>
                ))}
            </div>

            <div className="hostingDetalleContent">
                {tabActiva === 'general' && (
                    <TabGeneral
                        sub={sub}
                        isAdmin={isAdmin}
                        onProvision={onProvision}
                        provisionLoading={provisionLoading}
                        onRestart={onRestart}
                        restartLoading={restartLoading}
                        onStop={onStop}
                        stopLoading={stopLoading}
                        onStart={onStart}
                        startLoading={startLoading}
                    />
                )}
                {tabActiva === 'recursos' && <TabRecursos sub={sub} />}
                {tabActiva === 'dominio' && <TabDominio sub={sub} domainInfo={domainInfo} subscriptionId={sub.id} />}
                {tabActiva === 'acceso' && <TabAcceso sshInfo={sshInfo} sub={sub} />}
                {tabActiva === 'facturacion' && (
                    <TabFacturacion
                        sub={sub}
                        onPlanChange={onPlanChange}
                        planChangeLoading={planChangeLoading}
                    />
                )}
                {tabActiva === 'backups' && <TabBackups sub={sub} />}
                {tabActiva === 'correo' && <TabCorreo sub={sub} />}
        {tabActiva === 'eventos' && <TabEventos hostingId={hostingId} />}
            </div>
        </div>
    );
}
