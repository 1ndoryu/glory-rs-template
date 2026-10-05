/* [277A-13] Modal de edición SEO para páginas estáticas.
 * Permite editar title, description, og_image_url y json_ld_type.
 * [277A-18] OG image: galería con recorte (ImageGalleryPicker).
 * [277A-18] JSON-LD type: select editable con opciones predefinidas.
 * [259A-5 5b-2] Migrado a <Modal> + recetas (modalTitulo/modalAcciones,
 * ModalBody/ModalField/ModalLabel) + Button + SelectDropdown. Clases locales
 * solo para layout especifico (header, contadores, preview OG, ayudas). */
import React, {useState, useEffect} from 'react';
import {X, ImageIcon} from 'lucide-react';
import {useMutation, useQueryClient} from '@tanstack/react-query';
import {apiUpdateSeoSetting, type SeoSetting} from '../../api/admin-seo';
import {ImageGalleryPicker} from '../ui/ImageGalleryPicker';
import {Modal, ModalBody, ModalField, ModalLabel} from '../ui/Modal';
import {Button} from '../ui/Button';
import {SelectDropdown} from '../ui/SelectDropdown';
import './ModalSeoEdit.css';

interface Props {
    setting: SeoSetting | null;
    onClose: () => void;
}

const TITLE_MIN = 30;
const TITLE_MAX = 60;
const DESC_MIN = 70;
const DESC_MAX = 160;

const JSON_LD_OPTIONS = [
    {value: '', label: '— Ninguno —'},
    {value: 'Organization+WebSite', label: 'Organization + WebSite'},
    {value: 'Organization', label: 'Organization'},
    {value: 'BreadcrumbList+Person', label: 'BreadcrumbList + Person'},
    {value: 'FAQPage', label: 'FAQPage'},
    {value: 'CollectionPage', label: 'CollectionPage'},
    {value: 'ContactPage', label: 'ContactPage'},
];

