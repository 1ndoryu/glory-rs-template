/**
 * Componente: SidebarPanel
 * Barra lateral de navegacion del panel de usuario.
 * Muestra avatar, nombre, rol y links de navegacion con iconos.
 * [044A-38 Fase 1] Tabs dinámicos por rol + botón switch-role para admin.
 */
import React, {useState, useCallback, useRef} from 'react';
import {useQuery} from '@tanstack/react-query';
import {useTranslation} from 'react-i18next';
import {FolderOpen, Receipt, User, CreditCard, ClipboardList, PackageOpen, ArrowRightLeft, MessageSquare, RotateCcw, UserCog, Server, Settings, FileEdit, AlertTriangle, Wallet, Banknote, Menu, Network, Globe, Mail, ReceiptText} from 'lucide-react';import {useClickOutside} from '../../hooks/useClickOutside';
import {useNotifications} from '../../hooks/useNotifications';
import {obtenerTabsPorRol, type SeccionPanel} from '../../data/panel';
import {useCurrentProfile} from '../../hooks/useCurrentProfile';
import {useAuthStore} from '../../stores/authStore';
import {apiSwitchRole, extraerMensajeError} from '../../api/auth';
import {apiListBillingItems} from '../../api/billing';
import {BILLING_ITEMS_KEY} from '../../hooks/useBillingItems';
import type {UserRole} from '../../api/auth';
import {toast} from '../../stores/toastStore';
import {Button} from '../ui/Button';
import OptimizedImage from '../ui/OptimizedImage';
import './SidebarPanel.css';

interface SidebarPanelProps {
    seccionActiva: SeccionPanel;
    onCambiarSeccion: (seccion: SeccionPanel) => void;
}

/* [064A-34] Mapa de iconos por seccion. Eliminados: servicios, empleados, config-servicios. */
const ICONOS_SECCION: Record<SeccionPanel, React.ElementType> = {
    'proyectos': FolderOpen,
    'pagos': Receipt,
    'perfil': User,
    'metodos-pago': CreditCard,
    'asignados': ClipboardList,
    'disponibles': PackageOpen,
    'delegaciones': ArrowRightLeft,
    'mensajes': MessageSquare,
    'todos-ordenes': ClipboardList,
    'reembolsos': RotateCcw,
    'retiros': Banknote,
    'usuarios': UserCog,
    'hosting': Server,
    'configuracion': Settings,
    'contenido': FileEdit,
    'problemas': AlertTriangle,
    'wallet': Wallet,
    /* [304A-1] Infraestructura: ícono Network para servidores/despliegues */
    'infraestructura': Network,
    /* [304A-3] Dominios Contabo */
    'dominios': Globe,
    /* [311A-1] Trazabilidad de correos */
    'correos': Mail,
    /* [026B-1] Cobros pendientes admin */
    'cobros': ReceiptText,
};

/* [044A-38 Fase 1] Etiquetas legibles para cada rol */
const ROLE_LABELS: Record<UserRole, string> = {
    admin: 'Admin',
    employee: 'Freelancer',
    client: 'Cliente',
};

