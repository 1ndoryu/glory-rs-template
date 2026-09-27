/* [074A-12] Editor de proyecto CMS — Modal con tabs: General | Media | Tech | SEO.
 * Más campos que EditorBlog: cliente, categorías, tecnologías, enlaces, skills.
 * Lógica de formulario extraída a useEditorProyecto.
 * sentinel-disable-file html-nativo-en-vez-de-componente: solo las tabs del editor
 * usan <button> nativo (sin componente Tabs en el DS; mismo patron EditorBlog).
 * sentinel-disable-file limite-lineas: Editor modal con 4 tabs (General/Media/Tech/SEO) — dividir
 * cada tab en componente aparte añadiría prop-drilling sin beneficio real, el archivo es cohesivo. */
import React, { useState, useCallback, useEffect } from 'react';
import {X} from 'lucide-react';
import { Modal } from '../ui/Modal';
import { Input } from '../ui/Input';
import { Textarea } from '../ui/Textarea';
import { Button } from '../ui/Button';
import { SlugInput } from '../ui/SlugInput';
import { UploadImage } from '../ui/UploadImage';
import { IconPickerEnlace } from './IconPickerEnlace';
import { GaleriaEditor } from './GaleriaEditor';
import { useEditorProyecto } from '../../hooks/useEditorProyecto';
import type { AdminProject, CreateProjectBody, UpdateProjectBody } from '../../api/admin-projects';
import './EditorProyecto.css';

type TabEditor = 'general' | 'media' | 'tech' | 'seo';

interface EditorProyectoProps {
    abierto: boolean;
    onCerrar: () => void;
    proyecto: AdminProject | null;
    onGuardar: (body: CreateProjectBody | UpdateProjectBody) => Promise<void>;
    guardando: boolean;
}

const TABS: { id: TabEditor; label: string }[] = [
    { id: 'general', label: 'General' },
    { id: 'media', label: 'Media' },
    { id: 'tech', label: 'Tech' },
    { id: 'seo', label: 'SEO' },
];

