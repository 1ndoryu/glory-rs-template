/**
 * Componente: ServicioIndividualIsland
 * Página de detalle de un servicio individual.
 * Estructura: Hero -> Galeria -> CTA -> Skills -> Relacionados -> Contacto -> Footer
 * Skills y datos centralizados en data/ (DRY).
 * [044A-38 Fase 1] CTA "Contratar" apunta a /panel si logueado (Fase 2 conecta con API de órdenes).
 * [084A-4] Restaurado ModalCompra — click en plan abre modal de pago (revertido 074A-36).
 * sentinel-disable-file componente-sin-hook: Lógica minimal (fetch API + wiring de estado)
 * no justifica hook separado; datos estáticos vienen de imports directos.
 * [094A-24] La compra solo puede salir de `apiData` real para no ofrecer servicios
 * que ya no existen en el backend y terminan en 404 al crear la orden.
 */
import {useState, useCallback, useEffect} from 'react';
import {useTranslation} from 'react-i18next';
import {useAuthStore} from '../stores/authStore';
import {useChatStore} from '../stores/chatStore';
import '../styles/variables.css';
import './ServicioIndividualIsland.css';
import {LayoutPagina} from '../components/layout/LayoutPagina';
import {SEOHead} from '../components/seo/SEOHead';
import {serviceSchema, breadcrumbSchema} from '../components/seo/schemas';
import {SeccionHeroServicio} from '../components/servicios/SeccionHeroServicio';
import {SeccionGaleriaServicio} from '../components/servicios/SeccionGaleriaServicio';
import {SeccionSkillsServicio} from '../components/servicios/SeccionSkillsServicio';
import {SeccionServiciosRelacionados} from '../components/servicios/SeccionServiciosRelacionados';
import {SeccionCta} from '../components/ui/SeccionCta';
import {SeccionPlanesServicio} from '../components/servicios/SeccionPlanesServicio';
import {ModalCompra} from '../components/servicios/ModalCompra';
import {SeccionContacto} from '../components/home/SeccionContacto';
import {NotFoundIsland} from './NotFoundIsland';
import {SKILLS_POR_DEFECTO} from '../data/skills';
import type {PlanServicio} from '../data/planes/tipos';
import {apiGetServiceBySlug, type PublicService, type PublicServicePlan} from '../api/admin-services';

/* [064A-47] Imagen del servicio añadida al hero. */
interface ServicioIndividualIslandProps {
    titulo?: string;
    descripcion?: string;
    precio_desde?: string;
    slug?: string;
    imagen?: string;
}

