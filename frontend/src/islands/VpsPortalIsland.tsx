/* [125A-4] Landing page de vps.nakomi.studio.
 * Inspirado en relace.ai: hero bold, demo terminal dark, features grid,
 * precios live desde useVpsCatalog, FAQ accordion.
 * [125A-5] Login como modal inline usando Modal + useAutenticacion.
 * Standalone: nav y footer propios, sin LayoutPagina.
 * [i18n] Todas las cadenas hardcoded reemplazadas por t() con fallback es.
 * Features y FAQ construidos dinámicamente desde FEATURE_KEYS/FAQ_KEYS. */
/* [259A-5 5b-6] FAQ a <Button>; resto de nativos migrados a DS; disable html retirado por obsoleto. */
import {useState, type ElementType} from 'react';
import {useTranslation} from 'react-i18next';
import {Cpu, Shield, HardDrive, TerminalSquare, Activity, Server, ChevronDown} from 'lucide-react';
import {SEOHead} from '../components/seo/SEOHead';
import {useVpsCatalog} from '../hooks/useVpsCatalog';
import {Button} from '../components/ui/Button';
import {Input} from '../components/ui/Input';
import {Modal, ModalBody} from '../components/ui/Modal';
import {useAutenticacion} from '../hooks/useAutenticacion';
import {useAuthStore} from '../stores/authStore';
import {navegar} from '../navegacionSPA';
/* [259A-5] Scroll a ancla via platform/dom (boundary sentinel). */
import {desplazarHastaElemento} from '../platform/dom';
import './VpsPortalIsland.css';

interface Feature {
    icono: ElementType;
    titulo: string;
    desc: string;
}

/* Claves para construir features dinámicamente con i18n.
 * Cada clave mapea a vps_portal.features.{key}.titulo / .desc */
const FEATURE_KEYS = ['dedicated', 'verified', 'storage', 'root', 'bootstrap', 'scaling'] as const;
const FEATURE_ICONS = [Cpu, Shield, HardDrive, TerminalSquare, Activity, Server];
const FEATURES_FALLBACK: {titulo: string; desc: string}[] = [
    {titulo: 'Recursos dedicados', desc: 'CPU, RAM y NVMe SSD dedicados. Sin compartir nodo con otros clientes.'},
    {titulo: 'Entrega verificada', desc: 'Provisionamos el servidor después del pago y enviamos IP, usuario y acceso inicial.'},
    {titulo: 'NVMe o SSD', desc: 'Storage coherente con el catálogo vigente de Contabo.'},
    {titulo: 'Root + SSH', desc: 'Acceso root completo desde el día uno. Instala lo que necesites.'},
    {titulo: 'Bootstrap inicial', desc: 'Docker, firewall y hostname ya configurados. Despliega de inmediato.'},
    {titulo: 'Escalado claro', desc: 'Cambia de tier cuando lo necesites. Pricing transparente sin sorpresas.'},
];

/* FAQ construido dinámicamente con i18n. Claves q1..q5, a1..a5 */
const FAQ_ITEMS_FALLBACK = [
    {q: '¿Cuándo se activa el VPS?', a: 'Después del pago provisionamos el servidor y enviamos los accesos cuando Contabo lo entregue. El tiempo habitual es menos de 24 h en días laborables.'},
    {q: '¿Qué incluye el bootstrap inicial?', a: 'Docker instalado y activo, firewall ufw con puertos 22, 80 y 443 abiertos, hostname configurado y MOTD con tus recursos de hardware.'},
    {q: '¿Puedo cancelar en cualquier momento?', a: 'Sí. La suscripción se cancela desde tu panel y el servidor se desprovisiona al finalizar el ciclo de facturación actual.'},
    {q: '¿Qué sistema operativo incluye?', a: 'Ubuntu 22.04 LTS por defecto. Si necesitas otra distribución contáctanos antes de completar la compra.'},
    {q: '¿Tienen IPv6?', a: 'IPv4 dedicada en todos los planes. IPv6 disponible bajo consulta para VPS 3 y VPS 4.'},
];
const FAQ_KEYS = ['1', '2', '3', '4', '5'] as const;

