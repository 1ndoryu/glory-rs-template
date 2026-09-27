/**
 * Componente: ContactoIsland
 * Pagina de contacto completa con formulario.
 * Campos: nombre, email, telefono, descripcion, presupuesto.
 */
/* [259A-5 5b-6] Nativos migrados a DS; disable html retirado por obsoleto. */
import React, {useState} from 'react';
import {useTranslation} from 'react-i18next';
import '../styles/variables.css';
import './ContactoIsland.css';
import {LayoutPagina} from '../components/layout/LayoutPagina';
import {SEOHead} from '../components/seo/SEOHead';
import {Button} from '../components/ui/Button';
import {Input} from '../components/ui/Input';
import {SelectDropdown} from '../components/ui/SelectDropdown';
import {Textarea} from '../components/ui/Textarea';
import {INFO_CONTACTO} from '../data/contacto';

interface ContactoIslandProps {
    titulo?: string;
}

interface FormularioContacto {
    nombre: string;
    email: string;
    telefono: string;
    descripcion: string;
    presupuesto: string;
}

const PRESUPUESTOS_KEYS = [
    {value: '', key: 'contact.budget_select'},
    {value: 'menos-5k', key: 'contact.budget_1'},
    {value: '5k-10k', key: 'contact.budget_2'},
    {value: '10k-25k', key: 'contact.budget_3'},
    {value: '25k-50k', key: 'contact.budget_4'},
    {value: 'mas-50k', key: 'contact.budget_5'},
];

export const ContactoIsland = ({titulo}: ContactoIslandProps): JSX.Element => {
    const {t} = useTranslation();
    const [formulario, setFormulario] = useState<FormularioContacto>({
        nombre: '',
        email: '',
        telefono: '',
        descripcion: '',
        presupuesto: '',
    });
    const [enviado, setEnviado] = useState(false);

    const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
        const {name, value} = e.target;
        setFormulario(prev => ({...prev, [name]: value}));
    };

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        /* TO-DO: Integrar con backend (REST API o email) */
        setEnviado(true);
    };

    return (
        <LayoutPagina className="contactoMain" id="paginaContacto">
            <SEOHead
                title="Contacto"
                description="Contáctanos para tu próximo proyecto de desarrollo web o diseño digital."
                path="/contacto"
            />
            {/* Hero */}
            <section className="contactoHero">
                <div className="contactoHeroContenido">
                    <div className="contactoHeroTexto">
                        <h1 className="contactoHeroTitulo">{titulo || t('contact.title')}</h1>
                    </div>
                    <div className="contactoHeroDescripcion">
                        <p>{t('contact.hero_description')}</p>
                    </div>
                </div>
            </section>

            {/* Formulario */}
            <section className="contactoFormularioSeccion">
                <div className="contactoFormularioContenedor">
                    {enviado ? (
                        <div className="contactoExito" role="status" aria-live="polite">
                            <h2 className="contactoExitoTitulo">{t('contact.success_title')}</h2>
                            <p className="contactoExitoTexto">{t('contact.success_message')}</p>
                            <Button
                                variante="outline"
                                onClick={() => {
                                    setEnviado(false);
                                    setFormulario({nombre: '', email: '', telefono: '', descripcion: '', presupuesto: ''});
                                }}
                            >
                                {t('contact.success_another')}
                            </Button>
                        </div>
                    ) : (
                        <form className="contactoFormulario" onSubmit={handleSubmit}>
                            <div className="formularioGrid">
                                {/* Nombre */}
                                <div className="campoCampo">
                                    <label htmlFor="nombre" className="campoEtiqueta">{t('contact.name_label')}</label>
                                    <Input
                                        type="text"
                                        id="nombre"
                                        name="nombre"
                                        value={formulario.nombre}
                                        onChange={handleChange}
                                        placeholder={t('contact.name_placeholder')}
                                        className="campoInput"
                                        required
                                    />
                                </div>

                                {/* Email */}
                                <div className="campoCampo">
                                    <label htmlFor="email" className="campoEtiqueta">{t('contact.email_label')}</label>
                                    <Input
                                        type="email"
                                        id="email"
                                        name="email"
                                        value={formulario.email}
                                        onChange={handleChange}
                                        placeholder="tu@email.com"
                                        className="campoInput"
                                        required
                                    />
                                </div>

                                {/* Teléfono */}
                                <div className="campoCampo">
                                    <label htmlFor="telefono" className="campoEtiqueta">{t('contact.phone_label')}</label>
                                    <Input
                                        type="tel"
                                        id="telefono"
                                        name="telefono"
                                        value={formulario.telefono}
                                        onChange={handleChange}
                                        placeholder={t('contact.phone_placeholder')}
                                        className="campoInput"
                                    />
                                </div>

                                {/* Presupuesto */}
                                <div className="campoCampo">
                                    <label className="campoEtiqueta">{t('contact.budget_label')}</label>
                                    <SelectDropdown
                                        value={formulario.presupuesto}
                                        opciones={PRESUPUESTOS_KEYS.map(p => ({value: p.value, label: t(p.key)}))}
                                        onChange={v => setFormulario(prev => ({...prev, presupuesto: v}))}
                                        ariaLabel={t('contact.budget_label')}
                                    />
                                </div>
                            </div>

                            {/* Descripción - campo completo */}
                            <div className="campoCampo campoCompleto">
                                <label htmlFor="descripcion" className="campoEtiqueta">{t('contact.project_label')}</label>
                                <Textarea
                                    id="descripcion"
                                    name="descripcion"
                                    value={formulario.descripcion}
                                    onChange={handleChange}
                                    placeholder={t('contact.project_placeholder')}
                                    className="campoTextarea"
                                    rows={6}
                                    required
                                />
                            </div>

                            <div className="formularioAcciones">
                                <Button variante="primario" tamano="mediano" className="formularioBoton">
                                    {t('contact.submit')}
                                </Button>
                                <p className="formularioNota">
                                    {t('contact.help')}
                                </p>
                            </div>
                        </form>
                    )}

                    {/* Info lateral - datos centralizados */}
                    <aside className="contactoInfo" aria-label="Información de contacto">
                        <div className="contactoInfoBloque">
                            <h3 className="contactoInfoTitulo">Email</h3>
                            <a href={`mailto:${INFO_CONTACTO.email}`} className="contactoInfoEnlace">{INFO_CONTACTO.email}</a>
                        </div>
                        <div className="contactoInfoBloque">
                            <h3 className="contactoInfoTitulo">Teléfono</h3>
                            <a href={`tel:${INFO_CONTACTO.telefono.replace(/\s/g, '')}`} className="contactoInfoEnlace">{INFO_CONTACTO.telefono}</a>
                        </div>
                        <div className="contactoInfoBloque">
                            <h3 className="contactoInfoTitulo">Ubicación</h3>
                            <p className="contactoInfoTexto">{INFO_CONTACTO.ubicacionDetalle}</p>
                        </div>
                        <div className="contactoInfoBloque">
                            <h3 className="contactoInfoTitulo">Redes</h3>
                            <div className="contactoInfoRedes">
                                {INFO_CONTACTO.redesSociales.map(red => (
                                    <a key={red.nombre} href={red.url} className="contactoInfoEnlace" target="_blank" rel="noopener noreferrer">
                                        {red.nombre}
                                    </a>
                                ))}
                            </div>
                        </div>
                    </aside>
                </div>
            </section>
        </LayoutPagina>
    );
};

export default ContactoIsland;
