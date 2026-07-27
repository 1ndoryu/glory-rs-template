import {useTranslation} from 'react-i18next';
import {Cpu, Shield, HardDrive, TerminalSquare, Activity, Server} from 'lucide-react';
import {LayoutPagina} from '../components/layout/LayoutPagina';
import {SEOHead} from '../components/seo/SEOHead';
import {faqSchema} from '../components/seo/schemas';
import {SeccionContacto} from '../components/home/SeccionContacto';
import {useChatStore} from '../stores/chatStore';
import {Button} from '../components/ui/Button';
import {Tarjeta} from '../components/ui/Tarjeta';
import {SolucionHeroImagen} from '../components/soluciones/SolucionHeroImagen';
import {PlanFeatureTooltip} from '../components/servicios/PlanFeatureTooltip';
import {useVpsCatalog} from '../hooks/useVpsCatalog';
import {navegar} from '../navegacionSPA';
import '../components/servicios/SeccionPlanesServicio.css';
import './SolucionHostingIsland.css';

const FEATURES_FALLBACK = [
    {icono: Cpu, titulo: 'Recursos dedicados', desc: 'Cada VPS entrega CPU, RAM y SSD dedicados sin compartir el nodo con otros clientes.'},
    {icono: Shield, titulo: 'Entrega verificada', desc: 'Después del pago provisionamos el servidor y enviamos IP, usuario y acceso inicial.'},
    {icono: HardDrive, titulo: 'NVMe o SSD', desc: 'Opciones de almacenamiento coherentes con el catálogo vigente de Contabo.'},
    {icono: TerminalSquare, titulo: 'Root + SSH', desc: 'Acceso completo al servidor para administrar procesos, paquetes y despliegues.'},
    {icono: Activity, titulo: 'Bootstrap inicial', desc: 'Entregamos hostname, MOTD, Docker y firewall básico ya configurados.'},
    {icono: Server, titulo: 'Escalado claro', desc: 'Pasas de un tier a otro cuando tus necesidades cambien, con pricing transparente y sin sorpresas.'},
];

function formatMonthlyPrice(priceCents: number): string {
    return `$${(priceCents / 100).toFixed(priceCents % 100 === 0 ? 0 : 2)}`;
}

