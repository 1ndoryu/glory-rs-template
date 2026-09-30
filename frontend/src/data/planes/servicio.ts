/*
 * Resolucion de planes por slug de servicio.
 * [higiene] Vive aqui y no en ./index: el indice es barril puro (solo
 * re-exports) y mezclarlo con logica ejecutable lo marcaba mixed-barrel-logic.
 * Se importa via './index' igual que antes (re-exportado abajo del barril).
 */
import {PLANES_WEB, PLANES_APPS, PLANES_IA, PLANES_BRANDING, PLANES_ECOMMERCE, PLANES_SEO, PLANES_MARKETING} from './planes';
import type {PlanesDeServicio} from './tipos';

export const PLANES_POR_SERVICIO: PlanesDeServicio[] = [PLANES_WEB, PLANES_APPS, PLANES_IA, PLANES_BRANDING, PLANES_SEO, PLANES_MARKETING, PLANES_ECOMMERCE];

/*
 * Mapeo de slugs generados por WP (desde titulos) a slugs esperados por los planes.
 * Esto asegura compatibilidad si el slug no se sincronizó correctamente como 'diseno-web'.
 */
const ALIAS_SERVICIOS: Record<string, string> = {
    'diseno-de-sitios-web': 'diseno-web',
    'desarrollo-de-aplicaciones': 'desarrollo-apps',
    'agentes-de-ia': 'agentes-ia',
    'identidad-de-marca': 'branding',
    'e-commerce': 'ecommerce'
};

/*
 * Obtener planes para un servicio especifico por su slug.
 * Busca coincidencia parcial (slug.includes) para flexibilidad con slugs de WP.
 * Retorna null si no se encuentran planes para el slug dado.
 */
export const obtenerPlanesServicio = (slug: string): PlanesDeServicio | null => {
    if (!slug) return null;

    // Normalizar slug si coincide con uno de los alias conocidos
    const slugNormalizado = ALIAS_SERVICIOS[slug] || slug;

    return PLANES_POR_SERVICIO.find(p => slugNormalizado.includes(p.servicioSlug)) || null;
};
