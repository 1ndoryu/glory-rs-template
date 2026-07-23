/* [044A-1] App principal con React Router.
 * Reemplaza el sistema de islands de WordPress por rutas SPA.
 * Cada island se convierte en una ruta. Las páginas de detalle
 * reciben el slug del URL param y buscan datos en data/.
 * [044A-38 Fase 1] Redirige / → /panel si el usuario está logueado.
 * [154A-6] Code splitting con React.lazy para rutas pesadas (PanelIsland, AdminEditorProvider).
 * [155A-1] GoogleAuthCallback procesa el ?code= que Google devuelve tras el redirect. */

import {useLayoutEffect, useEffect, useState, lazy, Suspense} from 'react';
import {BrowserRouter, Routes, Route, useNavigate, useParams} from 'react-router-dom';
import {QueryClient, QueryClientProvider} from '@tanstack/react-query';
import {registrarNavigate} from './navegacionSPA';
import {ScrollToTop} from './components/ui/ScrollToTop';

/* Pages (ex-islands) — solo BienvenidaIsland queda eager (es la ruta de aterrizaje principal).
 * [175A-1] ServiciosIsland, ProyectosIsland y NotFoundIsland → lazy para reducir bundle inicial. */
import {BienvenidaIsland} from './islands/BienvenidaIsland';
const ServiciosIsland = lazy(() => import('./islands/ServiciosIsland').then(m => ({default: m.ServiciosIsland})));
const ProyectosIsland = lazy(() => import('./islands/ProyectosIsland').then(m => ({default: m.ProyectosIsland})));
const NotFoundIsland = lazy(() => import('./islands/NotFoundIsland').then(m => ({default: m.NotFoundIsland})));

/* [204A-2] Lazy: páginas de detalle y rutas secundarias.
 * Solo cargan cuando el usuario navega a ellas, reduciendo el CSS+JS inicial.
 * Las listings (Servicios, Proyectos, Blog) siguen eager porque son rutas de aterrizaje frecuentes. */
const ServicioIndividualIsland = lazy(() => import('./islands/ServicioIndividualIsland').then(m => ({default: m.ServicioIndividualIsland})));
const ProyectoIndividualIsland = lazy(() => import('./islands/ProyectoIndividualIsland').then(m => ({default: m.ProyectoIndividualIsland})));
const NosotrosIsland = lazy(() => import('./islands/NosotrosIsland').then(m => ({default: m.NosotrosIsland})));
const SolucionHostingIsland = lazy(() => import('./islands/SolucionHostingIsland').then(m => ({default: m.SolucionHostingIsland})));
const SolucionHostingWordPressIsland = lazy(() => import('./islands/SolucionHostingIsland').then(m => ({default: m.SolucionHostingWordPressIsland})));
const HostingConfiguradorIsland = lazy(() => import('./islands/HostingConfiguradorIsland').then(m => ({default: m.HostingConfiguradorIsland})));
const SolucionVpsIsland = lazy(() => import('./islands/SolucionVpsIsland').then(m => ({default: m.SolucionVpsIsland})));
const VpsConfiguradorIsland = lazy(() => import('./islands/VpsConfiguradorIsland').then(m => ({default: m.VpsConfiguradorIsland})));

const UsuarioPublicoIsland = lazy(() => import('./islands/UsuarioPublicoIsland').then(m => ({default: m.UsuarioPublicoIsland})));
/* [095A-5] Página legal requerida por el footer */
const PrivacidadIsland = lazy(() => import('./islands/PrivacidadIsland').then(m => ({default: m.PrivacidadIsland})));

/* [054A-5] Toast system */
import {ToastContainer} from './components/ui/ToastContainer';
import {useChatStore} from './stores/chatStore';

/* [154A-6] Lazy load: PanelIsland importa @tiptap (~350KB), Stripe, y componentes admin pesados.
 * AdminEditorProvider importa editores inline que solo usan admins.
 * ChatWidget es moderado pero no es crítico para el first paint. */
const PanelIsland = lazy(() => import('./islands/PanelIsland').then(m => ({default: m.PanelIsland})));
const AdminEditorProvider = lazy(() => import('./components/AdminEditorProvider').then(m => ({default: m.AdminEditorProvider})));
const ChatWidget = lazy(() => import('./components/chat/ChatWidget').then(m => ({default: m.ChatWidget})));
const AuthenticatedNotificationRuntime = lazy(() => import('./components/notifications/AuthenticatedNotificationRuntime').then(m => ({default: m.AuthenticatedNotificationRuntime})));

