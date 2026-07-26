/* [SEO-A] Sub-tab Páginas: tabla con estado SEO de todas las páginas públicas.
 * Reutiliza patrón visual de SeccionCorreo (tabla + filtro + badges). */
import React, {useState} from 'react';
import {Filter} from 'lucide-react';
import type {SeoPageEntry} from '../../api/admin-seo';
import {Select} from '../ui/Select';

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

export const SubTabSeoPaginas: React.FC<Props> = ({pages}) => {
    const [filtro, setFiltro] = useState('');

    const filtered = filtro
        ? pages.filter(p => p.page_type === filtro)
        : pages;

    return (
        <>
            <div className="correosFiltro">
                <Filter size={18} />
                <Select value={filtro} onChange={e => setFiltro(e.target.value)}>
                    <option value="">Todas las páginas</option>
                    <option value="static">Estáticas</option>
                    <option value="service">Servicios</option>
                    <option value="project">Proyectos</option>
                </Select>
                <span className="correosTotal">{filtered.length} páginas</span>
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
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>
        </>
    );
};
