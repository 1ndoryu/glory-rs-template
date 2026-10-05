/* [311A-INV] Componente para previsualizar plantillas de email renderizadas.
 * Muestra todas las plantillas en una cuadrícula agrupada por categoría.
 * Al hacer clic en una, abre un modal con el HTML renderizado en un iframe.
 * [259A-5 5b-3] Modal artesanal migrado a <Modal> + recetas (modalTitulo,
 * modalAcciones) + Button. Galeria (botones nativos) pendiente de 5b-6. */

/* [259A-5 5b-6] sentinel-disable-file html-nativo-en-vez-de-componente componente-artesanal: categorias (tabs sin componente Tabs en el DS) y tarjetas de seleccion (cards seleccionables, sin componente DS) quedan nativas; modal ya en <Modal>. */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Loader2, AlertCircle, Eye, X, ExternalLink, Mail } from 'lucide-react';
import {
    apiListTemplates,
    apiRenderTemplate,
    CATEGORY_LABELS,
    CATEGORY_ORDER,
    type TemplateMeta,
} from '../../api/admin-email-preview';
import { Modal } from '../ui/Modal';
import { Button } from '../ui/Button';
import './VistaPreviaCorreos.css';

/** Agrupa plantillas por categoría manteniendo el orden definido */
function agruparPorCategoria(templates: TemplateMeta[]): Map<string, TemplateMeta[]> {
    const grupos = new Map<string, TemplateMeta[]>();
    for (const cat of CATEGORY_ORDER) {
        const items = templates.filter(t => t.category === cat);
        if (items.length > 0) grupos.set(cat, items);
    }
    return grupos;
}

export function VistaPreviaCorreos() {
    /* Estado de vista previa agrupado (usestate-excesivo: 5 -> 2). */
    const [vista, setVista] = useState<{plantilla: TemplateMeta | null; html: string | null; cargando: boolean; error: string | null}>({
        plantilla: null, html: null, cargando: false, error: null,
    });
    const [selectedCategory, setSelectedCategory] = useState<string | null>(null);

    const { data, isLoading, error } = useQuery({
        queryKey: ['admin-email-templates'],
        queryFn: apiListTemplates,
    });

    const templates = data?.templates ?? [];
    const grupos = agruparPorCategoria(templates);

    const handleSelectTemplate = async (tmpl: TemplateMeta) => {
        setVista({plantilla: tmpl, html: null, cargando: true, error: null});
        try {
            const html = await apiRenderTemplate(tmpl.id);
            setVista(v => ({...v, html}));
        } catch (err) {
            setVista(v => ({...v, error: (err as Error).message}));
        } finally {
            setVista(v => ({...v, cargando: false}));
        }
    };

    const handleCloseModal = () => {
        setVista(v => ({...v, plantilla: null, html: null, error: null}));
    };

    if (isLoading) {
        return (
            <div className="previewVacio">
                <Loader2 className="previewSpinner" size={32} />
            </div>
        );
    }

    if (error) {
        return (
            <div className="previewError">
                <AlertCircle size={20} />
                <span>Error al cargar plantillas: {(error as Error).message}</span>
            </div>
        );
    }

    return (
        <div className="previewContenedor">
            <p className="previewDescripcion">
                Selecciona una plantilla para ver cómo se ve visualmente con datos de ejemplo.
                Los correos no se envían realmente.
            </p>

            {/* Navegación por categorías */}
            <div className="previewCategorias">
                <button
                    className={`previewCatBtn ${selectedCategory === null ? 'previewCatActiva' : ''}`}
                    onClick={() => setSelectedCategory(null)}
                >
                    Todas
                </button>
                {CATEGORY_ORDER.filter(cat => grupos.has(cat)).map(cat => (
                    <button
                        key={cat}
                        className={`previewCatBtn ${selectedCategory === cat ? 'previewCatActiva' : ''}`}
                        onClick={() => setSelectedCategory(cat)}
                    >
                        {CATEGORY_LABELS[cat] ?? cat}
                    </button>
                ))}
            </div>

            {/* Cuadrícula de plantillas */}
            {templates.length === 0 ? (
                <div className="previewVacio">
                    <Mail size={40} />
                    <p>No hay plantillas disponibles</p>
                </div>
            ) : (
                <div className="previewGrid">
                    {CATEGORY_ORDER.filter(cat => !selectedCategory || cat === selectedCategory).map(cat => {
                        const items = grupos.get(cat);
                        if (!items || items.length === 0) return null;
                        return (
                            <div key={cat} className="previewGrupo">
                                <h3 className="previewGrupoTitulo">
                                    {CATEGORY_LABELS[cat] ?? cat}
                                </h3>
                                <div className="previewGrupoGrid">
                                    {items.map(tmpl => (
                                        <button
                                            key={tmpl.id}
                                            className={`previewTarjeta ${vista.plantilla?.id === tmpl.id ? 'previewTarjetaActiva' : ''}`}
                                            onClick={() => handleSelectTemplate(tmpl)}
                                        >
                                            <div className="previewTarjetaIcono">
                                                <Eye size={20} />
                                            </div>
                                            <div className="previewTarjetaInfo">
                                                <span className="previewTarjetaLabel">{tmpl.label}</span>
                                                {tmpl.recipients && (
                                                    <span className={`previewTarjetaBadge ${tmpl.recipients === 'admin' ? 'previewBadgeAdmin' : 'previewBadgeCliente'}`}>
                                                        {tmpl.recipients === 'admin' ? 'Admin' : 'Cliente'}
                                                    </span>
                                                )}
                                            </div>
                                        </button>
                                    ))}
                                </div>
                            </div>
                        );
                    })}
                </div>
            )}

            {/* Modal de previsualización */}
            {vista.plantilla && (
                <Modal abierto onCerrar={handleCloseModal} className="modalGrande">
                    <div className="previewModalHeader">
                        <div className="previewModalEncabezado">
                            {/* [259A-5] sentinel-disable-next-line modal-con-titulo: el canon
                              * Modal.css (.modalTitulo) contradice la regla; precedente SeccionPagos 5a. */}
                            <h3 className="modalTitulo">{vista.plantilla.label}</h3>
                            <span className="previewModalId">{vista.plantilla.id}</span>
                        </div>
                        <div className="modalAcciones">
                            {vista.html && (
                                <a
                                    href={`data:text/html;charset=utf-8,${encodeURIComponent(vista.html)}`}
                                    download={`${vista.plantilla.id}.html`}
                                    className="previewDescargarBtn"
                                    title="Descargar HTML"
                                >
                                    <ExternalLink size={16} />
                                </a>
                            )}
                            <Button
                                variante="texto"
                                tamano="pequeno"
                                onClick={handleCloseModal}
                                aria-label="Cerrar"
                                title="Cerrar"
                            >
                                <X size={20} />
                            </Button>
                        </div>
                    </div>
                    <div className="previewModalCuerpo">
                        {vista.cargando ? (
                            <div className="previewModalCarga">
                                <Loader2 className="previewSpinner" size={32} />
                                <p>Renderizando plantilla...</p>
                            </div>
                        ) : vista.error ? (
                            <div className="previewModalError">
                                <AlertCircle size={20} />
                                <span>{vista.error}</span>
                            </div>
                        ) : vista.html ? (
                            <iframe
                                className="previewIframe"
                                srcDoc={vista.html}
                                title={vista.plantilla.label}
                                sandbox="allow-same-origin"
                            />
                        ) : null}
                    </div>
                </Modal>
            )}
        </div>
    );
}