/* [155A-1] Google OAuth — importaciones para el callback */
import {useAuthStore} from './stores/authStore';
import {apiGoogleLogin} from './api/auth';
import {toast} from './stores/toastStore';

import './App.css';

const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            staleTime: 5 * 60 * 1000,
            retry: 1,
        },
    },
});

/* [175A-2] Pre-poblar cache de React Query con datos inyectados por el servidor.
 * El middleware prerender.rs escribe window.__INITIAL_DATA__.projects en el HTML
 * para que GaleriaHero y SeccionShowcase rendericen sin esperar la API en 4G slow. */
if (typeof window !== 'undefined') {
    const initialData = (window as unknown as {__INITIAL_DATA__?: {projects?: unknown}})['__INITIAL_DATA__'];
    if (initialData?.projects) {
        queryClient.setQueryData(['public-projects-showcase'], initialData.projects);
    }
}

/* Registra navigate de React Router en el módulo navegacionSPA para compatibilidad */
function NavigateRegistrar() {
    const navigate = useNavigate();
    useLayoutEffect(() => {
        /* [065A-4] Registrar navigate antes del primer paint evita que CTAs tempranos
         * caigan al fallback window.location.href y recarguen el documento completo. */
        registrarNavigate((to: string) => navigate(to));
    }, [navigate]);
    return null;
}

/* [155A-1] Procesa el ?code= que Google devuelve tras el redirect OAuth.
 * Debe montarse dentro de BrowserRouter para acceder a useNavigate.
 * Elimina el code del URL antes del exchange para evitar reuso accidental. */