export const SolucionVpsIsland = (): JSX.Element => {
    const {t} = useTranslation();
    const abrirChat = useChatStore(s => s.abrir);
    const {plans} = useVpsCatalog();

    const lowestPrice = Math.min(...plans.map(plan => plan.monthly_price_cents));
    const lowestPriceLabel = Number.isFinite(lowestPrice)
        ? formatMonthlyPrice(lowestPrice)
        : '$4.73';

    return (
        <LayoutPagina className="hostingPaginaMain">
            <SEOHead
                title="Servidores VPS"
                description={`Servidores VPS dedicados con acceso root, bootstrap inicial, velocidad y tráfico visibles. Planes desde ${lowestPriceLabel}/mes.`}
                path="/soluciones/vps"
                jsonLd={faqSchema([
                    {question: '¿Qué es un VPS de Nakomi Studio?', answer: 'Es un servidor virtual dedicado con acceso root, Docker preinstalado, firewall básico y bootstrap inicial. Lo provisionamos y te enviamos IP y credenciales listas para usar.'},
                    {question: '¿Puedo escalar mi VPS después de contratarlo?', answer: 'Sí, puedes cambiar de tier en cualquier momento. El pricing es transparente y visible antes de pagar, sin sorpresas ni cargos ocultos.'},
                    {question: '¿El VPS incluye backups?', answer: 'El VPS se entrega con acceso root completo. Los backups los gestionas tú o podemos añadir un plan de backups administrados como servicio adicional.'},
                    {question: '¿Qué sistema operativo puedo instalar?', answer: 'Ofrecemos las distribuciones Linux más populares: Ubuntu, Debian y CentOS. El servidor se entrega con Docker y herramientas básicas ya configuradas.'},
                ])}
            />

            <section className="hostingHero">
                <div className="hostingHeroContenido">
                    <span className="hostingHeroEtiqueta">{t('content.solutions.vps.titulo', 'Servidores VPS')}</span>
                    <h1 className="hostingHeroTitulo">Infraestructura dedicada con precios claros</h1>
                    <p className="hostingHeroDesc">
                        Compra un VPS dedicado con acceso root, Docker listo, firewall básico, velocidad de puerto y tráfico visibles antes de pagar.
                    </p>
                    <div className="hostingHeroBotones">
                        <Button variante="primario" onClick={() => {
                            document.getElementById('planesVps')?.scrollIntoView({behavior: 'smooth'});
                        }}>
                            Ver planes
                        </Button>
                        <Button variante="outline" onClick={() => abrirChat('page:vps')}>
                            Conversar
                        </Button>
                    </div>
                </div>
            </section>

            <SolucionHeroImagen
                src="/assets/Proyectos portadas/Kamples portada.jpg"
                alt="Infraestructura VPS dedicada para despliegues de aplicaciones."
                storageKey="nakomi-vps-hero-image"
            />

            <section className="hostingFeatures">
                <h2 className="hostingFeaturesTitle">Qué incluye la entrega</h2>
                <p className="hostingFeaturesSubtitle">
                    La provisión se entrega lista para operar, pero manteniendo el control completo del servidor en tus manos.
                </p>
                <div className="hostingFeaturesGrid">
                    {FEATURES_FALLBACK.map(feature => (
                        <Tarjeta key={feature.titulo} className="hostingFeatureCard" fondo="#f5f3f1">
                            <div className="hostingFeatureIcono">
                                <feature.icono size={20} strokeWidth={1.5} />
                            </div>
                            <h3>{feature.titulo}</h3>
                            <p>{feature.desc}</p>
                        </Tarjeta>
                    ))}
                </div>
            </section>

            <section id="planesVps" className="planesSeccion">
                <div className="planesContenedor">
                    <div className="planesCabecera">
                        <h2 className="planesTitulo">Planes VPS dedicados</h2>
                        <p className="planesSubtitulo">Checkout mensual con recursos, storage, tráfico y cargo inicial visibles antes de Stripe.</p>
                    </div>
                    <div className="planesGrid">
                        {plans.map(plan => (
                            <div key={plan.tier_name} className="tarjetaPlan">
                                <div className="tarjetaPlanCabecera">
                                    <h3 className="tarjetaPlanNombre">{plan.display_name}</h3>
                                    <div className="tarjetaPlanPrecio">
                                        <span className="tarjetaPlanPrecioCifra">{formatMonthlyPrice(plan.monthly_price_cents)}</span>
                                        <span className="tarjetaPlanPrecioPeriodo">/mes</span>
                                    </div>
                                    {plan.setup_fee_cents > 0 && (
                                        <p className="tarjetaPlanDescripcion">Puesta en marcha: {formatMonthlyPrice(plan.setup_fee_cents)}</p>
                                    )}
                                    <p className="tarjetaPlanDescripcion">{plan.description}</p>
                                </div>
                                <ul className="tarjetaPlanCaracteristicas">
                                    {plan.features.map(feature => (
                                        <li key={feature} className="tarjetaPlanItem tarjetaPlanItemIncluido">
                                            <span className="tarjetaPlanItemIcono">
                                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                                                    <polyline points="20 6 9 17 4 12" />
                                                </svg>
                                            </span>
                                            <PlanFeatureTooltip feature={feature} context="vps" />
                                        </li>
                                    ))}
                                </ul>
                                <div className="tarjetaPlanAccion">
                                    <Button
                                        variante="outline"
                                        tamano="mediano"
                                        onClick={() => navegar(`/soluciones/vps/configurar/${plan.tier_name}`)}
                                    >
                                        Configurar VPS
                                    </Button>
                                    <Button variante="texto" className="tarjetaPlanConversar" onClick={() => abrirChat('page:vps')}>
                                        Conversar
                                    </Button>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </section>

            <SeccionContacto />
        </LayoutPagina>
    );
};