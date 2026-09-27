/* [154A-10] Editor de galería de imágenes del proyecto.
 * [124A-PROJ1] Soporta GaleriaImagen con layout full/half (1/1 o 1/2 ancho).
 * [124A-GAL1] Drag-to-reorder con @dnd-kit para reorganizar imágenes.
 * [259A-5 5b-6] sentinel-disable-file html-nativo-en-vez-de-componente: solo el
 * input[type=file] oculto queda nativo (sin componente DS); botones migrados a <Button>. */
import React, { useCallback, useRef, useState } from 'react';
import { X, Maximize2, Columns2, GripVertical } from 'lucide-react';
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors } from '@dnd-kit/core';
import { SortableContext, rectSortingStrategy, useSortable, arrayMove } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { DragEndEvent } from '@dnd-kit/core';
import { Button } from '../ui/Button';
import { apiUploadImage } from '../../api/uploads';
import type { GaleriaImagen } from '../../types/contenido';
import './EditorProyecto.css';
import './ListaShared.css';

interface GaleriaEditorProps {
    galeria: GaleriaImagen[];
    onChange: (galeria: GaleriaImagen[]) => void;
}

/* [124A-GAL1] Item sortable individual de la galería */
const GaleriaItemSortable: React.FC<{
    img: GaleriaImagen;
    idx: number;
    id: string;
    onToggleLayout: (idx: number) => void;
    onRemove: (idx: number) => void;
}> = ({ img, idx, id, onToggleLayout, onRemove }) => {
    const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id });

    /* [259A-6] dnd-kit via --var (style prop solo inyecta --var).
     * [279A-1] dnd-kit exige style object en el nodo sortable; las --var son
     * mediciones por frame de la libreria (transform/transition/opacity), sin
     * equivalente declarativo. varsense-disable-next-line cssInlineReact */
    const style: React.CSSProperties = {
        '--lista-sortable-transform': CSS.Transform.toString(transform),
        '--lista-sortable-transition': transition,
        '--lista-sortable-opacity': isDragging ? 0.5 : 1,
    } as React.CSSProperties;

    return (
        <div
            ref={setNodeRef}
            /* [279A-1] Ver justificacion dnd-kit arriba. varsense-disable-next-line cssInlineReact */
            style={style}
            className={`utilSortable editorProyectoGaleriaItem ${img.layout === 'half' ? 'editorProyectoGaleriaItem--half' : ''}`}
        >
            <div className="editorProyectoGaleriaGrip" {...attributes} {...listeners}>
                <GripVertical size={14} />
            </div>
            <img src={img.url} alt={`Galería ${idx + 1}`} loading="lazy" />
            <div className="editorProyectoGaleriaControles">
                <Button
                    type="button"
                    variante="texto"
                    className="editorProyectoGaleriaLayout"
                    onClick={() => onToggleLayout(idx)}
                    title={img.layout === 'full' ? 'Cambiar a 1/2 ancho' : 'Cambiar a ancho completo'}
                >
                    {img.layout === 'full' ? <Maximize2 size={14} /> : <Columns2 size={14} />}
                    <span>{img.layout === 'full' ? '1/1' : '1/2'}</span>
                </Button>
                <Button
                    type="button"
                    variante="texto"
                    className="editorProyectoGaleriaEliminar"
                    onClick={() => onRemove(idx)}
                    title="Eliminar imagen"
                    aria-label="Eliminar imagen"
                >
                    <X size={14} />
                </Button>
            </div>
        </div>
    );
};

export const GaleriaEditor: React.FC<GaleriaEditorProps> = ({ galeria, onChange }) => {
    const [subiendo, setSubiendo] = useState(false);
    const inputRef = useRef<HTMLInputElement>(null);

    const sensors = useSensors(
        useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
    );

    /* IDs estables para dnd-kit basados en url+index */
    const itemIds = galeria.map((img, idx) => `gal-${idx}-${img.url}`);

    const handleDragEnd = useCallback((event: DragEndEvent) => {
        const { active, over } = event;
        if (!over || active.id === over.id) return;
        const oldIndex = itemIds.indexOf(active.id as string);
        const newIndex = itemIds.indexOf(over.id as string);
        if (oldIndex !== -1 && newIndex !== -1) {
            onChange(arrayMove([...galeria], oldIndex, newIndex));
        }
    }, [galeria, onChange, itemIds]);

    const handleUpload = useCallback(async (files: FileList | null) => {
        if (!files || files.length === 0) return;
        setSubiendo(true);
        const nuevas: GaleriaImagen[] = [];
        for (const file of Array.from(files)) {
            if (!file.type.startsWith('image/')) continue;
            try {
                const res = await apiUploadImage(file);
                nuevas.push({ url: res.url, layout: 'full' });
            } catch { /* error manejado por toast futuro */ }
        }
        if (nuevas.length > 0) {
            onChange([...galeria, ...nuevas]);
        }
        setSubiendo(false);
        if (inputRef.current) inputRef.current.value = '';
    }, [galeria, onChange]);

    const handleRemove = useCallback((idx: number) => {
        onChange(galeria.filter((_, i) => i !== idx));
    }, [galeria, onChange]);

    /* [124A-PROJ1] Alternar layout full ↔ half */
    const handleToggleLayout = useCallback((idx: number) => {
        const copia = [...galeria];
        copia[idx] = { ...copia[idx], layout: copia[idx].layout === 'full' ? 'half' : 'full' };
        onChange(copia);
    }, [galeria, onChange]);

    return (
        <div className="editorProyectoGaleria">
            <span className="editorProyectoLabel">Galería</span>

            {galeria.length > 0 && (
                <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                    <SortableContext items={itemIds} strategy={rectSortingStrategy}>
                        <div className="editorProyectoGaleriaGrid">
                            {galeria.map((img, idx) => (
                                <GaleriaItemSortable
                                    key={itemIds[idx]}
                                    id={itemIds[idx]}
                                    img={img}
                                    idx={idx}
                                    onToggleLayout={handleToggleLayout}
                                    onRemove={handleRemove}
                                />
                            ))}
                        </div>
                    </SortableContext>
                </DndContext>
            )}

            <input
                ref={inputRef}
                type="file"
                accept="image/*"
                multiple
                className="editorProyectoGaleriaFileInput"
                onChange={e => handleUpload(e.target.files)}
            />
            <Button
                variante="texto"
                tamano="pequeno"
                onClick={() => inputRef.current?.click()}
                disabled={subiendo}
            >
                {subiendo ? 'Subiendo...' : '+ Añadir imágenes'}
            </Button>
        </div>
    );
};
