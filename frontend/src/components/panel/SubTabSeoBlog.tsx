/* [SEO-A] Sub-tab Blog: lista de posts con indicador SEO.
 * Reutiliza patrón visual de ListaServicios (filas compactas). */
import React from 'react';
import type {SeoBlogEntry} from '../../api/admin-seo';

interface Props {
    posts: SeoBlogEntry[];
}

const BLOG_STATUS_LABELS: Record<string, string> = {
    published: 'Publicado',
    draft: 'Borrador',
    archived: 'Archivado',
};

const SEO_STATUS_LABELS: Record<string, string> = {
    ok: 'OK',
    warning: 'Mejorable',
    error: 'Incompleto',
};

export const SubTabSeoBlog: React.FC<Props> = ({posts}) => {
    if (posts.length === 0) {
        return <div className="contenidoPlaceholder"><span className="contenidoPlaceholderTexto">No hay posts de blog</span></div>;
    }

    return (
        <div className="listaServiciosLista">
            {posts.map(post => (
                <div key={post.id} className="listaServiciosFila">
                    <div className="listaServiciosFilaInfo">
                        <span className="listaServiciosNombre">{post.title}</span>
                        <span className={`listaBlogBadge listaBlogBadge--${post.status}`}>
                            {BLOG_STATUS_LABELS[post.status] ?? post.status}
                        </span>
                        <span className={`listaServiciosBadge listaServiciosBadge--${post.seo_status === 'ok' ? 'published' : 'draft'}`}>
                            SEO: {SEO_STATUS_LABELS[post.seo_status] ?? post.seo_status}
                        </span>
                        <span className="listaServiciosPrecio">/blog/{post.slug}</span>
                    </div>
                    <div className="seoBlogMeta">
                        {post.meta_title
                            ? <span>{post.meta_title}</span>
                            : <em className="seoBlogMetaVacio">Sin meta title</em>}
                        {' · '}
                        {post.meta_description
                            ? <span>{post.meta_description.substring(0, 80)}{post.meta_description.length > 80 ? '...' : ''}</span>
                            : <em className="seoBlogMetaVacio">Sin meta description</em>}
                    </div>
                </div>
            ))}
        </div>
    );
};
