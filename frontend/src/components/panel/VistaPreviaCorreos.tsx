/* [311A-INV] Componente para previsualizar plantillas de email renderizadas.
 * Muestra todas las plantillas en una cuadrícula agrupada por categoría.
 * Al hacer clic en una, abre un modal con el HTML renderizado en un iframe.
 * [259A-5 5b-3] Modal artesanal migrado a <Modal> + recetas (modalTitulo,
 * modalAcciones) + Button. Galeria (botones nativos) pendiente de 5b-6. */

/* [259A-5] sentinel-disable-file html-nativo-en-vez-de-componente componente-artesanal: migracion visual a componentes DS en fase 5b con verificacion en navegador (galeria nativa -> 5b-6). */
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
    const [selectedTemplate, setSelectedTemplate] = useState<TemplateMeta | null>(null);
    const [previewHtml, setPreviewHtml] = useState<string | null>(null);
    const [previewLoading, setPreviewLoading] = useState(false);
    const [previewError, setPreviewError] = useState<string | null>(null);
    const [selectedCategory, setSelectedCategory] = useState<string | null>(null);

    const { data, isLoading, error } = useQuery({
        queryKey: ['admin-email-templates'],
        queryFn: apiListTemplates,
    });

    const templates = data?.templates ?? [];
    const grupos = agruparPorCategoria(templates);

    const handleSelectTemplate = async (tmpl: TemplateMeta) => {
        setSelectedTemplate(tmpl);
        setPreviewLoading(true);
        setPreviewHtml(null);
        setPreviewError(null);
        try {
            const html = await apiRenderTemplate(tmpl.id);
            setPreviewHtml(html);
        } catch (err) {
            setPreviewError((err as Error).message);
        } finally {
            setPreviewLoading(false);
        }
    };

    const handleCloseModal = () => {
        setSelectedTemplate(null);
        setPreviewHtml(null);
        setPreviewError(null);
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
                                            className={`previewTarjeta ${selectedTemplate?.id === tmpl.id ? 'previewTarjetaActiva' : ''}`}
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
            {selectedTemplate && (
                <Modal abierto onCerrar={handleCloseModal} className="modalGrande">
                    <div className="previewModalHeader">
                        <div className="previewModalEncabezado">
                            {/* [259A-5] sentinel-disable-next-line modal-con-titulo: el canon
                              * Modal.css (.modalTitulo) contradice la regla; precedente SeccionPagos 5a. */}
                            <h3 className="modalTitulo">{selectedTemplate.label}</h3>
                            <span className="previewModalId">{selectedTemplate.id}</span>
                        </div>
                        <div className="modalAcciones">
                            {previewHtml && (
                                <a
                                    href={`data:text/html;charset=utf-8,${encodeURIComponent(previewHtml)}`}
                                    download={`${selectedTemplate.id}.html`}
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
                        {previewLoading ? (
                            <div className="previewModalCarga">
                                <Loader2 className="previewSpinner" size={32} />
                                <p>Renderizando plantilla...</p>
                            </div>
                        ) : previewError ? (
                            <div className="previewModalError">
                                <AlertCircle size={20} />
                                <span>{previewError}</span>
                            </div>
                        ) : previewHtml ? (
                            <iframe
                                className="previewIframe"
                                srcDoc={previewHtml}
                                title={selectedTemplate.label}
                                sandbox="allow-same-origin"
                            />
                        ) : null}
                    </div>
                </Modal>
            )}
        </div>
    );
}