const PLANES_FALLBACK = [
    {nombre: 'Cloud VPS 10', precio: '$4.73', desc: 'Entrada dedicada para automatizaciones.', destacado: false, features: ['4 vCPU dedicados', '8 GB RAM', '75 GB NVMe o 150 GB SSD', 'Puerto de 200 Mbit/s']},
    {nombre: 'Cloud VPS 20', precio: '$8.82', desc: 'Balanceado para APIs y SaaS liviano.', destacado: false, features: ['6 vCPU dedicados', '12 GB RAM', '100 GB NVMe o 200 GB SSD', 'Puerto de 1 Gbit/s']},
    {nombre: 'VPS 30', precio: '$17.64', desc: 'Para workloads medianos.', destacado: false, features: ['8 vCPU dedicados', '24 GB RAM', '200 GB NVMe o 400 GB SSD', 'Puerto de 1 Gbit/s']},
    {nombre: 'VPS 40', precio: '$31.50', desc: 'Para cargas con más memoria.', destacado: false, features: ['12 vCPU dedicados', '48 GB RAM', '250 GB NVMe o 500 GB SSD', 'Puerto de 1 Gbit/s']},
];

function formatPrice(cents: number): string {
    return `$${(cents / 100).toFixed(cents % 100 === 0 ? 0 : 2)}`;
}

function scrollTo(id: string): void {
    desplazarHastaElemento(id);
}

