/* [074A-9] Lista de servicios en el CMS admin.
 * [114A-7] Menú 3 puntos con archivar/desarchivar/eliminar.
 * [124A-CMS7] Convertido de grid a lista vertical, mismo patrón que ListaProyectos.
 * [124A-CMS10] Drag-to-reorder via @dnd-kit, mismo patrón que ListaProyectos. */
import React, {useState} from 'react';
import {Plus, Archive, ArchiveRestore, Trash2, Globe, Eye, EyeOff, GripVertical, Search} from 'lucide-react';
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
import type {AdminService} from '../../api/admin-services';
import './ListaServicios.css';

interface ListaServiciosProps {
    servicios: AdminService[];
    cargando: boolean;
    onEditar: (servicio: AdminService) => void;
    onCrear: () => void;
    onArchivar: (id: string) => void;
    onDesarchivar?: (id: string) => void;
    onEliminar?: (id: string) => void;
    onPublicar?: (id: string) => void;
    onToggleHome?: (id: string, visible: boolean) => void;
    onReordenar?: (items: {id: string; sort_order: number}[]) => void;
}

/* Badge de status con color semántico */
function BadgeStatus({status}: {status: string}) {
    const clase = `listaServiciosBadge listaServiciosBadge--${status}`;
    const labels: Record<string, string> = {
        published: 'Publicado',
        draft: 'Borrador',
        archived: 'Archivado',
    };
    return <span className={clase}>{labels[status] ?? status}</span>;
}

/* [124A-CMS10] Fila sortable individual */
function FilaServicio({
    svc,
    menuActivo,
    setMenuActivo,
    onEditar,
    onArchivar,
    onDesarchivar,
    onEliminar,
    onPublicar,
    onToggleHome,
}: {
    svc: AdminService;
    menuActivo: string | null;
    setMenuActivo: React.Dispatch<React.SetStateAction<string | null>>;
    onEditar: (s: AdminService) => void;
    onArchivar: (id: string) => void;
    onDesarchivar?: (id: string) => void;
    onEliminar?: (id: string) => void;
    onPublicar?: (id: string) => void;
    onToggleHome?: (id: string, visible: boolean) => void;
}) {
    const {attributes, listeners, setNodeRef, transform, transition, isDragging} = useSortable({id: svc.id});
    const style = {transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0.5 : 1};

    const items: MenuContextualItem[] = [];
    if (onToggleHome) {
        items.push({
            id: 'toggle-home',
            label: svc.is_active ? 'Ocultar del Home' : 'Mostrar en Home',
            icon: svc.is_active ? <EyeOff size={14} /> : <Eye size={14} />,
            onSelect: () => onToggleHome(svc.id, !svc.is_active),
        });
    }
    if (svc.status !== 'published' && onPublicar) {
        items.push({id: 'publicar', label: 'Publicar', icon: <Globe size={14} />, onSelect: () => onPublicar(svc.id)});
    }
    if (svc.status !== 'archived') {
        items.push({id: 'archivar', label: 'Archivar', icon: <Archive size={14} />, onSelect: () => onArchivar(svc.id)});
    }
    if (svc.status === 'archived' && onDesarchivar) {
        items.push({id: 'desarchivar', label: 'Desarchivar', icon: <ArchiveRestore size={14} />, onSelect: () => onDesarchivar(svc.id)});
    }
    if (onEliminar) {
        items.push({id: 'eliminar', label: 'Eliminar', icon: <Trash2 size={14} />, danger: true, onSelect: () => onEliminar(svc.id)});
    }

    return (
        <div
            ref={setNodeRef}
            style={style}
            className={`listaServiciosFila ${!svc.is_active ? 'listaServiciosFila--inactivo' : ''}`}
        >
            <div className="listaServiciosGrip" {...attributes} {...listeners}>
                <GripVertical size={16} />
            </div>

            {svc.image_url && (
                <div className="listaServiciosMiniatura">
                    <OptimizedImage src={svc.image_url} alt={svc.title} loading="lazy" />
                </div>
            )}

            <div
                className="listaServiciosFilaInfo"
                onClick={() => onEditar(svc)}
                role="button"
                tabIndex={0}
                onKeyDown={e => { if (e.key === 'Enter') onEditar(svc); }}
            >
                <span className="listaServiciosNombre">{svc.title}</span>
                <BadgeStatus status={svc.status} />
                <span className="listaServiciosPrecio">
                    ${(svc.base_price_cents / 100).toFixed(0)} {svc.currency}
                </span>
                <span className="listaServiciosPlanes">
                    {svc.plans.length} plan{svc.plans.length !== 1 ? 'es' : ''}
                </span>
            </div>

            <div className="listaServiciosMenu" onClick={e => e.stopPropagation()}>
                <MenuContextual
                    abierto={menuActivo === svc.id}
                    onToggle={() => setMenuActivo(prev => prev === svc.id ? null : svc.id)}
                    onCerrar={() => setMenuActivo(null)}
                    ariaLabel="Acciones del servicio"
                    items={items}
                />
            </div>
        </div>
    );
}

export const ListaServicios: React.FC<ListaServiciosProps> = ({
    servicios,
    cargando,
    onEditar,
    onCrear,
    onArchivar,
    onDesarchivar,
    onEliminar,
    onPublicar,
    onToggleHome,
    onReordenar,
}) => {
    const [menuActivo, setMenuActivo] = useState<string | null>(null);
    /* [124A-SEARCH1] Búsqueda en tiempo real */
    const { busqueda, setBusqueda, filtrados } = useBusquedaLista(
        servicios, ['title', 'slug']
    );

    const sensors = useSensors(
        useSensor(PointerSensor, {activationConstraint: {distance: 5}}),
        useSensor(KeyboardSensor, {coordinateGetter: sortableKeyboardCoordinates}),
    );

    const handleDragEnd = (event: DragEndEvent) => {
        const {active, over} = event;
        if (!over || active.id === over.id) return;
        const oldIndex = servicios.findIndex(s => s.id === active.id);
        const newIndex = servicios.findIndex(s => s.id === over.id);
        if (oldIndex === -1 || newIndex === -1) return;
        const reordered = arrayMove(servicios, oldIndex, newIndex);
        const items = reordered.map((s, i) => ({id: s.id, sort_order: i}));
        onReordenar?.(items);
    };

    if (cargando) {
        return <div className="listaServiciosCargando">Cargando servicios...</div>;
    }

    return (
        <div className="listaServicios">
            <div className="listaServiciosAcciones">
                <div className="listaBusqueda">
                    <Search size={14} className="listaBusquedaIcono" />
                    <Input
                        value={busqueda}
                        onChange={e => setBusqueda(e.target.value)}
                        placeholder="Buscar servicios..."
                        className="listaBusquedaInput"
                    />
                </div>
                <Button variante="primario" tamano="pequeno" onClick={onCrear}>
                    <Plus size={14} />
                    Nuevo
                </Button>
            </div>

            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                <SortableContext items={filtrados.map(s => s.id)} strategy={verticalListSortingStrategy}>
                    <div className="listaServiciosLista">
                        {filtrados.map(svc => (
                            <FilaServicio
                                key={svc.id}
                                svc={svc}
                                menuActivo={menuActivo}
                                setMenuActivo={setMenuActivo}
                                onEditar={onEditar}
                                onArchivar={onArchivar}
                                onDesarchivar={onDesarchivar}
                                onEliminar={onEliminar}
                                onPublicar={onPublicar}
                                onToggleHome={onToggleHome}
                            />
                        ))}
                    </div>
                </SortableContext>
            </DndContext>
        </div>
    );
};
