/**
 * Componente: ProyectoIndividualIsland
 * [124A-PROJ1] Rediseño estilo kontrapunkt: Hero → Portada → Case Intro → Galería → Relacionados → Contacto
 * Eliminados: Skills, CTA, carrusel infinito.
 */
import {useState, useEffect} from 'react';
import '../styles/variables.css';
import './ProyectoIndividualIsland.css';
import {LayoutPagina} from '../components/layout/LayoutPagina';
import {SEOHead} from '../components/seo/SEOHead';
import {breadcrumbSchema} from '../components/seo/schemas';
import {SeccionHeroProyecto} from '../components/proyectos/SeccionHeroProyecto';
import {SeccionGaleriaProyecto} from '../components/proyectos/SeccionGaleriaProyecto';
import {SeccionProyectosRelacionados} from '../components/proyectos/SeccionProyectosRelacionados';
import {SeccionContacto} from '../components/home/SeccionContacto';
import type {EnlaceProyecto, GaleriaImagen} from '../types/contenido';
import {apiGetProjectBySlug, type AdminProject} from '../api/admin-projects';
import {NotFoundIsland} from './NotFoundIsland';

interface ProyectoIndividualIslandProps {
    titulo?: string;
    descripcion?: string;
    cliente?: string;
    categorias?: string;
    imagen?: string;
    slug?: string;
}

/* [124A-PROJ1] Datos del proyecto para renderizar */
interface ProyectoDetalle {
    titulo: string;
    descripcion: string;
    cliente: string;
    categorias: string;
    imagenPortada: string;
    galeria: GaleriaImagen[];
    tecnologias: string[];
    enlaces: EnlaceProyecto[];
}

/* [074A-12] Convierte AdminProject (API) → ProyectoDetalle
 * [124A-DETAIL1] Usa detail_title si existe, y primera imagen de galería si use_first_gallery_image */
function convertirDesdeApi(p: AdminProject): ProyectoDetalle {
    const usarPrimeraGaleria = p.use_first_gallery_image && p.gallery.length > 0;
    /* [174A-2] Si la primera imagen de galería se usa como portada, quitarla de la galería
     * para evitar que aparezca duplicada en la sección de imágenes del detalle. */
    const galeriaSinPortada = usarPrimeraGaleria ? p.gallery.slice(1) : p.gallery;
    return {
        titulo: p.detail_title || p.title,
        descripcion: p.description,
        cliente: p.client || '',
        categorias: p.categories.join(', '),
        imagenPortada: usarPrimeraGaleria ? p.gallery[0].url : (p.featured_image || ''),
        galeria: galeriaSinPortada,
        tecnologias: p.technologies,
        enlaces: p.links.map(l => ({tipo: l.tipo as EnlaceProyecto['tipo'], url: l.url, etiqueta: l.etiqueta})),
    };
}

export const ProyectoIndividualIsland = ({titulo = 'Proyecto', descripcion = '', cliente = '', categorias = '', imagen = '', slug = ''}: ProyectoIndividualIslandProps): JSX.Element => {
    const [detalle, setDetalle] = useState<ProyectoDetalle | null>(() => (
        slug
            ? null
            : {
                titulo,
                descripcion,
                cliente,
                categorias,
                imagenPortada: imagen,
                galeria: [],
                tecnologias: [],
                enlaces: [],
            }
    ));
    const [apiError, setApiError] = useState(false);

    useEffect(() => {
        const controller = new AbortController();
        if (slug) {
            /* [155A-16] No mostrar PROYECTOS_DATA legacy mientras llega CMS.
             * Ese fallback producía un flash de contenido viejo antes de hidratar el detalle real. */
            setDetalle(null);
            setApiError(false);
            apiGetProjectBySlug(slug)
                .then(data => {
                    if (!controller.signal.aborted) {
                        setDetalle(convertirDesdeApi(data));
                    }
                })
                .catch(() => {
                    if (!controller.signal.aborted) {
                        setApiError(true);
                    }
                });
        }
        return () => controller.abort();
    }, [slug]);

    if (slug && apiError) {
        return <NotFoundIsland />;
    }

    if (!detalle) {
        return (
            <LayoutPagina className="proyectoIndividualMain" id="paginaProyecto">
                <p>Cargando...</p>
            </LayoutPagina>
        );
    }

    return (
        <LayoutPagina className="proyectoIndividualMain" id="paginaProyecto">
            <SEOHead
                title={detalle.titulo}
                description={detalle.descripcion}
                path={`/proyectos/${slug || ''}`}
                jsonLd={breadcrumbSchema([
                    {name: 'Inicio', url: '/'},
                    {name: 'Proyectos', url: '/proyectos'},
                    {name: detalle.titulo, url: `/proyectos/${slug}`},
                ])}
            />

            {/* Hero + Portada + Case Introduction (integrados en un componente) */}
            <SeccionHeroProyecto
                titulo={detalle.titulo}
                descripcion={detalle.descripcion}
                cliente={detalle.cliente}
                categorias={detalle.categorias}
                tecnologias={detalle.tecnologias}
                enlaces={detalle.enlaces}
                imagenPortada={detalle.imagenPortada}
            />

            {/* Galería con layout alternante full/half */}
            <SeccionGaleriaProyecto imagenes={detalle.galeria} />

            {/* Proyectos relacionados (grid 4 columnas) */}
            <SeccionProyectosRelacionados
                slugActual={slug}
                tituloActual={detalle.titulo}
                categorias={detalle.categorias}
            />

            <SeccionContacto />
        </LayoutPagina>
    );
};

export default ProyectoIndividualIsland;