export function VpsPortalIsland(): JSX.Element {
    const {t} = useTranslation();
    const {plans} = useVpsCatalog();
    const [faqAbierto, setFaqAbierto] = useState<number | null>(null);
    const [modalLoginAbierto, setModalLoginAbierto] = useState(false);
    const logueado = useAuthStore(s => s.logueado);
    const auth = useAutenticacion(() => {
        setModalLoginAbierto(false);
        navegar('/panel');
    });

    function abrirPanelOLogin(): void {
        if (logueado) {
            navegar('/panel');
        } else {
            setModalLoginAbierto(true);
        }
    }

    const lowestPrice = plans.length
        ? formatPrice(Math.min(...plans.map(p => p.monthly_price_cents)))
        : '$4.73';

    const preciosActivos = plans.length > 0
        ? plans.map(p => ({
            nombre: p.display_name,
            precio: formatPrice(p.monthly_price_cents),
            desc: p.description,
            destacado: false,
            features: p.features,
        }))
        : PLANES_FALLBACK;

    /* Construir features con i18n — fallback español si no hay traducción */
    const features: Feature[] = FEATURE_KEYS.map((key, i) => ({
        icono: FEATURE_ICONS[i],
        titulo: t(`vps_portal.features.${key}.titulo`, FEATURES_FALLBACK[i].titulo),
        desc: t(`vps_portal.features.${key}.desc`, FEATURES_FALLBACK[i].desc),
    }));

    /* Construir FAQ con i18n */
    const faqItems = FAQ_KEYS.map(key => ({
        q: t(`vps_portal.faq.q${key}`, FAQ_ITEMS_FALLBACK[Number(key) - 1].q),
        a: t(`vps_portal.faq.a${key}`, FAQ_ITEMS_FALLBACK[Number(key) - 1].a),
    }));

    return (
        <div className="vpsPortal">
            <SEOHead
                title={t('vps_portal.seo_title', 'Nakomi VPS — Infraestructura dedicada sin intermediarios')}
                description={t('vps_portal.seo_desc', 'Servidores VPS con recursos dedicados, root SSH, Docker listo, velocidad y tráfico visibles. Planes desde {{price}}/mes.').replace('{{price}}', lowestPrice)}
                path="/"
            />
            <nav className="vpsNavPortal">
                <div className="vpsNavContenido">
                    <a className="vpsNavLogo" href="https://nakomi.studio">
                        <span className="vpsNavLogoMarca">Nakomi</span>
                        <span className="vpsNavLogoProducto">VPS</span>
                    </a>
                    <div className="vpsNavLinks">
                        <Button variante="texto" className="vpsNavLink" type="button" onClick={() => scrollTo('caracteristicas')}>{t('vps_portal.nav.features', 'Características')}</Button>
                        <Button variante="texto" className="vpsNavLink" type="button" onClick={() => scrollTo('precios')}>{t('vps_portal.nav.pricing', 'Precios')}</Button>
                        <Button variante="texto" className="vpsNavLink" type="button" onClick={() => scrollTo('faq')}>{t('vps_portal.nav.faq', 'FAQ')}</Button>
                    </div>
                    <div className="vpsNavAcciones">
                        <Button variante="outline" onClick={abrirPanelOLogin}>
                            {logueado ? t('vps_portal.nav.my_panel', 'Mi panel') : t('vps_portal.nav.login', 'Iniciar sesión')}
                        </Button>
                        <Button variante="primario" onClick={() => scrollTo('precios')}>{t('vps_portal.nav.view_plans', 'Ver planes')}</Button>
                    </div>
                </div>
            </nav>
            <section className="vpsHero">
                <span className="vpsHeroEtiqueta">{t('vps_portal.hero.badge', 'Nakomi VPS')}</span>
                <h1 className="vpsHeroTitulo">
                    {t('vps_portal.hero.title', 'Infraestructura dedicada')}<br />
                    <span className="vpsHeroTituloAcento">{t('vps_portal.hero.title_accent', 'sin intermediarios.')}</span>
                </h1>
                <p className="vpsHeroSub">
                    {t('vps_portal.hero.subtitle', 'VPS con recursos propios, bootstrap inicial, tráfico visible y margen operativo claro.')}
                </p>
                <div className="vpsHeroBotones">
                    <Button variante="primario" onClick={() => scrollTo('precios')}>{t('vps_portal.hero.cta_primary', 'Ver planes y precios')}</Button>
                    <Button variante="outline" onClick={() => scrollTo('caracteristicas')}>{t('vps_portal.hero.cta_secondary', 'Características')}</Button>
                </div>
                <div className="vpsHeroStats">
                    <div className="vpsHeroStat">
                        <span className="vpsHeroStatVal">100%</span>
                        <span className="vpsHeroStatLabel">{t('vps_portal.hero.stat_dedicated', 'Recursos dedicados')}</span>
                    </div>
                    <div className="vpsHeroStat">
                        <span className="vpsHeroStatVal">&lt;24h</span>
                        <span className="vpsHeroStatLabel">{t('vps_portal.hero.stat_provision', 'Alta revisada')}</span>
                    </div>
                    <div className="vpsHeroStat">
                        <span className="vpsHeroStatVal">desde {lowestPrice}</span>
                        <span className="vpsHeroStatLabel">{t('vps_portal.hero.stat_per_month', 'por mes')}</span>
                    </div>
                </div>
            </section>
            <section className="vpsDemoTerminal">
                <div className="vpsDemoContenido">
                    <p className="vpsDemoEtiqueta">{t('vps_portal.demo.badge', 'Bootstrap inicial')}</p>
                    <h2 className="vpsDemoTitulo">{t('vps_portal.demo.title', 'Tu servidor, listo para desplegar.')}</h2>
                    <div className="vpsDemoBloque" aria-label="Ejemplo de sesión SSH tras provisioning">
                        <div className="vpsDemoCabecera">
                            <span className="vpsDemoPunto vpsDemoPuntoRojo" />
                            <span className="vpsDemoPunto vpsDemoPuntoAmbar" />
                            <span className="vpsDemoPunto vpsDemoPuntoVerde" />
                            <span className="vpsDemoCabeceraLabel">ssh root@203.0.113.42</span>
                        </div>
                        <pre className="vpsDemoCodigo">{`$ ssh root@203.0.113.42
Ubuntu 22.04.3 LTS — Nakomi VPS 2
Docker   : ● active
Firewall : ● active (22, 80, 443 open)
CPU      : 2 vCPU dedicated
RAM      : 4 GB
Disk     : 60 GB NVMe SSD
root@nakomi-vps:~$ docker ps
CONTAINER ID   IMAGE   STATUS
(servidor limpio — listo para desplegar)
root@nakomi-vps:~$ █`}</pre>
                    </div>
                </div>
            </section>
            <section className="vpsFeatures" id="caracteristicas">
                <div className="vpsSectionContenido">
                    <h2 className="vpsSectionTitulo">{t('vps_portal.features.title', 'Todo lo que necesitas para operar')}</h2>
                    <p className="vpsSectionSub">{t('vps_portal.features.subtitle', 'Sin servicios gestionados innecesarios, sin overhead de plataforma.')}</p>
                    <div className="vpsFeaturesGrid">
                        {features.map((f) => {
                            const Icono = f.icono;
                            return (
                                <div key={f.titulo} className="vpsFeatureCard">
                                    <span className="vpsFeatureIcono"><Icono size={18} /></span>
                                    <strong className="vpsFeatureTitulo">{f.titulo}</strong>
                                    <p className="vpsFeatureDesc">{f.desc}</p>
                                </div>
                            );
                        })}
                    </div>
                </div>
            </section>
            <section className="vpsPrecios" id="precios">
                <div className="vpsSectionContenido">
                    <h2 className="vpsSectionTitulo">{t('vps_portal.pricing.title', 'Planes claros, recursos reales')}</h2>
                    <p className="vpsSectionSub">{t('vps_portal.pricing.subtitle', 'Suscripción mensual. Sin contratos anuales. Cancela cuando quieras.')}</p>
                    <div className="vpsPreciosGrid">
                        {preciosActivos.map((plan) => (
                            <div key={plan.nombre} className={`vpsPrecioCard${plan.destacado ? ' vpsPrecioCardDestacado' : ''}`}>
                                <h3 className="vpsPrecioNombre">{plan.nombre}</h3>
                                <div className="vpsPrecioValor">
                                    <span className="vpsPrecioCantidad">{plan.precio}</span>
                                    <span className="vpsPrecioPeriodo">{t('vps_portal.pricing.per_month', '/mes')}</span>
                                </div>
                                <p className="vpsPrecioDesc">{plan.desc}</p>
                                <ul className="vpsPrecioFeatures">
                                    {plan.features.map((f) => <li key={f}>{f}</li>)}
                                </ul>
                                <Button variante="outline" onClick={() => navegar(`/soluciones/vps/configurar/${plans.find(p => p.display_name === plan.nombre)?.tier_name ?? ''}`)}>
                                    {t('vps_portal.pricing.configure', 'Configurar')} {plan.nombre}
                                </Button>
                            </div>
                        ))}
                    </div>
                </div>
            </section>
            <section className="vpsFaq" id="faq">
                <div className="vpsSectionContenido vpsSectionAngosto">
                    <h2 className="vpsSectionTitulo">{t('vps_portal.faq.title', 'Preguntas frecuentes')}</h2>
                    <div className="vpsFaqLista">
                        {faqItems.map((item, i) => (
                            <div key={item.q} className="vpsFaqItem">
                                <Button
                                    type="button"
                                    variante="texto"
                                    className="vpsFaqPregunta"
                                    onClick={() => setFaqAbierto(faqAbierto === i ? null : i)}
                                    aria-expanded={faqAbierto === i}
                                >
                                    <span>{item.q}</span>
                                    <ChevronDown className={`vpsFaqChevron${faqAbierto === i ? ' vpsFaqChevronAbierto' : ''}`} size={16} />
                                </Button>
                                {faqAbierto === i && <p className="vpsFaqRespuesta">{item.a}</p>}
                            </div>
                        ))}
                    </div>
                </div>
            </section>
            <section className="vpsCtaFinal">
                <h2 className="vpsCtaFinalTitulo">{t('vps_portal.cta.title', 'Infraestructura lista en minutos.')}</h2>
                <p className="vpsCtaFinalSub">{t('vps_portal.cta.subtitle', 'Configura tu VPS hoy y completa el checkout con los recursos visibles antes de pagar.')}</p>
                <Button variante="primario" onClick={() => scrollTo('precios')}>{t('vps_portal.cta.button', 'Ver planes')}</Button>
            </section>
            <footer className="vpsFooterPortal">
                <div className="vpsFooterContenido">
                    <span className="vpsFooterMarca">{t('vps_portal.footer.brand', 'Nakomi VPS')}</span>
                    <div className="vpsFooterLinks">
                        <a href="https://nakomi.studio" className="vpsFooterLink">{t('vps_portal.footer.studio', 'Nakomi Studio')}</a>
                        <a href="/politica-privacidad" className="vpsFooterLink">{t('vps_portal.footer.privacy', 'Privacidad')}</a>
                    </div>
                    <span className="vpsFooterCopy">© {new Date().getFullYear()} Nakomi Studio</span>
                </div>
            </footer>
            <Modal abierto={modalLoginAbierto} onCerrar={() => setModalLoginAbierto(false)}>
                <ModalBody as="form" onSubmit={auth.handleLogin}>
                    <p className="modalTexto">{t('vps_portal.login.message', 'Accede al panel para ver tus solicitudes y servidores.')}</p>
                    {auth.error && <p className="vpsLoginError">{auth.error}</p>}
                    <div className="modalCampo">
                        <label className="modalEtiqueta" htmlFor="vps-email">{t('vps_portal.login.email', 'Correo')}</label>
                        <Input
                            id="vps-email"
                            type="email"
                            autoComplete="email"
                            placeholder="tu@email.com"
                            value={auth.login.email}
                            onChange={(e) => auth.actualizarLogin('email', e.target.value)}
                            required
                        />
                    </div>
                    <div className="modalCampo">
                        <label className="modalEtiqueta" htmlFor="vps-password">{t('vps_portal.login.password', 'Contraseña')}</label>
                        <Input
                            id="vps-password"
                            type="password"
                            autoComplete="current-password"
                            placeholder="••••••••"
                            value={auth.login.password}
                            onChange={(e) => auth.actualizarLogin('password', e.target.value)}
                            required
                        />
                    </div>
                    <div className="modalAcciones">
                        <Button type="button" variante="outline" onClick={() => setModalLoginAbierto(false)}>
                            {t('vps_portal.login.cancel', 'Cancelar')}
                        </Button>
                        <Button type="submit" variante="primario" disabled={auth.cargando}>
                            {auth.cargando ? t('vps_portal.login.entering', 'Entrando...') : t('vps_portal.login.enter', 'Entrar')}
                        </Button>
                    </div>
                </ModalBody>
            </Modal>
        </div>
    );
}

export default VpsPortalIsland;
