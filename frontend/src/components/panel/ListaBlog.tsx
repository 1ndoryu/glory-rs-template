/* [074A-11] Lista de blog posts en el CMS admin.
 * [114A-7] Menú 3 puntos con archivar/desarchivar/eliminar.
 * [124A-CMS7] Convertido de grid a lista vertical, mismo patrón que ListaServicios/ListaProyectos.
 * [124A-CMS10] Drag-to-reorder via @dnd-kit. */
import React, {useState} from 'react';
import {Plus, Archive, ArchiveRestore, Trash2, Globe, GripVertical, Star, Search} from 'lucide-react';
import {
    DndContext,
    closestCenter,
    KeyboardSensor,
    PointerSensor,
    useSensor,
    useSensors,
    type DragEndEvent,
} from '@dnd-kit/core';
import {
    arrayMove,
    SortableContext,
    sortableKeyboardCoordinates,
    useSortable,
    verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import {CSS} from '@dnd-kit/utilities';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import OptimizedImage from '../ui/OptimizedImage';
import {MenuContextual, type MenuContextualItem} from '../ui/ContextMenu';
import {useBusquedaLista} from '../../hooks/useBusquedaLista';
import type {AdminBlogPost} from '../../api/admin-blog';
import './ListaBlog.css';

interface ListaBlogProps {
    posts: AdminBlogPost[];
    cargando: boolean;
    onEditar: (post: AdminBlogPost) => void;
    onCrear: () => void;
    onArchivar: (id: string) => void;
    onDesarchivar?: (id: string) => void;
    onEliminar?: (id: string) => void;
    onPublicar?: (id: string) => void;
    onReordenar?: (items: {id: string; sort_order: number}[]) => void;
    onToggleFeatured?: (id: string, featured: boolean) => void;
}

/* Badge de status con color semántico */
function BadgeStatus({status}: {status: string}) {
    const clase = `listaBlogBadge listaBlogBadge--${status}`;
    const labels: Record<string, string> = {
        published: 'Publicado',
        draft: 'Borrador',
        archived: 'Archivado',
    };
    return <span className={clase}>{labels[status] ?? status}</span>;
}

/* Formatea fecha ISO a formato legible */
function formatearFecha(iso: string): string {
    try {
        return new Date(iso).toLocaleDateString('es', {day: 'numeric', month: 'short', year: 'numeric'});
    } catch {
        return iso;
    }
}

/* [124A-CMS10] Fila sortable individual */
function FilaBlogPost({
    post,
    menuActivo,
    setMenuActivo,
    onEditar,
    onArchivar,
    onDesarchivar,
    onEliminar,
    onPublicar,
    onToggleFeatured,
}: {
    post: AdminBlogPost;
    menuActivo: string | null;
    setMenuActivo: React.Dispatch<React.SetStateAction<string | null>>;
    onEditar: (p: AdminBlogPost) => void;
    onArchivar: (id: string) => void;
    onDesarchivar?: (id: string) => void;
    onEliminar?: (id: string) => void;
    onPublicar?: (id: string) => void;
    onToggleFeatured?: (id: string, featured: boolean) => void;
}) {
    const {attributes, listeners, setNodeRef, transform, transition, isDragging} = useSortable({id: post.id});
    const style = {transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0.5 : 1};

    const items: MenuContextualItem[] = [];
    if (post.status !== 'published' && onPublicar) {
        items.push({id: 'publicar', label: 'Publicar', icon: <Globe size={14} />, onSelect: () => onPublicar(post.id)});
    }
    if (post.status !== 'archived') {
        items.push({id: 'archivar', label: 'Archivar', icon: <Archive size={14} />, onSelect: () => onArchivar(post.id)});
    }
    if (post.status === 'archived' && onDesarchivar) {
        items.push({id: 'desarchivar', label: 'Desarchivar', icon: <ArchiveRestore size={14} />, onSelect: () => onDesarchivar(post.id)});
    }
    if (onEliminar) {
        items.push({id: 'eliminar', label: 'Eliminar', icon: <Trash2 size={14} />, danger: true, onSelect: () => onEliminar(post.id)});
    }

    return (
        <div
            ref={setNodeRef}
            style={style}
            className={`listaBlogFila ${post.status === 'archived' ? 'listaBlogFila--inactivo' : ''}`}
        >
            <div className="listaBlogGrip" {...attributes} {...listeners}>
                <GripVertical size={16} />
            </div>

            {/* [124A-BLOG1] Toggle featured: estrella clickeable */}
            <Button
                variante="texto"
                className={`listaBlogStar ${post.is_featured ? 'listaBlogStar--activo' : ''}`}
                onClick={() => onToggleFeatured?.(post.id, !post.is_featured)}
                title={post.is_featured ? 'Quitar de destacados' : 'Marcar como destacado'}
                aria-label={post.is_featured ? 'Quitar de destacados' : 'Marcar como destacado'}
            >
                <Star size={16} fill={post.is_featured ? 'currentColor' : 'none'} />
            </Button>

            {post.featured_image && (
                <div className="listaBlogMiniatura">
                    <OptimizedImage src={post.featured_image} alt={post.title} loading="lazy" />
                </div>
            )}

            <div
                className="listaBlogFilaInfo"
                onClick={() => onEditar(post)}
                role="button"
                tabIndex={0}
                onKeyDown={e => { if (e.key === 'Enter') onEditar(post); }}
            >
                <span className="listaBlogNombre">{post.title}</span>
                <BadgeStatus status={post.status} />
                <span className="listaBlogFecha">
                    {formatearFecha(post.created_at)}
                </span>
                {post.tags.length > 0 && (
                    <span className="listaBlogTags">
                        {post.tags.slice(0, 3).join(', ')}
                    </span>
                )}
            </div>

            <div className="listaBlogMenu" onClick={e => e.stopPropagation()}>
                <MenuContextual
                    abierto={menuActivo === post.id}
                    onToggle={() => setMenuActivo(prev => prev === post.id ? null : post.id)}
                    onCerrar={() => setMenuActivo(null)}
                    ariaLabel="Acciones del post"
                    items={items}
                />
            </div>
        </div>
    );
}

export const ListaBlog: React.FC<ListaBlogProps> = ({
    posts,
    cargando,
    onEditar,
    onCrear,
    onArchivar,
    onDesarchivar,
    onEliminar,
    onPublicar,
    onReordenar,
    onToggleFeatured,
}) => {
    const [menuActivo, setMenuActivo] = useState<string | null>(null);
    /* [124A-SEARCH1] Búsqueda en tiempo real */
    const { busqueda, setBusqueda, filtrados } = useBusquedaLista(
        posts, ['title', 'slug']
    );

    const sensors = useSensors(
        useSensor(PointerSensor, {activationConstraint: {distance: 5}}),
        useSensor(KeyboardSensor, {coordinateGetter: sortableKeyboardCoordinates}),
    );

    const handleDragEnd = (event: DragEndEvent) => {
        const {active, over} = event;
        if (!over || active.id === over.id) return;
        const oldIndex = posts.findIndex(p => p.id === active.id);
        const newIndex = posts.findIndex(p => p.id === over.id);
        if (oldIndex === -1 || newIndex === -1) return;
        const reordered = arrayMove(posts, oldIndex, newIndex);
        const items = reordered.map((p, i) => ({id: p.id, sort_order: i}));
        onReordenar?.(items);
    };

    if (cargando) {
        return <div className="listaBlogCargando">Cargando posts...</div>;
    }

    return (
        <div className="listaBlog">
            <div className="listaBlogAcciones">
                <div className="listaBusqueda">
                    <Search size={14} className="listaBusquedaIcono" />
                    <Input
                        value={busqueda}
                        onChange={e => setBusqueda(e.target.value)}
                        placeholder="Buscar posts..."
                        className="listaBusquedaInput"
                    />
                </div>
                <Button variante="primario" tamano="pequeno" onClick={onCrear}>
                    <Plus size={14} />
                    Nuevo
                </Button>
            </div>

            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                <SortableContext items={filtrados.map(p => p.id)} strategy={verticalListSortingStrategy}>
                    <div className="listaBlogLista">
                        {filtrados.map(post => (
                            <FilaBlogPost
                                key={post.id}
                                post={post}
                                menuActivo={menuActivo}
                                setMenuActivo={setMenuActivo}
                                onEditar={onEditar}
                                onArchivar={onArchivar}
                                onDesarchivar={onDesarchivar}
                                onEliminar={onEliminar}
                                onPublicar={onPublicar}
                                onToggleFeatured={onToggleFeatured}
                            />
                        ))}
                    </div>
                </SortableContext>
            </DndContext>
        </div>
    );
};
