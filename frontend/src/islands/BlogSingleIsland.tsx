/**
 * Island: BlogSingleIsland
 * Página individual de un post del blog.
 * [074A-11] Conectado a API pública /api/blog/:slug con fallback a datos estáticos.
 * Lógica de datos extraída a useBlogSingle. */
import {useTranslation} from 'react-i18next';
import DOMPurify from 'dompurify';
import {spaClick} from '../navegacionSPA';
import {LayoutPagina} from '../components/layout/LayoutPagina';
import {SEOHead} from '../components/seo/SEOHead';
import {blogPostSchema} from '../components/seo/schemas';
import {SeccionContacto} from '../components/home/SeccionContacto';
import {POSTS_BLOG} from '../data/blog';
import {obtenerImagenBlog} from '../hooks/useImagenes';
import {useBlogSingle} from '../hooks/useBlogSingle';
import OptimizedImage from '../components/ui/OptimizedImage';
import './BlogSingleIsland.css';

interface BlogSingleIslandProps {
    slug?: string;
    titulo?: string;
    contenido?: string;
    fecha?: string;
    categoria?: string;
    imagen?: string;
}

export const BlogSingleIsland = ({
    slug = '',
    titulo: tituloProp,
    contenido: contenidoProp,
    fecha: fechaProp,
    categoria: categoriaProp,
    imagen: imagenProp
}: BlogSingleIslandProps): JSX.Element => {
    const {t} = useTranslation();

    /* [074A-11] Datos del post: API > props > fallback estático */
    const {titulo, contenido, fecha, fechaModificacion, categoria, imagen} = useBlogSingle({
        slug,
        titulo: tituloProp,
        contenido: contenidoProp,
        fecha: fechaProp,
        categoria: categoriaProp,
        imagen: imagenProp,
    });

    return (
        <LayoutPagina className="blogSingleMain" id="paginaBlogSingle">
            <SEOHead
                title={titulo}
                description={contenido.substring(0, 160)}
                path={`/blog/${slug || ''}`}
                type="article"
                jsonLd={blogPostSchema(titulo, contenido.substring(0, 160), slug || '', fecha, fechaModificacion)}
            />
            {/* Hero del articulo */}
            <section className="blogSingleHero">
                <div className="blogSingleHeroContenido">
                    <div className="blogSingleMeta">
                        {categoria && <span className="blogSingleCategoria">{categoria}</span>}
                        {fecha && (
                            <>
                                <span className="blogSingleSeparador">•</span>
                                <span className="blogSingleFecha">{fecha}</span>
                            </>
                        )}
                    </div>
                    <h1 className="blogSingleTitulo">{titulo}</h1>
                </div>
            </section>

            {/* Imagen destacada */}
            <section className="blogSingleImagenSeccion">
                <div className="blogSingleImagenWrapper">
                    <OptimizedImage src={imagen} alt={titulo} className="blogSingleImagen" width={1200} height={675} sizes="100vw" loading="eager" />
                </div>
            </section>

            {/* Contenido del articulo */}
            <section className="blogSingleContenido">
                <article className="blogSingleArticulo">
                    {contenido ? (
                        <div
                            className="blogSingleTexto"
                            dangerouslySetInnerHTML={{__html: DOMPurify.sanitize(contenido)}}
                        />
                    ) : (
                        <div className="blogSingleTexto">
                            <p>{t('blog_single.placeholder_1')}</p>
                            <p>{t('blog_single.placeholder_2')}</p>
                        </div>
                    )}
                </article>
            </section>

            {/* Articulos relacionados — solo si hay otros posts */}
            {POSTS_BLOG.filter(p => {
                const postSlug = p.link?.split('/').filter(Boolean).pop() || '';
                return postSlug !== slug;
            }).length > 0 && (
                <section className="blogSingleRelacionados">
                    <div className="blogSingleRelacionadosContenedor">
                        <h2 className="blogSingleRelacionadosTitulo">{t('blog_single.more_articles')}</h2>
                        <div className="blogSingleRelacionadosGrid">
                            {POSTS_BLOG.filter(p => {
                                const postSlug = p.link?.split('/').filter(Boolean).pop() || '';
                                return postSlug !== slug;
                            }).slice(0, 2).map(post => (
                                <a key={post.id} href={post.link} className="blogSingleRelacionadoCard" onClick={e => { if (post.link) spaClick(e, post.link); }}>
                                    <div className="blogSingleRelacionadoImagen">
                                        <OptimizedImage src={post.imagen || obtenerImagenBlog(post.id)} alt={post.titulo} width={480} height={640} sizes="(max-width: 768px) 100vw, 50vw" />
                                    </div>
                                    <div className="blogSingleRelacionadoInfo">
                                        <span className="blogSingleRelacionadoCategoria">{post.categoria}</span>
                                        <h3 className="blogSingleRelacionadoNombre">{post.titulo}</h3>
                                    </div>
                                </a>
                            ))}
                        </div>
                    </div>
                </section>
            )}

            <SeccionContacto />
        </LayoutPagina>
    );
};

export default BlogSingleIsland;
