/* Island: PanelIsland
 * Panel de usuario con header custom (sin header/footer global) y sidebar lateral.
 * [044A-38 Fase 1] Secciones dinámicas por rol (admin/employee/client).
 * Redirige a / si no hay sesión activa. Tabs y sección inicial dependen del effectiveRole.
 * sentinel-disable-file componente-sin-hook limite-lineas: PanelIsland es orquestador de layout — su lógica
 * (redirect + tab-switch) es minimal y específica del routing, no reutilizable en hook. */
import React, {useState, useEffect} from 'react';
import {useLocation, useNavigate} from 'react-router-dom';
import {HeaderPanel} from '../components/panel/HeaderPanel';
import {SeccionPerfil} from '../components/panel/SeccionPerfil';
import {SeccionMetodosPago} from '../components/panel/SeccionMetodosPago';
import {SeccionProyectos} from '../components/panel/SeccionProyectos';
import {SeccionPagos} from '../components/panel/SeccionPagos';
import {SeccionDisponibles} from '../components/panel/SeccionDisponibles';
import {SeccionDelegaciones} from '../components/panel/SeccionDelegaciones';
import {SeccionChat} from '../components/panel/SeccionChat';
import {SeccionReembolsos} from '../components/panel/SeccionReembolsos';
import {SeccionUsuarios} from '../components/panel/SeccionUsuarios';
import {SeccionHosting} from '../components/panel/SeccionHosting';
import {SeccionConfiguracion} from '../components/panel/SeccionConfiguracion';
import {SeccionContenido} from '../components/panel/SeccionContenido';
import {SeccionProblemas} from '../components/panel/SeccionProblemas';
import {SeccionWallet} from '../components/panel/SeccionWallet';
import {SeccionRetiros} from '../components/panel/SeccionRetiros';
import {SeccionInfraestructura} from '../components/panel/SeccionInfraestructura';
import {SeccionDominios} from '../components/panel/SeccionDominios';
import {SeccionCorreo} from '../components/panel/SeccionCorreo';
import {SeccionCobros} from '../components/panel/SeccionCobros';
import {SeccionSeo} from '../components/panel/SeccionSeo';
/* [064A-34] EmployeesSection y ServicesCatalogSection eliminados del panel. */
import {SidebarPanel} from '../components/panel/SidebarPanel';
import {PlaceholderSeccion} from '../components/panel/PlaceholderSeccion';
/* [154A-5] Banner para usuarios sin contraseña (quick_register) */
import {BannerPassword} from '../components/panel/BannerPassword';
import {PANEL_TAB_KEY, obtenerTabsPorRol, seccionInicialPorRol, type SeccionPanel} from '../data/panel';
import {useAuthStore} from '../stores/authStore';
import {SEOHead} from '../components/seo/SEOHead';
import type {UserRole} from '../api/auth';
import {resolvePanelSectionFromUrl, syncPanelSectionInUrl} from '../utils/panelUrlState';
import '../styles/variables.css';
import './PanelIsland.css';

