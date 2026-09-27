/* [074A-11] Hook interno para el estado del formulario de EditorBlog.
 * Extrae los useState del componente para cumplir regla max 3 useState.
 * Patrón: misma estructura que useEditorServicio.ts. */
import {useState, useCallback, useEffect} from 'react';
import type {AdminBlogPost, CreateBlogPostBody, UpdateBlogPostBody} from '../api/admin-blog';

/* [259A-5] Estado segregado por faceta (ISP, regla large-interface-isp).
 * Mismo patron que useEditorServicio: el hook sigue devolviendo
 * EditorBlogState plano via extends, sin cambios en consumidores. */
export interface EditorBlogContenido {
    titulo: string;
    slug: string;
    extracto: string;
    contenido: string;
    imagenUrl: string;
}

export interface EditorBlogPublicacion {
    status: string;
    tags: string[];
    metaTitle: string;
    metaDescription: string;
    isFeatured: boolean;
}

export interface EditorBlogAccionesContenido {
    setTitulo: (v: string) => void;
    setSlug: (v: string) => void;
    setExtracto: (v: string) => void;
    setContenido: (v: string) => void;
    setImagenUrl: (v: string) => void;
}

export interface EditorBlogAccionesPublicacion {
    setStatus: (v: string) => void;
    setTags: (v: string[]) => void;
    setMetaTitle: (v: string) => void;
    setMetaDescription: (v: string) => void;
    setIsFeatured: (v: boolean) => void;
}

export interface EditorBlogOperaciones {
    buildBody: () => CreateBlogPostBody | UpdateBlogPostBody;
    resetear: () => void;
}

export interface EditorBlogState extends EditorBlogContenido, EditorBlogPublicacion, EditorBlogAccionesContenido, EditorBlogAccionesPublicacion, EditorBlogOperaciones {
}

export function useEditorBlog(post: AdminBlogPost | null, abierto: boolean): EditorBlogState {
    const [titulo, setTitulo] = useState('');
    const [slug, setSlug] = useState('');
    const [extracto, setExtracto] = useState('');
    const [contenido, setContenido] = useState('');
    const [imagenUrl, setImagenUrl] = useState('');
    const [status, setStatus] = useState('draft');
    const [tags, setTags] = useState<string[]>([]);
    const [metaTitle, setMetaTitle] = useState('');
    const [metaDescription, setMetaDescription] = useState('');
    /* [124A-PROJ] is_featured: controla si el post aparece destacado en el inicio */
    const [isFeatured, setIsFeatured] = useState(false);

    const resetear = useCallback(() => {
        setTitulo('');
        setSlug('');
        setExtracto('');
        setContenido('');
        setImagenUrl('');
        setStatus('draft');
        setTags([]);
        setMetaTitle('');
        setMetaDescription('');
        setIsFeatured(false);
    }, []);

    /* Sincronizar con post seleccionado */
    useEffect(() => {
        if (post) {
            setTitulo(post.title);
            setSlug(post.slug);
            setExtracto(post.excerpt ?? '');
            setContenido(post.content);
            setImagenUrl(post.featured_image ?? '');
            setStatus(post.status);
            setTags(post.tags);
            setMetaTitle(post.meta_title ?? '');
            setMetaDescription(post.meta_description ?? '');
            setIsFeatured(post.is_featured ?? false);
        } else {
            resetear();
        }
    }, [post, abierto, resetear]);

    const buildBody = useCallback((): CreateBlogPostBody | UpdateBlogPostBody => ({
        title: titulo,
        slug,
        excerpt: extracto || undefined,
        content: contenido,
        featured_image: imagenUrl || undefined,
        status,
        tags: tags.length > 0 ? tags : undefined,
        meta_title: metaTitle || undefined,
        meta_description: metaDescription || undefined,
        is_featured: isFeatured,
    }), [titulo, slug, extracto, contenido, imagenUrl, status, tags, metaTitle, metaDescription, isFeatured]);

    return {
        titulo, slug, extracto, contenido, imagenUrl, status, tags,
        metaTitle, metaDescription, isFeatured,
        setTitulo, setSlug, setExtracto, setContenido, setImagenUrl,
        setStatus, setTags, setMetaTitle, setMetaDescription, setIsFeatured,
        buildBody, resetear,
    };
}