function GoogleAuthCallback() {
    const navigate = useNavigate();
    const authLogin = useAuthStore(s => s.login);

    useEffect(() => {
        const params = new URLSearchParams(window.location.search);
        const code = params.get('code');
        if (!code) return;

        /* Limpiar ?code= de la URL inmediatamente (el code es de un solo uso) */
        const cleanUrl = window.location.pathname;
        window.history.replaceState({}, '', cleanUrl);

        apiGoogleLogin(code)
            .then(resp => {
                authLogin(resp.token, resp.user_id, resp.email, resp.role, resp.effective_role, resp.needs_password);
                navigate('/panel');
            })
            .catch(() => {
                toast.error('Error al iniciar sesión con Google. Inténtalo de nuevo.');
            });
    // Solo en el primer mount — no re-ejecutar si navigate/authLogin cambia referencia
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    return null;
}

/* Wrapper: resuelve slug de servicio a props */
function ServicioDetallePage() {
    const {slug} = useParams<{slug: string}>();
    return <ServicioIndividualIsland slug={slug} />;
}

/* Wrapper: resuelve slug de proyecto a props */
function ProyectoDetallePage() {
    const {slug} = useParams<{slug: string}>();
    return <ProyectoIndividualIsland slug={slug} />;
}

function HomePage() {
    /* [155A-15] Nakomi siempre renderiza su home principal.
     * El portal vps.nakomi.studio pertenece a un despliegue separado de coolify-manager-rs;
     * mezclarlo por hostname hacía que abrir 127.0.0.1 o el dominio VPS cambiara de producto. */
    return <BienvenidaIsland />;
}

function DeferredGlobalWidgets() {
    const [ready, setReady] = useState(false);
    const chatAbierto = useChatStore(s => s.abierto);
    /* [185A-2] Solo cargar AdminEditorProvider (y por ende el chunk editor 119KB) si el
     * usuario es admin. Para visitantes normales el editor nunca se importa, eliminando
     * el editor chunk de la cadena crítica y ahorrando ~120KB en el primer paint. */
    const isAdmin = useAuthStore(s => s.user?.effectiveRole === 'admin');

    useEffect(() => {
        const idleWindow = window as Window & {
            requestIdleCallback?: (callback: () => void, options?: {timeout: number}) => number;
            cancelIdleCallback?: (handle: number) => void;
        };

        if (idleWindow.requestIdleCallback) {
            const idleId = idleWindow.requestIdleCallback(() => setReady(true), {timeout: 2500});
            return () => idleWindow.cancelIdleCallback?.(idleId);
        }

        const timeoutId = window.setTimeout(() => setReady(true), 1500);
        return () => window.clearTimeout(timeoutId);
    }, []);

    if (!ready && !chatAbierto) return null;

    return (
        <>
            {/* [237A-7d] Runtime global de notificaciones WS: una sola conexión para todo el sitio */}
            <Suspense fallback={null}><AuthenticatedNotificationRuntime /></Suspense>
            <Suspense fallback={null}><ChatWidget /></Suspense>
            {isAdmin && <Suspense fallback={null}><AdminEditorProvider /></Suspense>}
        </>
    );
}

/* [074A-1] Home siempre muestra BienvenidaIsland, logueado o no.
 * Los usuarios logueados pueden navegar libremente por el sitio.
 * El panel se accede desde el header (botón "Panel"). */
function App() {
    return (
        <QueryClientProvider client={queryClient}>
            <ToastContainer />
            <BrowserRouter>
                <ScrollToTop />
                <NavigateRegistrar />
                <GoogleAuthCallback />
                <Routes>
                    <Route path="/" element={<HomePage />} />
                    <Route path="/servicios" element={<Suspense fallback={null}><ServiciosIsland /></Suspense>} />
                    <Route path="/servicios/:slug" element={<Suspense fallback={null}><ServicioDetallePage /></Suspense>} />
                    <Route path="/proyectos" element={<Suspense fallback={null}><ProyectosIsland /></Suspense>} />
                    <Route path="/proyectos/:slug" element={<Suspense fallback={null}><ProyectoDetallePage /></Suspense>} />
                    <Route path="/nosotros" element={<Suspense fallback={null}><NosotrosIsland /></Suspense>} />
                    {/* [155A-6] /soluciones ya no es accesible; solo quedan las subpáginas reales. */}
                    <Route path="/soluciones/hosting-wordpress" element={<Suspense fallback={null}><SolucionHostingWordPressIsland /></Suspense>} />
                    <Route path="/soluciones/hosting-wordpress/configurar" element={<Suspense fallback={null}><HostingConfiguradorIsland kind="wordpress" /></Suspense>} />
                    <Route path="/soluciones/hosting-wordpress/configurar/:plan" element={<Suspense fallback={null}><HostingConfiguradorIsland kind="wordpress" /></Suspense>} />
                    <Route path="/soluciones/hosting" element={<Suspense fallback={null}><SolucionHostingIsland /></Suspense>} />
                    <Route path="/soluciones/hosting/configurar" element={<Suspense fallback={null}><HostingConfiguradorIsland kind="normal" /></Suspense>} />
                    <Route path="/soluciones/hosting/configurar/:plan" element={<Suspense fallback={null}><HostingConfiguradorIsland kind="normal" /></Suspense>} />
                    <Route path="/soluciones/vps" element={<Suspense fallback={null}><SolucionVpsIsland /></Suspense>} />
                    <Route path="/soluciones/vps/configurar" element={<Suspense fallback={null}><VpsConfiguradorIsland /></Suspense>} />
                    <Route path="/soluciones/vps/configurar/:tier" element={<Suspense fallback={null}><VpsConfiguradorIsland /></Suspense>} />
                    {/* [064A-5] Ruta /contacto eliminada — todos los CTAs abren el chat */}
                    {/* [095A-5] Política de privacidad accesible desde el footer */}
                    <Route path="/politica-privacidad" element={<Suspense fallback={null}><PrivacidadIsland /></Suspense>} />
                    <Route path="/usuario/:username" element={<Suspense fallback={null}><UsuarioPublicoIsland /></Suspense>} />
                    <Route path="/panel" element={<Suspense fallback={<div className="panelCargando" />}><PanelIsland /></Suspense>} />
                    <Route path="/panel/" element={<Suspense fallback={<div className="panelCargando" />}><PanelIsland /></Suspense>} />
                    <Route path="/panel/chat" element={<Suspense fallback={<div className="panelCargando" />}><PanelIsland /></Suspense>} />
                    {/* [044A-28] Página 404 real en vez de redirigir silenciosamente al home */}
                    <Route path="*" element={<Suspense fallback={null}><NotFoundIsland /></Suspense>} />
                </Routes>
                <DeferredGlobalWidgets />
            </BrowserRouter>
        </QueryClientProvider>
    );
}

export default App;
