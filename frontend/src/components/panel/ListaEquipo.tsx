/* [074A-13] Lista de miembros del equipo para el panel admin CMS.
 * [114A-7] Menú 3 puntos con archivar/desarchivar/eliminar.
 * [124A-CMS7] Convertido de grid a lista vertical, mismo patrón que ListaServicios/ListaBlog. */

import React, {useState} from 'react';
import {Archive, ArchiveRestore, Trash2, Globe, Search} from 'lucide-react';
import {AdminTeamMember} from '../../api/admin-team';
import {Badge} from '../ui/Badge';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import OptimizedImage from '../ui/OptimizedImage';
import {MenuContextual, type MenuContextualItem} from '../ui/ContextMenu';
import {useBusquedaLista} from '../../hooks/useBusquedaLista';
import './ListaEquipo.css';

interface ListaEquipoProps {
    miembros: AdminTeamMember[];
    cargando: boolean;
    onEditar: (m: AdminTeamMember) => void;
    onCrear: () => void;
    onArchivar: (id: string) => void;
    onDesarchivar?: (id: string) => void;
    onEliminar?: (id: string) => void;
    onPublicar?: (id: string) => void;
}

export const ListaEquipo: React.FC<ListaEquipoProps> = ({miembros, cargando, onEditar, onCrear, onArchivar, onDesarchivar, onEliminar, onPublicar}) => {
    const [menuActivo, setMenuActivo] = useState<string | null>(null);
    /* [124A-SEARCH1] Búsqueda en tiempo real */
    const { busqueda, setBusqueda, filtrados } = useBusquedaLista(
        miembros, ['name', 'role']
    );

    if (cargando) return <p className="equipoListaCargando">Cargando miembros...</p>;

    return (
        <div className="equipoListaContenedor">
            <div className="equipoListaAcciones">
                <div className="listaBusqueda">
                    <Search size={14} className="listaBusquedaIcono" />
                    <Input
                        value={busqueda}
                        onChange={e => setBusqueda(e.target.value)}
                        placeholder="Buscar miembros..."
                        className="listaBusquedaInput"
                    />
                </div>
                <Button variante="primario" tamano="pequeno" onClick={onCrear}>+ Nuevo miembro</Button>
            </div>
            {filtrados.length === 0 && <p className="equipoListaVacia">No hay miembros del equipo.</p>}
            <div className="equipoListaLista">
            {filtrados.map(m => {
                const items: MenuContextualItem[] = [];
                /* [084A-10] Publicar: solo si no está ya publicado */
                if (m.status !== 'published' && onPublicar) {
                    items.push({id: 'publicar', label: 'Publicar', icon: <Globe size={14} />, onSelect: () => onPublicar(m.id)});
                }
                if (m.status !== 'archived') {
                    items.push({id: 'archivar', label: 'Archivar', icon: <Archive size={14} />, onSelect: () => onArchivar(m.id)});
                }
                if (m.status === 'archived' && onDesarchivar) {
                    items.push({id: 'desarchivar', label: 'Desarchivar', icon: <ArchiveRestore size={14} />, onSelect: () => onDesarchivar(m.id)});
                }
                if (onEliminar) {
                    items.push({id: 'eliminar', label: 'Eliminar', icon: <Trash2 size={14} />, danger: true, onSelect: () => onEliminar(m.id)});
                }

                return (
                    <article key={m.id} className="equipoListaFila">
                        <div className="equipoListaAvatar">
                            {m.avatar && <OptimizedImage src={m.avatar} alt={m.name} loading="lazy" />}
                        </div>

                        <div
                            className="equipoListaFilaInfo"
                            onClick={() => onEditar(m)}
                            role="button"
                            tabIndex={0}
                            onKeyDown={e => { if (e.key === 'Enter') onEditar(m); }}
                        >
                            <span className="equipoListaNombre">{m.name}</span>
                            <span className="equipoListaCargo">{m.role}</span>
                            <Badge label={m.status} />
                            <span className="equipoListaOrden">#{m.sort_order}</span>
                        </div>

                        <div className="equipoListaMenu" onClick={e => e.stopPropagation()}>
                            <MenuContextual
                                abierto={menuActivo === m.id}
                                onToggle={() => setMenuActivo(prev => prev === m.id ? null : m.id)}
                                onCerrar={() => setMenuActivo(null)}
                                ariaLabel="Acciones del miembro"
                                items={items}
                            />
                        </div>
                    </article>
                );
            })}
            </div>
        </div>
    );
};
