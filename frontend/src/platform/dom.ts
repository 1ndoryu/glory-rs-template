/* [259A-5] Adapter DOM: unico punto de contacto con document fuera de React.
 * Boundary declarado para la regla sentinel dom-access-outside-platform.
 * Solo accesos imperativos sin equivalente React (scroll lock, portal, scroll
 * a ancla, creacion de nodos para canvas/descargas, raiz del bootstrap). */

import {anchuraVentana} from './viewport';

export function nodoCuerpoDocumento(): HTMLElement {
    return document.body;
}

export function obtenerElementoPorId(id: string): HTMLElement | null {
    return document.getElementById(id);
}

export function desplazarHastaElemento(id: string, comportamiento: ScrollBehavior = 'smooth'): void {
    obtenerElementoPorId(id)?.scrollIntoView({behavior: comportamiento});
}

export function crearElemento<K extends keyof HTMLElementTagNameMap>(etiqueta: K): HTMLElementTagNameMap[K] {
    return document.createElement(etiqueta);
}

/* Bloquea el scroll de la pagina compensando el ancho del scrollbar para
 * evitar layout shift. Devuelve el ancho compensado. Par de
 * desbloquearDesplazamientoPagina: llamar siempre en el cleanup. */
export function bloquearDesplazamientoPagina(): number {
    const anchoBarra = anchuraVentana() - document.documentElement.clientWidth;
    document.body.style.overflow = 'hidden';
    if (anchoBarra > 0) {
        document.body.style.paddingRight = `${anchoBarra}px`;
    }
    return anchoBarra;
}

export function desbloquearDesplazamientoPagina(): void {
    document.body.style.overflow = '';
    document.body.style.paddingRight = '';
}
