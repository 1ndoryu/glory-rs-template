/* [074A-11] Hook para cargar un blog post desde la API pública.
 * Prioridad: props > API > datos estáticos locales.
 * Extraído de BlogSingleIsland para cumplir regla max 3 useState. */
import {useState, useEffect} from 'react';
import {useTranslation} from 'react-i18next';
import {apiGetBlogPostBySlug} from '../api/admin-blog';
import {POSTS_BLOG} from '../data/blog';
import {obtenerImagenBlog} from './useImagenes';

interface BlogSingleData {
    titulo: string;
    contenido: string;
    fecha: string;
    fechaModificacion?: string;
    categoria: string;
    imagen: string;
}

interface UseBlogSingleProps {
    slug: string;
    titulo?: string;
    contenido?: string;
    fecha?: string;
    categoria?: string;
    imagen?: string;
}

export function useBlogSingle({slug, titulo: tituloProp, contenido: contenidoProp, fecha: fechaProp, categoria: categoriaProp, imagen: imagenProp}: UseBlogSingleProps): BlogSingleData {
    const {t} = useTranslation();
    const [apiData, setApiData] = useState<{
        titulo: string; contenido: string; fecha: string; fechaModificacion?: string; categoria: string; imagen: string;
    } | null>(null);

    /* Fetch del post desde la API */
    useEffect(() => {
        if (!slug) return;
        const controller = new AbortController();
        apiGetBlogPostBySlug(slug)
            .then(post => {
                if (!controller.signal.aborted) {
                    setApiData({
                        titulo: post.title,
                        contenido: post.content,
                        fecha: new Date(post.published_at ?? post.created_at)
                            .toLocaleDateString('es', {day: 'numeric', month: 'short', year: 'numeric'}),
                        /* [277A-9] dateModified para Google: usa updated_at si existe, si no published_at */
                        fechaModificacion: post.updated_at
                            ? new Date(post.updated_at).toISOString().split('T')[0]
                            : undefined,
                        categoria: post.tags[0] ?? 'General',
                        imagen: post.featured_image ?? '',
                    });
                }
            })
            .catch(() => {
                /* Fallback silencioso a datos estáticos */
            });
        return () => controller.abort();
    }, [slug]);

    /* Prioridad: props > API > datos estáticos locales */
    let titulo = tituloProp || apiData?.titulo || t('blog_single.default_title');
    let contenido = contenidoProp || apiData?.contenido || '';
    let fecha = fechaProp || apiData?.fecha || '';
    let categoria = categoriaProp || apiData?.categoria || '';
    let imagen = imagenProp || apiData?.imagen || '';

    /* Fallback final a datos estáticos si no hay props ni API */
    if (!contenidoProp && !apiData && slug) {
        const postLocal = POSTS_BLOG.find(p => {
            const postSlug = p.link?.split('/').filter(Boolean).pop() || '';
            return postSlug === slug;
        });

        if (postLocal) {
            titulo = postLocal.titulo;
            contenido = postLocal.contenido || postLocal.resumen || '';
            fecha = postLocal.fecha;
            categoria = postLocal.categoria;
            imagen = postLocal.imagen || obtenerImagenBlog(postLocal.id);
        }
    }

    if (!imagen) imagen = obtenerImagenBlog(1);

    return {titulo, contenido, fecha, fechaModificacion: apiData?.fechaModificacion, categoria, imagen};
}
