/**
 * Planes de precios: indice central (barril puro: solo re-exports).
 * [044A-32] Todos los planes centralizados en planes.ts para edicion rapida.
 * [higiene] La resolucion por slug (PLANES_POR_SERVICIO, obtenerPlanesServicio)
 * vive en ./servicio: mezclar barril y logica lo marcaba mixed-barrel-logic.
 */
export type {CaracteristicaPlan, PlanServicio, PlanesDeServicio} from './tipos';
export {incluida, noIncluida} from './tipos';

export {PLANES_WEB, PLANES_APPS, PLANES_IA, PLANES_BRANDING, PLANES_ECOMMERCE, PLANES_SEO, PLANES_MARKETING} from './planes';

export {PLANES_POR_SERVICIO, obtenerPlanesServicio} from './servicio';