export const ServicioIndividualIsland = ({titulo, descripcion, precio_desde, slug, imagen}: ServicioIndividualIslandProps): JSX.Element => {
    const {t} = useTranslation();
    const logueado = useAuthStore(s => s.logueado);
    const abrirChat = useChatStore(s => s.abrir);

    /* [084A-4] Estado del modal de compra */
    const [planSeleccionado, setPlanSeleccionado] = useState<PlanServicio | null>(null);
    const handleSeleccionarPlan = useCallback((plan: PlanServicio) => setPlanSeleccionado(plan), []);
    const handleCerrarModal = useCallback(() => setPlanSeleccionado(null), []);

    /* [064A-5] Si logueado, CTA lleva al panel. Si no, abre el chat. */
    const ctaLink = logueado ? '/panel' : undefined;

    /* [074A-21] Fetch servicio de la API pública. */
    const [apiData, setApiData] = useState<PublicService | null>(null);
    const [apiError, setApiError] = useState(false);

    useEffect(() => {
        const controller = new AbortController();
        setApiError(false);

        if (slug) {
            apiGetServiceBySlug(slug)
                .then(data => { if (!controller.signal.aborted) setApiData(data); })
                .catch(() => {
                    if (!controller.signal.aborted) {
                        setApiData(null);
                        setApiError(true);
                    }
                });
        }

        return () => controller.abort();
    }, [slug]);

    if (slug && apiError) {
        return <NotFoundIsland />;
    }

    if (slug && !apiData) {
        return (
            <LayoutPagina className="servicioIndividualMain">
                <p>{t('common.loading', 'Cargando...')}</p>
            </LayoutPagina>
        );
    }

    const servicioId = apiData?.slug || slug || titulo?.toLowerCase().replace(/\s+/g, '-') || '';

    const tituloFinal = apiData?.title || titulo;
    const descripcionFinal = apiData?.description || descripcion;
    const imagenFinal = apiData?.image_url || imagen;
    const skillsApi = apiData?.skills ? (apiData.skills as Array<{titulo: string; descripcion: string}>).map((sk, i) => ({id: i, titulo: sk.titulo, descripcion: sk.descripcion})) : null;
    const planesApi = apiData?.plans?.map(plan => convertirPlanPublico(apiData.slug, plan)) || null;

    /* [044A-40] Obtener precio mínimo real de los planes del servicio. */
    const precioMinimo = (() => {
        if (precio_desde) return precio_desde;
        if (!planesApi || planesApi.length === 0) return '';
        const precios = planesApi
            .map(p => {
                if (p.esPersonalizado) return Infinity;
                const num = parseFloat(p.precio.replace(/[^0-9.]/g, ''));
                return isNaN(num) ? Infinity : num;
            })
            .filter(n => n !== Infinity);
        if (precios.length === 0) return '';
        return `$${Math.min(...precios)}`;
    })();

    return (
        <LayoutPagina className="servicioIndividualMain">
            <SEOHead
                title={tituloFinal}
                description={descripcionFinal}
                path={`/servicios/${servicioId}`}
                jsonLd={tituloFinal && descripcionFinal && servicioId ? {
                    '@context': 'https://schema.org',
                    '@graph': [
                        breadcrumbSchema([
                            {name: 'Inicio', url: '/'},
                            {name: 'Servicios', url: '/servicios'},
                            {name: tituloFinal, url: `/servicios/${servicioId}`},
                        ]),
                        serviceSchema(tituloFinal, descripcionFinal, servicioId),
                    ],
                } : undefined}
            />
            <SeccionHeroServicio titulo={tituloFinal} descripcion={descripcionFinal} imagen={imagenFinal} />
            <SeccionGaleriaServicio />
            <SeccionPlanesServicio slug={servicioId} planes={planesApi} onSeleccionarPlan={handleSeleccionarPlan} />
            {/* [084A-28] Pasar contexto de servicio al abrir chat para soporte contextual */}
            <SeccionCta descripcion={[t('service_detail.cta_1'), t('service_detail.cta_2')]} textoBotonPrimario={precioMinimo ? `${t('service_detail.cta_hire')} ${precioMinimo}` : t('service_detail.cta_hire')} linkBotonPrimario={ctaLink} onBotonPrimarioClick={logueado ? undefined : () => abrirChat(`service:${slug || servicioId}`)} textoBotonSecundario={t('service_detail.cta_contact')} onBotonSecundarioClick={() => abrirChat(`service:${slug || servicioId}`)} />
            <SeccionSkillsServicio skills={skillsApi || SKILLS_POR_DEFECTO} />
            <SeccionServiciosRelacionados servicioActualId={servicioId} />
            <SeccionContacto />
            {planSeleccionado && (
                <ModalCompra
                    plan={planSeleccionado}
                    servicioSlug={servicioId}
                    abierto={true}
                    onCerrar={handleCerrarModal}
                />
            )}
        </LayoutPagina>
    );
};

export default ServicioIndividualIsland;

function convertirPlanPublico(serviceSlug: string, plan: PublicServicePlan): PlanServicio {
    const features = Array.isArray(plan.features)
        ? plan.features
            .map((feature: unknown) => {
                const data = feature as Record<string, unknown>;
                const texto = typeof data.texto === 'string' ? data.texto : '';
                const incluido = typeof data.incluido === 'boolean' ? data.incluido : false;
                return texto ? {texto, incluido} : null;
            })
            .filter((feature): feature is {texto: string; incluido: boolean} => feature !== null)
        : [];

    const priceValue = plan.is_custom || plan.price_cents <= 0
        ? 'A medida'
        : formatPrice(plan.price_cents);

    return {
        id: plan.slug,
        nombre: plan.name,
        precio: priceValue,
        periodo: serviceSlug === 'agentes-ia' && !plan.is_custom ? '/mes' : undefined,
        descripcion: plan.description || '',
        caracteristicas: features,
        destacado: plan.is_highlighted,
        esPersonalizado: plan.is_custom,
        ctaTexto: plan.is_custom ? 'Hablar con nosotros' : 'Elegir plan',
        ctaLink: '/contacto/',
    };
}

function formatPrice(priceCents: number): string {
    const price = priceCents / 100;
    return Number.isInteger(price) ? `$${price}` : `$${price.toFixed(2)}`;
}
