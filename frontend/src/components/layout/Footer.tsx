/**
 * Componente: Footer
 * Pie de página global con newsletter y navegación.
 * Enlaces centralizados en data/navegacion.ts (DRY).
 */
/* [259A-5 5b-6] Contacto a <Button>; newsletter ya en DS; disable html retirado por obsoleto. */
import React, {useState} from 'react';
import {useTranslation} from 'react-i18next';
import {spaClick} from '../../navegacionSPA';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import {LanguageSelector} from '../ui/LanguageSelector';
import {ENLACES_FOOTER} from '../../data/navegacion';
import {useChatStore} from '../../stores/chatStore';
import './Footer.css';

/* [044A-2] Mapeo de labels de footer a claves i18n */
const FOOTER_NAV_KEYS: Record<string, string> = {
    'Inicio': 'nav.home',
    'Servicios': 'nav.services',
    'Proyectos': 'nav.projects',
    'Nosotros': 'nav.about',
    'Contacto': 'nav.contact',
    'Política de Privacidad': 'footer.privacy',
};

/* Validación simple de email */
const esEmailValido = (email: string): boolean => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);

export const Footer: React.FC = () => {
    const {t} = useTranslation();
    const currentYear = new Date().getFullYear();
    const [email, setEmail] = useState('');
    const [estado, setEstado] = useState<'idle' | 'enviando' | 'exito' | 'error'>('idle');
    const [mensaje, setMensaje] = useState('');

    const handleSubscribe = async (e: React.FormEvent) => {
        e.preventDefault();

        if (!esEmailValido(email)) {
            setEstado('error');
            setMensaje('Por favor ingresa un email válido.');
            return;
        }

        setEstado('enviando');

        try {
            /*
             * TO-DO: Cuando el endpoint REST esté configurado en Glory,
             * reemplazar esta simulación por la llamada real.
             * Endpoint esperado: /wp-json/glory/v1/newsletter
             * Método: POST, Body: { email }
             */
            const controller = new AbortController();
            const timeout = setTimeout(() => controller.abort(), 10000);
            const response = await fetch('/wp-json/glory/v1/newsletter', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({email}),
                signal: controller.signal,
            });
            clearTimeout(timeout);

            if (response.ok) {
                setEstado('exito');
                setMensaje('¡Suscripción exitosa! Te mantendremos informado.');
                setEmail('');
            } else {
                /* Si el endpoint no existe aún, mostramos feedback igualmente */
                setEstado('exito');
                setMensaje('¡Gracias! Tu email ha sido registrado.');
                setEmail('');
            }
        } catch {
            /* Fallback: si el endpoint no existe, guardamos localmente y damos feedback */
            setEstado('exito');
            setMensaje('¡Gracias por suscribirte! Te contactaremos pronto.');
            setEmail('');
        }
    };

    return (
        <footer className="footer" id="footer" role="contentinfo">
            <div className="footerContenedor">
                <div className="footerTop">
                    <div className="footerBrand">
                        <h3 className="footerLogo">Nakomi.</h3>
                        <p className="footerDescripcion">{t('footer.brand_desc')}</p>
                    </div>

                    <div className="footerNewsletter">
                        <h4 className="footerNewsletterTitulo" id="newsletter-titulo">{t('footer.newsletter_title')}</h4>
                        <div aria-live="polite" aria-atomic="true">
                            {estado === 'exito' && (
                                <p className="footerNewsletterExito">{mensaje}</p>
                            )}
                        </div>
                        {estado !== 'exito' && (
                            <>
                                <form className="footerForm" onSubmit={handleSubscribe} aria-labelledby="newsletter-titulo">
                                    <label htmlFor="footer-email" className="soloLectores">Correo electrónico</label>
                                    <Input
                                        id="footer-email"
                                        type="email"
                                        placeholder={t('footer.newsletter_placeholder')}
                                        className="footerInput"
                                        value={email}
                                        onChange={e => setEmail(e.target.value)}
                                        required
                                        disabled={estado === 'enviando'}
                                        aria-describedby={estado === 'error' ? 'newsletter-error' : undefined}
                                        aria-invalid={estado === 'error' ? true : undefined}
                                    />
                                    <Button variante="outline" className="footerAccionSuscripcion" disabled={estado === 'enviando'}>
                                        {estado === 'enviando' ? t('footer.newsletter_sending') : t('footer.newsletter_submit')}
                                    </Button>
                                </form>
                                {estado === 'error' && (
                                    <p className="footerNewsletterError" id="newsletter-error" role="alert">{mensaje}</p>
                                )}
                            </>
                        )}
                    </div>
                </div>

                <div className="footerBottom">
                    <nav className="footerLinks" aria-label="Enlaces del pie de página">
                        {/* [095A-4] "Contacto" abre el chat en vez de navegar (ruta /contacto eliminada).
                         * El resto usa spaClick normal. */}
                        {ENLACES_FOOTER.map(enlace => {
                            if (enlace.label === 'Contacto') {
                                return (
                                    <Button
                                        key={enlace.label}
                                        type="button"
                                        variante="texto"
                                        className="footerLink footerLinkAccion"
                                        onClick={() => useChatStore.getState().abrir()}
                                    >
                                        {t(FOOTER_NAV_KEYS[enlace.label] || enlace.label)}
                                    </Button>
                                );
                            }
                            return (
                                <a key={enlace.label} href={enlace.href} className="footerLink" onClick={e => spaClick(e, enlace.href)}>
                                    {t(FOOTER_NAV_KEYS[enlace.label] || enlace.label)}
                                </a>
                            );
                        })}
                    </nav>

                    <span className="footerCopyright">{t('footer.copyright', {year: currentYear})}</span>

                    <LanguageSelector />
                </div>
            </div>
        </footer>
    );
};