export const PanelIsland: React.FC = () => {
    const logueado = useAuthStore(s => s.logueado);
    const effectiveRole: UserRole = useAuthStore(s => s.user?.effectiveRole) || 'client';
    const navigate = useNavigate();
    const location = useLocation();

    const tabs = obtenerTabsPorRol(effectiveRole);

    /* [084A-9] Restaurar tab desde localStorage para persistir entre recargas y cierres de pestaña.
     * [154A-12b] Cambiado de sessionStorage a localStorage — el usuario reportó que se perdía al recargar. */
    const [seccionActiva, setSeccionActiva] = useState<SeccionPanel>(() => {
        const desdeUrl = resolvePanelSectionFromUrl(
            effectiveRole,
            obtenerTabsPorRol(effectiveRole).map(tab => tab.id),
        );
        if (desdeUrl) return desdeUrl;
        const stored = localStorage.getItem(PANEL_TAB_KEY) as SeccionPanel | null;
        const tabsRol = obtenerTabsPorRol(effectiveRole);
        if (stored && tabsRol.some(t => t.id === stored)) return stored;
        return seccionInicialPorRol(effectiveRole);
    });

    /* [044A-38 Fase 1] Si no hay sesión, redirigir al home */
    useEffect(() => {
        if (!logueado) {
            navigate('/', {replace: true});
        }
    }, [logueado, navigate]);

    /* [044A-38 Fase 1] Cuando cambia el rol efectivo, solo resetear si el tab guardado no es
     * válido para el nuevo rol. Antes reseteaba incondicionalmente y destruía la persistencia
     * en cada recarga — el bug reportado de "tabs siempre regresan a la primera". [164A-1] */
    useEffect(() => {
        const desdeUrl = resolvePanelSectionFromUrl(
            effectiveRole,
            obtenerTabsPorRol(effectiveRole).map(tab => tab.id),
        );
        if (desdeUrl) {
            localStorage.setItem(PANEL_TAB_KEY, desdeUrl);
            setSeccionActiva(desdeUrl);
            return;
        }

        const stored = localStorage.getItem(PANEL_TAB_KEY) as SeccionPanel | null;
        const tabsRol = obtenerTabsPorRol(effectiveRole);
        if (stored && tabsRol.some(t => t.id === stored)) {
            setSeccionActiva(stored);
            return;
        }
        const inicial = seccionInicialPorRol(effectiveRole);
        localStorage.setItem(PANEL_TAB_KEY, inicial);
        setSeccionActiva(inicial);
    }, [effectiveRole, location.search]);

    /* [084A-9] Persistir tab activa en localStorage para sobrevivir recargas y cierres.
     * [154A-12b] Cambiado de sessionStorage a localStorage. */
    useEffect(() => {
        localStorage.setItem(PANEL_TAB_KEY, seccionActiva);
        syncPanelSectionInUrl(seccionActiva);
    }, [seccionActiva]);

    /* [064A-5] Escuchar custom event desde HeaderPanel para cambiar tab */
    useEffect(() => {
        const handler = (e: Event) => {
            const tab = (e as CustomEvent).detail as SeccionPanel;
            if (tab) setSeccionActiva(tab);
        };
        window.addEventListener('panel-cambiar-tab', handler);
        return () => window.removeEventListener('panel-cambiar-tab', handler);
    }, []);

    const tabActual = tabs.find(t => t.id === seccionActiva) || tabs[0];

    /* Renderizar contenido segun seccion activa */
    const renderContenido = () => {
        switch (seccionActiva) {
            case 'perfil':
                return <SeccionPerfil />;
            case 'metodos-pago':
                return <SeccionMetodosPago />;
            /* [044A-38 Fase 2] Mis Proyectos con lista de órdenes + detalle + acciones */
            case 'proyectos':
            case 'asignados':
            case 'todos-ordenes':
                return <SeccionProyectos />;
            /* [044A-38 Fase 3] Historial de pagos por orden */
            case 'pagos':
                return <SeccionPagos />;
            /* [044A-38 Fase 4] Órdenes disponibles para tomar (empleado) */
            case 'disponibles':
                return <SeccionDisponibles />;
            /* [044A-38 Fase 4] Delegaciones entre empleados */
            case 'delegaciones':
                return <SeccionDelegaciones />;
            /* [044A-38 Fase 5] Chat integrado con ordenes */
            case 'mensajes':
                return <SeccionChat />;
            /* [044A-38 Fase 7] Reembolsos (admin) */
            case 'reembolsos':
                return <SeccionReembolsos />;
            /* [054A-1] Gestión de usuarios registrados (admin) */
            case 'usuarios':
                return <SeccionUsuarios />;
            /* [054A-2] Hosting: suscripciones, planes, estados, eventos */
            case 'hosting':
                return <SeccionHosting />;
            /* [064A-62] Configuración: seed, herramientas dev (admin) */
            case 'configuracion':
                return <SeccionConfiguracion />;
            /* [074A-7] CMS: editor de contenido editorial (admin) */
            case 'contenido':
                return <SeccionContenido />;
            /* [104A-28] Problemas reportados (admin) */
            case 'problemas':
                return <SeccionProblemas />;
            /* [154A-15a] Wallet / Mi Saldo */
            case 'wallet':
                return <SeccionWallet />;
            /* [T1-withdrawal] Retiros: gestión admin */
            case 'retiros':
                return <SeccionRetiros />;
            /* [304A-1] Infraestructura: despliegues Coolify + servidores Contabo (admin) */
            case 'infraestructura':
                return <SeccionInfraestructura />;
            /* [304A-3] Dominios Contabo (admin) */
            case 'dominios':
                return <SeccionDominios />;
            /* [311A-1] Trazabilidad de correos enviados (admin) */
            case 'correos':
                return <SeccionCorreo />;
            /* [026B-1] Cobros pendientes: gestión admin de billing_items */
            case 'cobros':
                return <SeccionCobros />;
            case 'seo':
                return <SeccionSeo />;
            default:
                return <PlaceholderSeccion tab={tabActual} />;
        }
    };

    if (!logueado) return <div className="panelCargando" />;

    return (
        <>
            <SEOHead title="Panel" noindex />
            <HeaderPanel />
            <section id="panelUsuario" className="panelContenedor">
                <div className="panelLayout">
                    <SidebarPanel
                        seccionActiva={seccionActiva}
                        onCambiarSeccion={setSeccionActiva}
                    />

                    <main className="panelContenidoPrincipal">
                        <BannerPassword />
                        <div className="panelContenidoCabecera">
                            <h1 className="panelTitulo">{tabActual.label}</h1>
                        </div>
                        <div className="panelContenido">
                            {renderContenido()}
                        </div>
                    </main>
                </div>
            </section>
        </>
    );
};
