/* [074A-12] Lista de proyectos en el CMS admin.
 * [114A-7] Menú 3 puntos con archivar/desarchivar/eliminar.
 * [124A-CMS3] Convertido de grid a lista vertical con drag-to-reorder via @dnd-kit.
 * El orden se persiste en BD al soltar (sort_order batch update). */
import React, {useState} from 'react';
import { Plus, Archive, ArchiveRestore, Trash2, Globe, GripVertical, Star, Search } from 'lucide-react';
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
import { CSS } from '@dnd-kit/utilities';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import OptimizedImage from '../ui/OptimizedImage';
import { MenuContextual, type MenuContextualItem } from '../ui/ContextMenu';
import { useBusquedaLista } from '../../hooks/useBusquedaLista';
import type { AdminProject } from '../../api/admin-projects';
import './ListaShared.css';
import './ListaProyectos.css';

interface ListaProyectosProps {
    proyectos: AdminProject[];
    cargando: boolean;
    onEditar: (proyecto: AdminProject) => void;
    onCrear: () => void;
    onArchivar: (id: string) => void;
    onDesarchivar?: (id: string) => void;
    onEliminar?: (id: string) => void;
    onPublicar?: (id: string) => void;
    onReordenar?: (items: {id: string; sort_order: number}[]) => void;
    onToggleFeatured?: (id: string, featured: boolean) => void;
}

function BadgeStatus({ status }: { status: string }) {
    const clase = `listaProyectosBadge listaProyectosBadge--${status}`;
    const labels: Record<string, string> = {
        published: 'Publicado',
        draft: 'Borrador',
        archived: 'Archivado',
    };
    return <span className={clase}>{labels[status] ?? status}</span>;
}

/* [124A-CMS3] Fila sortable individual */
function FilaProyecto({
    proyecto,
    menuActivo,
    setMenuActivo,
    onEditar,
    onArchivar,
    onDesarchivar,
    onEliminar,
    onPublicar,
    onToggleFeatured,
}: {
    proyecto: AdminProject;
    menuActivo: string | null;
    setMenuActivo: React.Dispatch<React.SetStateAction<string | null>>;
    onEditar: (p: AdminProject) => void;
    onArchivar: (id: string) => void;
    onDesarchivar?: (id: string) => void;
    onEliminar?: (id: string) => void;
    onPublicar?: (id: string) => void;
    onToggleFeatured?: (id: string, featured: boolean) => void;
}) {
    const {
        attributes,
        listeners,
        setNodeRef,
        transform,
        transition,
        isDragging,
    } = useSortable({ id: proyecto.id });

    const style = {
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.5 : 1,
    };

    const items: MenuContextualItem[] = [];
    if (proyecto.status !== 'published' && onPublicar) {
        items.push({id: 'publicar', label: 'Publicar', icon: <Globe size={14} />, onSelect: () => onPublicar(proyecto.id)});
    }
    if (proyecto.status !== 'archived') {
        items.push({id: 'archivar', label: 'Archivar', icon: <Archive size={14} />, onSelect: () => onArchivar(proyecto.id)});
    }
    if (proyecto.status === 'archived' && onDesarchivar) {
        items.push({id: 'desarchivar', label: 'Desarchivar', icon: <ArchiveRestore size={14} />, onSelect: () => onDesarchivar(proyecto.id)});
    }
    if (onEliminar) {
        items.push({id: 'eliminar', label: 'Eliminar', icon: <Trash2 size={14} />, danger: true, onSelect: () => onEliminar(proyecto.id)});
    }

    return (
        <div
            ref={setNodeRef}
            style={style}
            className={`listaFila listaProyectosFila ${proyecto.status === 'archived' ? 'listaProyectosFila--inactivo' : ''}`}
        >
            <div className="listaProyectosGrip" {...attributes} {...listeners}>
                <GripVertical size={16} />
            </div>

            {/* [124A-CMS2] Toggle featured: estrella clickeable */}
            <Button
                variante="texto"
                className={`listaProyectosStar ${proyecto.is_featured ? 'listaProyectosStar--activo' : ''}`}
                onClick={() => onToggleFeatured?.(proyecto.id, !proyecto.is_featured)}
                title={proyecto.is_featured ? 'Quitar de Selected Work' : 'Agregar a Selected Work'}
                aria-label={proyecto.is_featured ? 'Quitar de Selected Work' : 'Agregar a Selected Work'}
            >
                <Star size={16} fill={proyecto.is_featured ? 'currentColor' : 'none'} />
            </Button>

            {proyecto.featured_image && (
                <div className="listaProyectosMiniatura">
                    <OptimizedImage src={proyecto.featured_image} alt={proyecto.title} loading="lazy" />
                </div>
            )}

            <div
                className="listaProyectosFilaInfo"
                onClick={() => onEditar(proyecto)}
                role="button"
                tabIndex={0}
                onKeyDown={e => { if (e.key === 'Enter') onEditar(proyecto); }}
            >
                <span className="listaProyectosNombre">{proyecto.title}</span>
                <BadgeStatus status={proyecto.status} />
                {proyecto.categories.length > 0 && (
                    <span className="listaProyectosCategorias">
                        {proyecto.categories.slice(0, 3).join(', ')}
                    </span>
                )}
            </div>

            <div className="listaMenuContextual" onClick={e => e.stopPropagation()}>
                <MenuContextual
                    abierto={menuActivo === proyecto.id}
                    onToggle={() => setMenuActivo(prev => prev === proyecto.id ? null : proyecto.id)}
                    onCerrar={() => setMenuActivo(null)}
                    ariaLabel="Acciones del proyecto"
                    items={items}
                />
            </div>
        </div>
    );
}

export const ListaProyectos: React.FC<ListaProyectosProps> = ({
    proyectos,
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
        proyectos, ['title', 'slug', 'client', 'categories']
    );

    const sensors = useSensors(
        useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
        useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
    );

    const handleDragEnd = (event: DragEndEvent) => {
        const {active, over} = event;
        if (!over || active.id === over.id) return;

        const oldIndex = proyectos.findIndex(p => p.id === active.id);
        const newIndex = proyectos.findIndex(p => p.id === over.id);
        if (oldIndex === -1 || newIndex === -1) return;

        const reordered = arrayMove(proyectos, oldIndex, newIndex);
        const items = reordered.map((p, i) => ({ id: p.id, sort_order: i }));
        onReordenar?.(items);
    };

    if (cargando) {
        return <div className="listaProyectosCargando">Cargando proyectos...</div>;
    }

    return (
        <div className="listaProyectos">
            <div className="listaBarraAcciones">
                <div className="listaBusqueda">
                    <Search size={14} className="listaBusquedaIcono" />
                    <Input
                        value={busqueda}
                        onChange={e => setBusqueda(e.target.value)}
                        placeholder="Buscar proyectos..."
                        className="listaBusquedaInput"
                    />
                </div>
                <Button variante="primario" tamano="pequeno" onClick={onCrear}>
                    <Plus size={14} />
                    Nuevo
                </Button>
            </div>

            <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={handleDragEnd}
            >
                <SortableContext
                    items={filtrados.map(p => p.id)}
                    strategy={verticalListSortingStrategy}
                >
                    <div className="listaProyectosLista">
                        {filtrados.map(proyecto => (
                            <FilaProyecto
                                key={proyecto.id}
                                proyecto={proyecto}
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
