/* [270A-1] Sub-tab Páginas: tabla con estado SEO de todas las páginas públicas.
 * Reutiliza patrón visual de SeccionCorreo (tabla + badges).
 * Búsqueda por texto + filtro por tipo de página.
 * [277A-13] Columna acciones: ✏️ editar (estáticas), 🔗 ir al CMS (dinámicas). */
import React, {useState} from 'react';
import {Search, ChevronDown, Pencil, ExternalLink} from 'lucide-react';
import {useQuery} from '@tanstack/react-query';
import type {SeoPageEntry, SeoSetting} from '../../api/admin-seo';
import {apiGetSeoSettings} from '../../api/admin-seo';
import {MenuContextual} from '../ui/ContextMenu';
import {ModalSeoEdit} from './ModalSeoEdit';

interface Props {
    pages: SeoPageEntry[];
}

const STATUS_LABELS: Record<string, string> = {
    ok: 'OK',
    warning: 'Mejorable',
    error: 'Problema',
};

const TYPE_LABELS: Record<string, string> = {
    static: 'Estática',
    service: 'Servicio',
    project: 'Proyecto',
};

const TYPE_OPTIONS = [
    {id: '', label: 'Todas las páginas'},
    {id: 'static', label: 'Estáticas'},
    {id: 'service', label: 'Servicios'},
    {id: 'project', label: 'Proyectos'},
] as const;

/* Mapa de rutas dinámicas → secciones del panel */
const DYNAMIC_CMS_ROUTES: Record<string, string> = {
    service: '/panel?seccion=servicios',
    project: '/panel?seccion=proyectos',
};

export const SubTabSeoPaginas: React.FC<Props> = ({pages}) => {
    const [busqueda, setBusqueda] = useState('');
    const [filtroTipo, setFiltroTipo] = useState('');
    const [menuAbierto, setMenuAbierto] = useState(false);
    const [editingSetting, setEditingSetting] = useState<SeoSetting | null>(null);

    /* [277A-13] Cargar SEO settings de la DB para el modal de edición */
    const {data: seoSettings} = useQuery<SeoSetting[]>({
        queryKey: ['admin-seo-settings'],
        queryFn: apiGetSeoSettings,
        staleTime: 5 * 60 * 1000,
    });

    const settingsMap = new Map<string, SeoSetting>();
    if (seoSettings) {
        for (const s of seoSettings) {
            settingsMap.set(s.path, s);
        }
    }

    const q = busqueda.toLowerCase().trim();
    const filtered = pages.filter(p => {
        if (q && !p.label.toLowerCase().includes(q) &&
            !p.path.toLowerCase().includes(q) &&
            !(p.title && p.title.toLowerCase().includes(q))) {
            return false;
        }
        if (filtroTipo && p.page_type !== filtroTipo) {
            return false;
        }
        return true;
    });

    const handleEdit = (page: SeoPageEntry) => {
        const setting = settingsMap.get(page.path);
        if (setting) {
            setEditingSetting(setting);
        }
    };

    const handleGoToCms = (pageType: string) => {
        const route = DYNAMIC_CMS_ROUTES[pageType];
        if (route) {
            window.location.href = route;
        }
    };

    return (
        <>
            <div className="seoPaginasFiltros">
                <div className="seoPaginasBusqueda">
                    <Search size={16} className="seoPaginasBusquedaIcono" />
                    <input
                        type="text"
                        className="seoPaginasBusquedaInput"
                        placeholder="Buscar página por nombre, ruta o title..."
                        value={busqueda}
                        onChange={e => setBusqueda(e.target.value)}
                    />
                </div>
                <MenuContextual
                    abierto={menuAbierto}
                    onToggle={() => setMenuAbierto(prev => !prev)}
                    onCerrar={() => setMenuAbierto(false)}
                    ariaLabel="Filtrar por tipo de página"
                    triggerClassName="seoPaginasFiltroTipo"
                    triggerVariante="outline"
                    triggerTamano="pequeno"
                    triggerContent={<>{TYPE_LABELS[filtroTipo] ?? 'Todas las páginas'} <ChevronDown size={14} /></>}
                    items={TYPE_OPTIONS.map(opt => ({
                        id: opt.id,
                        label: opt.label,
                        onSelect: () => setFiltroTipo(opt.id),
                    }))}
                />
            </div>

            <div className="correosTablaWrapper">
                <table className="correosTabla">
                    <thead>
                        <tr>
                            <th>Página</th>
                            <th>Title</th>
                            <th>Descripción</th>
                            <th>OG</th>
                            <th>JSON-LD</th>
                            <th>Estado</th>
                            <th>Acciones</th>
                        </tr>
                    </thead>
                    <tbody>
                        {filtered.map(page => (
                            <tr key={page.path} className="correosFila">
                                <td>
                                    <div className="seoPaginaLabel">
                                        <span className="correosTag">{TYPE_LABELS[page.page_type] ?? page.page_type}</span>
                                        <span className="seoPaginaRuta">{page.label}</span>
                                    </div>
                                </td>
                                <td className="seoPaginaTitle">
                                    {page.title || <em>Sin título</em>}
                                    {page.title && <span className="seoPaginaLen">{page.title_len}c</span>}
                                </td>
                                <td className="correosAsunto">
                                    {page.description || <em>Sin descripción</em>}
                                </td>
                                <td>
                                    <span className={`correosBadge ${page.og_image_is_default ? 'badgeNeutral' : 'badgeExito'}`}>
                                        {page.og_image_is_default ? 'Default' : 'Sí'}
                                    </span>
                                </td>
                                <td>
                                    <span className="correosTag">
                                        {page.json_ld_type || '—'}
                                    </span>
                                </td>
                                <td>
                                    <span className={`listaServiciosBadge listaServiciosBadge--${page.status === 'ok' ? 'published' : page.status === 'warning' ? 'draft' : 'archived'}`}>
                                        {STATUS_LABELS[page.status] ?? page.status}
                                    </span>
                                </td>
                                <td>
                                    <div className="seoPaginaAcciones">
                                        {page.page_type === 'static' ? (
                                            <button
                                                type="button"
                                                className="seoPaginaAccionBtn"
                                                onClick={() => handleEdit(page)}
                                                title="Editar SEO"
                                                aria-label={`Editar SEO de ${page.label}`}
                                            >
                                                <Pencil size={14} />
                                            </button>
                                        ) : (
                                            <button
                                                type="button"
                                                className="seoPaginaAccionBtn"
                                                onClick={() => handleGoToCms(page.page_type)}
                                                title="Ir al CMS"
                                                aria-label={`Ir al CMS de ${page.label}`}
                                            >
                                                <ExternalLink size={14} />
                                            </button>
                                        )}
                                    </div>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>

            <ModalSeoEdit
                setting={editingSetting}
                onClose={() => setEditingSetting(null)}
            />
        </>
    );
};