export const ModalSeoEdit: React.FC<Props> = ({setting, onClose}) => {
    const queryClient = useQueryClient();
    /* Formulario agrupado en un objeto (usestate-excesivo: 5 -> 2). */
    const [form, setForm] = useState({title: '', description: '', ogImageUrl: '', jsonLdType: ''});
    const [showGallery, setShowGallery] = useState(false);

    useEffect(() => {
        if (setting) {
            setForm({
                title: setting.title,
                description: setting.description,
                ogImageUrl: setting.og_image_url || '',
                jsonLdType: setting.json_ld_type || '',
            });
        }
    }, [setting]);

    const mutation = useMutation({
        mutationFn: () =>
            apiUpdateSeoSetting(setting!.path, {
                title: form.title.trim(),
                description: form.description.trim(),
                og_image_url: form.ogImageUrl.trim() || null,
                json_ld_type: form.jsonLdType || null,
            }),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: ['admin-seo-audit']});
            queryClient.invalidateQueries({queryKey: ['admin-seo-settings']});
            onClose();
        },
    });

    if (!setting) return null;

    const titleLen = form.title.length;
    const descLen = form.description.length;
    const titleStatus =
        titleLen === 0 ? 'error' : titleLen < TITLE_MIN || titleLen > TITLE_MAX ? 'warning' : 'ok';
    const descStatus =
        descLen === 0 ? 'error' : descLen < DESC_MIN || descLen > DESC_MAX ? 'warning' : 'ok';
    const isValid = titleLen > 0 && descLen > 0;

    return (
        <>
            <Modal abierto onCerrar={onClose} className="modalMedio">
                <div className="modalSeoHeader">
                    {/* [259A-5] sentinel-disable-next-line modal-con-titulo: el canon
                      * Modal.css (.modalTitulo) contradice la regla; precedente SeccionPagos 5a. */}
                    <h3 className="modalTitulo">Editar SEO: {setting.label}</h3>
                    <Button variante="texto" tamano="pequeno" onClick={onClose} aria-label="Cerrar">
                        <X size={18} />
                    </Button>
                </div>

                <ModalBody>
                    <ModalField>
                        <ModalLabel className="modalSeoLabel">
                            Título
                            <span className={`modalSeoContador modalSeoContador--${titleStatus}`}>
                                {titleLen}/{TITLE_MAX}
                            </span>
                        </ModalLabel>
                        <input
                            type="text"
                            className="modalInput modalSeoInput"
                            value={form.title}
                            onChange={e => setForm(f => ({...f, title: e.target.value}))}
                            maxLength={255}
                            placeholder="Título de la página para Google"
                        />
                        <span className="modalSeoAyuda">
                            {titleLen < TITLE_MIN && titleLen > 0 && `⚠ Mínimo recomendado: ${TITLE_MIN} caracteres`}
                            {titleLen > TITLE_MAX && `⚠ Máximo recomendado: ${TITLE_MAX} caracteres`}
                            {titleLen >= TITLE_MIN && titleLen <= TITLE_MAX && '✓ Longitud ideal'}
                        </span>
                    </ModalField>

                    <ModalField>
                        <ModalLabel className="modalSeoLabel">
                            Descripción
                            <span className={`modalSeoContador modalSeoContador--${descStatus}`}>
                                {descLen}/{DESC_MAX}
                            </span>
                        </ModalLabel>
                        <textarea
                            className="modalInput modalSeoTextarea"
                            value={form.description}
                            onChange={e => setForm(f => ({...f, description: e.target.value}))}
                            maxLength={500}
                            rows={3}
                            placeholder="Descripción para los resultados de búsqueda"
                        />
                        <span className="modalSeoAyuda">
                            {descLen < DESC_MIN && descLen > 0 && `⚠ Mínimo recomendado: ${DESC_MIN} caracteres`}
                            {descLen > DESC_MAX && `⚠ Máximo recomendado: ${DESC_MAX} caracteres`}
                            {descLen >= DESC_MIN && descLen <= DESC_MAX && '✓ Longitud ideal'}
                        </span>
                    </ModalField>

                    {/* [277A-18] Imagen OG: preview + botón galería */}
                    <ModalField>
                        <ModalLabel>Imagen OG</ModalLabel>
                        <div className="modalSeoOgPreview">
                            {form.ogImageUrl ? (
                                <img
                                    src={form.ogImageUrl}
                                    alt="OG Preview"
                                    className="modalSeoOgThumb"
                                />
                            ) : (
                                <div className="modalSeoOgVacio">
                                    <ImageIcon size={20} />
                                    <span>Sin imagen OG</span>
                                </div>
                            )}
                        </div>
                        <div className="modalAcciones">
                            <Button
                                variante="outline"
                                tamano="pequeno"
                                onClick={() => setShowGallery(true)}
                            >
                                <ImageIcon size={14} />
                                Elegir de galería
                            </Button>
                            {form.ogImageUrl && (
                                <Button
                                    variante="texto"
                                    tamano="pequeno"
                                    onClick={() => setForm(f => ({...f, ogImageUrl: ''}))}
                                >
                                    <X size={14} />
                                    Quitar
                                </Button>
                            )}
                        </div>
                        <span className="modalSeoAyuda">Recomendado: 1200×630px. Se recorta automáticamente.</span>
                    </ModalField>

                    {/* [277A-18] JSON-LD type: select editable */}
                    <ModalField>
                        <ModalLabel>Tipo JSON-LD</ModalLabel>
                        <SelectDropdown
                            value={form.jsonLdType}
                            opciones={JSON_LD_OPTIONS}
                            onChange={jsonLdType => setForm(f => ({...f, jsonLdType}))}
                            ariaLabel="Tipo JSON-LD"
                        />
                        <span className="modalSeoAyuda">Schema structured data para Google rich snippets</span>
                    </ModalField>

                    <ModalField className="modalSeoCampo--readonly">
                        <ModalLabel>Ruta</ModalLabel>
                        <input
                            type="text"
                            className="modalInput modalSeoInput modalSeoInput--disabled"
                            value={setting.path}
                            disabled
                        />
                    </ModalField>
                </ModalBody>

                <div className="modalAcciones">
                    <Button
                        variante="texto"
                        onClick={onClose}
                    >
                        Cancelar
                    </Button>
                    <Button
                        variante="secundario"
                        onClick={() => mutation.mutate()}
                        disabled={!isValid || mutation.isPending}
                    >
                        {mutation.isPending ? 'Guardando...' : 'Guardar'}
                    </Button>
                </div>

                {mutation.isError && (
                    <div className="modalSeoError">
                        Error al guardar: {(mutation.error as Error).message}
                    </div>
                )}
            </Modal>

            {showGallery && (
                <ImageGalleryPicker
                    onSelect={url => {
                        setForm(f => ({...f, ogImageUrl: url}));
                        setShowGallery(false);
                    }}
                    onClose={() => setShowGallery(false)}
                />
            )}
        </>
    );
};