export const SidebarPanel: React.FC<SidebarPanelProps> = ({seccionActiva, onCambiarSeccion}) => {
    const {t} = useTranslation();
    const {perfil, avatarUrl} = useCurrentProfile();
    const authUser = useAuthStore(s => s.user);
    const actualizarRol = useAuthStore(s => s.actualizarRol);
    const logout = useAuthStore(s => s.logout);
    const [switchingRole, setSwitchingRole] = useState(false);

    const effectiveRole: UserRole = authUser?.effectiveRole || 'client';
    const isAdmin = authUser?.role === 'admin';
    const isImpersonating = authUser?.impersonating ?? false;
    const tabs = obtenerTabsPorRol(effectiveRole);
    const {data: billingItems = []} = useQuery({
        queryKey: BILLING_ITEMS_KEY,
        queryFn: apiListBillingItems,
        enabled: effectiveRole === 'client',
        staleTime: 30_000,
    });
    const pendingBillingCount = billingItems.filter(item => item.status === 'pending').length;

    /* [277A-2] Badge de notificaciones: usa useNotifications() que hace fetch
     * REST inicial + polling 30s + actualización via WS. Antes usaba un query
     * con enabled=false que nunca cargaba el conteo al recargar la página. */
    const { unreadCount: unreadNotifCount } = useNotifications();

    /* [114A-9] Nav inferior móvil: muestra 4 items + botón "Más" para overflow */
    const MAX_BOTTOM_NAV = 4;
    const [menuAbierto, setMenuAbierto] = useState(false);
    const menuRef = useRef<HTMLDivElement>(null);
    const visibleTabs = tabs.slice(0, MAX_BOTTOM_NAV);
    const overflowTabs = tabs.slice(MAX_BOTTOM_NAV);
    const overflowTieneHostingPendiente = pendingBillingCount > 0
        && overflowTabs.some(tab => tab.id === 'hosting');

    /* [124A-SENT-R7] Cerrar el menú overflow al hacer click/touch fuera. */
    useClickOutside(menuRef, () => setMenuAbierto(false), menuAbierto);

    const handleOverflowSelect = useCallback((seccion: SeccionPanel) => {
        onCambiarSeccion(seccion);
        setMenuAbierto(false);
    }, [onCambiarSeccion]);

    /* [084A-1] Cicla entre los 3 roles impersonando usuarios reales.
     * Admin → employee → client → admin. El backend busca un usuario real con ese rol. */
    const handleSwitchRole = useCallback(async () => {
        if (switchingRole || (!isAdmin && !isImpersonating)) return;
        const cycle: UserRole[] = ['admin', 'employee', 'client'];
        const currentIdx = cycle.indexOf(effectiveRole);
        const nextRole = cycle[(currentIdx + 1) % cycle.length];

        setSwitchingRole(true);
        try {
            const resp = await apiSwitchRole(nextRole);
            actualizarRol(resp.token, resp.user_id, resp.role, resp.effective_role, resp.impersonating);
            /* Resetear a la primera tab del nuevo rol */
            const newTabs = obtenerTabsPorRol(resp.effective_role);
            if (newTabs.length > 0) {
                onCambiarSeccion(newTabs[0].id);
            }
        } catch (error) {
            const message = extraerMensajeError(error);
            const status = typeof error === 'object' && error !== null && 'response' in error
                ? (error as {response?: {status?: number}}).response?.status
                : undefined;

            /* [035A-21] Si el backend detecta una impersonación huérfana tras reseed,
             * limpiamos la sesión local para evitar un bucle de 500 en el navegador. */
            if (status === 401 || (status === 403 && message.includes('sesión de impersonación'))) {
                logout();
            }

            toast.error(message);
            console.error('[SidebarPanel] Error al cambiar rol', error);
        } finally {
            setSwitchingRole(false);
        }
    }, [switchingRole, isAdmin, isImpersonating, effectiveRole, actualizarRol, logout, onCambiarSeccion]);

    return (
        <aside className="panelSidebar" aria-label={t('accessibility.panel_nav')}>
            {/* Info usuario */}
            <div className="sidebarUsuario">
                <div className="sidebarAvatar">
                    <OptimizedImage
                        src={avatarUrl}
                        alt="Avatar"
                        loading="eager"
                    />
                </div>
                <div className="sidebarInfo">
                    <span className="sidebarNombre">{perfil?.display_name || authUser?.email || 'Usuario'}</span>
                    <span className="sidebarRol">{ROLE_LABELS[effectiveRole]}</span>
                </div>
            </div>

            {/* [114A-9] Desktop: todos los items. Móvil: nav dentro de sidebarNav */}
            <nav className="sidebarNav" aria-label={t('accessibility.panel_sections')}>
                {/* Desktop: renderizar todos los tabs normalmente */}
                {tabs.map(tab => {
                    const Icono = ICONOS_SECCION[tab.id];
                    return (
                        <Button
                            key={tab.id}
                            className={`sidebarItem sidebarItemDesktop ${seccionActiva === tab.id ? 'sidebarItemActivo' : ''}`}
                            onClick={() => onCambiarSeccion(tab.id)}
                            aria-current={seccionActiva === tab.id ? 'page' : undefined}
                            variante="texto"
                        >
                            <Icono size={18} className="sidebarItemIcono" aria-hidden="true" />
                            <span className="sidebarItemTexto">{tab.label}</span>
                            {tab.id === 'hosting' && pendingBillingCount > 0 && (
                                <span className="sidebarItemIndicador" aria-label={`${pendingBillingCount} pagos pendientes`} />
                            )}
                            {tab.id === 'mensajes' && unreadNotifCount > 0 && (
                                <span className="sidebarItemIndicador" aria-label={`${unreadNotifCount} mensajes sin leer`} />
                            )}
                        </Button>
                    );
                })}

                {/* [114A-9] Móvil: solo los primeros N items + botón "Más" */}
                {visibleTabs.map(tab => {
                    const Icono = ICONOS_SECCION[tab.id];
                    return (
                        <Button
                            key={`m-${tab.id}`}
                            className={`sidebarItem sidebarItemMovil ${seccionActiva === tab.id ? 'sidebarItemActivo' : ''}`}
                            onClick={() => onCambiarSeccion(tab.id)}
                            aria-current={seccionActiva === tab.id ? 'page' : undefined}
                            variante="texto"
                            title={tab.label}
                        >
                            <Icono size={20} className="sidebarItemIcono" aria-hidden="true" />
                            {tab.id === 'hosting' && pendingBillingCount > 0 && (
                                <span className="sidebarItemIndicador sidebarItemIndicadorMovil" aria-label={`${pendingBillingCount} pagos pendientes`} />
                            )}
                            {tab.id === 'mensajes' && unreadNotifCount > 0 && (
                                <span className="sidebarItemIndicador sidebarItemIndicadorMovil" aria-label={`${unreadNotifCount} mensajes sin leer`} />
                            )}
                        </Button>
                    );
                })}

                {/* [114A-9] Botón "Más" en móvil si hay items overflow */}
                {overflowTabs.length > 0 && (
                    <div className="sidebarOverflowContenedor" ref={menuRef}>
                        <Button
                            className={`sidebarItem sidebarItemMovil sidebarOverflowBtn ${
                                overflowTabs.some(t => t.id === seccionActiva) ? 'sidebarItemActivo' : ''
                            }`}
                            onClick={() => setMenuAbierto(prev => !prev)}
                            variante="texto"
                            title="Más opciones"
                        >
                            <Menu size={20} className="sidebarItemIcono" aria-hidden="true" />
                            {overflowTieneHostingPendiente && (
                                <span className="sidebarItemIndicador sidebarItemIndicadorMovil" aria-label={`${pendingBillingCount} pagos pendientes`} />
                            )}
                        </Button>

                        {menuAbierto && (
                            <div className="sidebarOverflowMenu">
                                {overflowTabs.map(tab => {
                                    const Icono = ICONOS_SECCION[tab.id];
                                    return (
                                        <Button
                                            key={tab.id}
                                            className={`sidebarOverflowItem ${seccionActiva === tab.id ? 'sidebarItemActivo' : ''}`}
                                            onClick={() => handleOverflowSelect(tab.id)}
                                            variante="texto"
                                        >
                                            <Icono size={16} className="sidebarItemIcono" aria-hidden="true" />
                                            <span>{tab.label}</span>
                                            {tab.id === 'hosting' && pendingBillingCount > 0 && (
                                                <span className="sidebarItemIndicador" aria-label={`${pendingBillingCount} pagos pendientes`} />
                                            )}
                                        </Button>
                                    );
                                })}
                            </div>
                        )}
                    </div>
                )}
            </nav>

            {/* [084A-1] Botón switch-role — visible para admin real o cuando impersonando */}
            {(isAdmin || isImpersonating) && (
                <div className="sidebarSwitchRole">
                    <Button
                        className="sidebarSwitchBtn"
                        onClick={handleSwitchRole}
                        disabled={switchingRole}
                        title={`Cambiar vista a otro rol (actual: ${ROLE_LABELS[effectiveRole]})`}
                        variante="outline"
                    >
                        <ArrowRightLeft size={16} aria-hidden="true" />
                        <span>{switchingRole ? 'Cambiando...' : `Vista: ${ROLE_LABELS[effectiveRole]}`}</span>
                    </Button>
                    {isImpersonating && (
                        <span className="sidebarImpersonando">Impersonando</span>
                    )}
                </div>
            )}
        </aside>
    );
};
