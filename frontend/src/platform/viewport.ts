/* [259A-5] Adapter de viewport: unico punto de contacto con dimensiones de
 * ventana, scroll programatico y listeners de window. Boundary declarado para
 * la regla sentinel window-reference-outside-platform. */

export function anchuraVentana(): number {
    return window.innerWidth;
}

export function alturaVentana(): number {
    return window.innerHeight;
}

export function irArriba(): void {
    window.scrollTo(0, 0);
}

type ManejadorVentana = EventListenerOrEventListenerObject;
type OpcionesVentana = AddEventListenerOptions | boolean;

export function suscribirVentana(tipo: string, manejador: ManejadorVentana, opciones?: OpcionesVentana): void {
    window.addEventListener(tipo, manejador, opciones);
}

export function desuscribirVentana(tipo: string, manejador: ManejadorVentana, opciones?: OpcionesVentana): void {
    window.removeEventListener(tipo, manejador, opciones);
}