export const EditorProyecto: React.FC<EditorProyectoProps> = ({
    abierto,
    onCerrar,
    proyecto,
    onGuardar,
    guardando,
}) => {
    const [tab, setTab] = useState<TabEditor>('general');
    const form = useEditorProyecto(proyecto, abierto);

    const handleGuardar = useCallback(async () => {
        await onGuardar(form.buildBody());
    }, [form, onGuardar]);

    /* [20CA-14] Campos array como string local: se parsea a array en blur.
     * Esto permite escribir comas sin que desaparezcan al instante. */
    const [categoriasInput, setCategoriasInput] = useState(form.categorias.join(', '));
    const [tecnologiasInput, setTecnologiasInput] = useState(form.tecnologias.join(', '));
    useEffect(() => {
        setCategoriasInput(form.categorias.join(', '));
    }, [form.categorias]);
    useEffect(() => {
        setTecnologiasInput(form.tecnologias.join(', '));
    }, [form.tecnologias]);
    const handleCategoriasBlur = useCallback(() => {
        form.setCategorias(categoriasInput.split(',').map(t => t.trim()).filter(Boolean));
    }, [categoriasInput, form]);
    const handleTecnologiasBlur = useCallback(() => {
        form.setTecnologias(tecnologiasInput.split(',').map(t => t.trim()).filter(Boolean));
    }, [tecnologiasInput, form]);

    return (
        <Modal abierto={abierto} onCerrar={onCerrar} className="modalGrande modalSinPadding editorProyectoModal">
            <div className="editorProyectoTabs">
                {TABS.map(t => (
                    <button
                        key={t.id}
                        type="button"
                        className={`editorProyectoTab ${tab === t.id ? 'editorProyectoTab--activo' : ''}`}
                        onClick={() => setTab(t.id)}
                    >
                        {t.label}
                    </button>
                ))}
            </div>

            <div className="editorProyectoCuerpo">
                {tab === 'general' && (
                    <div className="editorProyectoSeccion">
                        <label className="editorProyectoLabel">
                            Título
                            <Input
                                value={form.titulo}
                                onChange={e => form.setTitulo(e.target.value)}
                                placeholder="Nombre del proyecto"
                            />
                        </label>

                        <SlugInput titulo={form.titulo} valor={form.slug} onChange={form.setSlug} />

                        <label className="editorProyectoLabel">
                            Cliente
                            <Input
                                value={form.cliente}
                                onChange={e => form.setCliente(e.target.value)}
                                placeholder="Nombre del cliente"
                            />
                        </label>

                        <label className="editorProyectoLabel">
                            Descripción
                            <Textarea
                                className="editorProyectoTextarea"
                                value={form.descripcion}
                                onChange={e => form.setDescripcion(e.target.value)}
                                placeholder="Descripción breve del proyecto"
                                rows={4}
                            />
                        </label>

                        <label className="editorProyectoLabel">
                            Categorías (separadas por comas)
                            <Input
                                value={categoriasInput}
                                onChange={e => setCategoriasInput(e.target.value)}
                                onBlur={handleCategoriasBlur}
                                placeholder="Desarrollo web, Branding, E-commerce"
                            />
                        </label>

                        {/* [124A-SHOW1] Categoría showcase: agrupa proyectos en el home */}
                        <label className="editorProyectoLabel">
                            Showcase Category
                            <Input
                                value={form.showcaseCategory}
                                onChange={e => form.setShowcaseCategory(e.target.value)}
                                placeholder="Website & Digital Experiences"
                            />
                            <span className="editorProyectoHint">
                                Proyectos con la misma categoría showcase se agrupan juntos en el inicio.
                            </span>
                        </label>

                        {/* [124A-DETAIL1] Título alternativo para la página de detalle del proyecto */}
                        <label className="editorProyectoLabel">
                            Título de detalle
                            <Input
                                value={form.detailTitle}
                                onChange={e => form.setDetailTitle(e.target.value)}
                                placeholder="Título alternativo para la página individual"
                            />
                            <span className="editorProyectoHint">
                                Si se deja vacío, se usa el título principal del proyecto.
                            </span>
                        </label>

                        {/* [124A-PROJ] Visibilidad: controla dónde aparece el proyecto en el sitio */}
                        <div className="editorProyectoVisibilidad">
                            <span className="editorProyectoLabel">Visibilidad</span>
                            <label className="editorProyectoCheckboxLabel">
                                <input
                                    type="checkbox"
                                    checked={form.inCarousel}
                                    onChange={e => form.setInCarousel(e.target.checked)}
                                />
                                Aparece en la galería del inicio
                            </label>
                            <label className="editorProyectoCheckboxLabel">
                                <input
                                    type="checkbox"
                                    checked={form.isFeatured}
                                    onChange={e => form.setIsFeatured(e.target.checked)}
                                />
                                Aparece en Selected Work
                            </label>
                        </div>

                    </div>
                )}

                {tab === 'media' && (
                    <div className="editorProyectoSeccion">
                        <UploadImage
                            valor={form.imagenUrl}
                            onChange={form.setImagenUrl}
                            etiqueta="Imagen destacada"
                        />

                        {/* Imagen personalizada para la galería del hero (inicio).
                         * Si no se sube, se usa la imagen destacada. */}
                        <UploadImage
                            valor={form.imagenGaleria}
                            onChange={form.setImagenGaleria}
                            etiqueta="Imagen para galería del inicio (opcional)"
                        />

                        {/* [124A-DETAIL1] Toggle: usar primera imagen de galería como portada en detalle */}
                        <label className="editorProyectoCheckboxLabel">
                            <input
                                type="checkbox"
                                checked={form.useFirstGalleryImage}
                                onChange={e => form.setUseFirstGalleryImage(e.target.checked)}
                            />
                            Usar primera imagen de galería como portada en la página de detalle
                        </label>

                        {/* [154A-10] Galería de imágenes del proyecto */}
                        <GaleriaEditor
                            galeria={form.galeria}
                            onChange={form.setGaleria}
                        />
                    </div>
                )}

                {tab === 'tech' && (
                    <div className="editorProyectoSeccion">
                        <label className="editorProyectoLabel">
                            Tecnologías (separadas por comas)
                            <Input
                                value={tecnologiasInput}
                                onChange={e => setTecnologiasInput(e.target.value)}
                                onBlur={handleTecnologiasBlur}
                                placeholder="React, TypeScript, Rust, PostgreSQL"
                            />
                        </label>

                        <div className="editorProyectoEnlaces">
                            <span className="editorProyectoLabel">Enlaces</span>
                            {form.enlaces.map((enlace, idx) => (
                                <div key={`enlace-${enlace.tipo}-${enlace.url}`} className="editorProyectoEnlaceRow">
                                    <IconPickerEnlace
                                        value={enlace.tipo}
                                        onChange={nuevoTipo => {
                                            const nuevos = [...form.enlaces];
                                            nuevos[idx] = { ...nuevos[idx], tipo: nuevoTipo };
                                            form.setEnlaces(nuevos);
                                        }}
                                    />
                                    <Input
                                        value={enlace.url}
                                        onChange={e => {
                                            const nuevos = [...form.enlaces];
                                            nuevos[idx] = { ...nuevos[idx], url: e.target.value };
                                            form.setEnlaces(nuevos);
                                        }}
                                        placeholder="https://..."
                                    />
                                    <Button
                                        type="button"
                                        variante="texto"
                                        className="editorProyectoEnlaceEliminar"
                                        onClick={() => form.setEnlaces(form.enlaces.filter((_, i) => i !== idx))}
                                        aria-label="Eliminar enlace"
                                    >
                                        <X size={14} />
                                    </Button>
                                </div>
                            ))}
                            <Button
                                variante="texto"
                                tamano="pequeno"
                                onClick={() => form.setEnlaces([...form.enlaces, { tipo: 'web', url: '' }])}
                            >
                                + Añadir enlace
                            </Button>
                        </div>

                        <div className="editorProyectoSkills">
                            <span className="editorProyectoLabel">Skills</span>
                            {form.skills.map((skill, idx) => (
                                <div key={`skill-${skill.titulo}-${idx}`} className="editorProyectoSkillRow">
                                    <Input
                                        value={skill.titulo}
                                        onChange={e => {
                                            const nuevos = [...form.skills];
                                            nuevos[idx] = { ...nuevos[idx], titulo: e.target.value };
                                            form.setSkills(nuevos);
                                        }}
                                        placeholder="Título del skill"
                                    />
                                    <Input
                                        value={skill.descripcion ?? ''}
                                        onChange={e => {
                                            const nuevos = [...form.skills];
                                            nuevos[idx] = { ...nuevos[idx], descripcion: e.target.value || undefined };
                                            form.setSkills(nuevos);
                                        }}
                                        placeholder="Descripción (opcional)"
                                    />
                                    <Button
                                        type="button"
                                        variante="texto"
                                        className="editorProyectoSkillEliminar"
                                        onClick={() => form.setSkills(form.skills.filter((_, i) => i !== idx))}
                                        aria-label="Eliminar skill"
                                    >
                                        <X size={14} />
                                    </Button>
                                </div>
                            ))}
                            <Button
                                variante="texto"
                                tamano="pequeno"
                                onClick={() => form.setSkills([...form.skills, { titulo: '' }])}
                            >
                                + Añadir skill
                            </Button>
                        </div>
                    </div>
                )}

                {tab === 'seo' && (
                    <div className="editorProyectoSeccion">
                        <label className="editorProyectoLabel">
                            Meta título
                            <Input
                                value={form.metaTitle}
                                onChange={e => form.setMetaTitle(e.target.value)}
                                placeholder="Título para buscadores"
                                maxLength={70}
                            />
                        </label>

                        <label className="editorProyectoLabel">
                            Meta descripción
                            <Textarea
                                className="editorProyectoTextarea"
                                value={form.metaDescription}
                                onChange={e => form.setMetaDescription(e.target.value)}
                                placeholder="Descripción para buscadores (max 160 caracteres)"
                                rows={3}
                                maxLength={160}
                            />
                        </label>

                        <div className="editorProyectoSeoPreview">
                            <span className="editorProyectoSeoPreviewLabel">Vista previa en Google</span>
                            <div className="editorProyectoSeoCard">
                                <span className="editorProyectoSeoTitulo">
                                    {form.metaTitle || form.titulo || 'Título del proyecto'}
                                </span>
                                <span className="editorProyectoSeoUrl">
                                    nakomi.studio/proyectos/{form.slug || 'slug'}
                                </span>
                                <span className="editorProyectoSeoDesc">
                                    {form.metaDescription || form.descripcion || 'Descripción del proyecto...'}
                                </span>
                            </div>
                        </div>
                    </div>
                )}
            </div>

            <div className="editorProyectoFooter">
                <div className="editorProyectoEstado">
                    <Button
                        type="button"
                        variante="outline"
                        tamano="pequeno"
                        className={form.status === 'draft' ? 'editorProyectoStatusBtn--activo' : ''}
                        onClick={() => form.setStatus('draft')}
                    >
                        Borrador
                    </Button>
                    <Button
                        type="button"
                        variante="outline"
                        tamano="pequeno"
                        className={form.status === 'published' ? 'editorProyectoStatusBtn--activo' : ''}
                        onClick={() => form.setStatus('published')}
                    >
                        Publicado
                    </Button>
                </div>
                <Button
                    variante="primario"
                    tamano="pequeno"
                    onClick={handleGuardar}
                    disabled={guardando || !form.titulo.trim() || !form.slug.trim()}
                >
                    {guardando ? 'Guardando...' : 'Guardar'}
                </Button>
            </div>
        </Modal>
    );
};
