/* [SEO-A] Sección SEO del panel admin — auditoría de páginas públicas.
 * Sub-tabs: Resumen | Páginas | Blog | GEO.
 * Reutiliza clases CSS de SeccionContenido para layout base. */
import React, {useState, useEffect} from 'react';
import {BarChart3, FileText, PenTool, Globe} from 'lucide-react';
import {useQuery} from '@tanstack/react-query';
import {Button} from '../ui/Button';
import {apiGetSeoAudit, type SeoAuditResponse} from '../../api/admin-seo';
import {SubTabSeoResumen} from './SubTabSeoResumen';
import {SubTabSeoPaginas} from './SubTabSeoPaginas';
import {SubTabSeoBlog} from './SubTabSeoBlog';
import {SubTabSeoGeo} from './SubTabSeoGeo';
import './SeccionSeo.css';

type SubTab = 'resumen' | 'paginas' | 'blog' | 'geo';

interface SubTabConfig {
    id: SubTab;
    label: string;
    icono: React.ElementType;
}

const SUB_TABS: SubTabConfig[] = [
    {id: 'resumen', label: 'Resumen', icono: BarChart3},
    {id: 'paginas', label: 'Páginas', icono: FileText},
    {id: 'blog', label: 'Blog', icono: PenTool},
    {id: 'geo', label: 'GEO', icono: Globe},
];

const SEO_SUBTAB_KEY = 'panel-seo-subtab';
const VALID_SUBTABS: SubTab[] = SUB_TABS.map(t => t.id);

export const SeccionSeo: React.FC = () => {
    const [subTab, setSubTab] = useState<SubTab>(() => {
        const stored = localStorage.getItem(SEO_SUBTAB_KEY) as SubTab | null;
        if (stored && VALID_SUBTABS.includes(stored)) return stored;
        return 'resumen';
    });

    useEffect(() => {
        localStorage.setItem(SEO_SUBTAB_KEY, subTab);
    }, [subTab]);

    const {data, isLoading, error} = useQuery<SeoAuditResponse>({
        queryKey: ['admin-seo-audit'],
        queryFn: apiGetSeoAudit,
        staleTime: 5 * 60 * 1000,
    });

    if (isLoading) {
        return (
            <div className="contenidoContenedor">
                <div className="seoCargando">Cargando auditoría SEO...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="contenidoContenedor">
                <div className="contenidoError">Error al cargar auditoría SEO: {(error as Error).message}</div>
            </div>
        );
    }

    if (!data) return null;

    return (
        <div className="contenidoContenedor">
            <div className="contenidoSubTabs">
                {SUB_TABS.map(tab => {
                    const Icono = tab.icono;
                    return (
                        <Button
                            key={tab.id}
                            type="button"
                            variante="texto"
                            className={`contenidoSubTab ${subTab === tab.id ? 'contenidoSubTab--activo' : ''}`}
                            onClick={() => setSubTab(tab.id)}
                        >
                            <Icono size={16} />
                            {tab.label}
                        </Button>
                    );
                })}
            </div>

            <div className="contenidoPanel">
                {subTab === 'resumen' && <SubTabSeoResumen summary={data.summary} pages={data.pages} />}
                {subTab === 'paginas' && <SubTabSeoPaginas pages={data.pages} />}
                {subTab === 'blog' && <SubTabSeoBlog posts={data.blog_posts} />}
                {subTab === 'geo' && <SubTabSeoGeo checks={data.geo_checks} />}
            </div>
        </div>
    );
};
