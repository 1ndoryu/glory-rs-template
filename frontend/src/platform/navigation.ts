/* [259A-5] Adapter de navegacion: unico punto de contacto con window.location/history.
 * La regla sentinel window-reference-outside-platform exige que el acceso a
 * window viva en un boundary (/platform/, /adapters/, /navigation/).
 * Los consumidores usan estas funciones y nunca tocan window directamente. */

export function obtenerHref(): string {
    return window.location.href;
}

export function obtenerOrigen(): string {
    return window.location.origin;
}

export function obtenerHost(): string {
    return window.location.host;
}

export function obtenerProtocolo(): string {
    return window.location.protocol;
}

export function obtenerRuta(): string {
    return window.location.pathname;
}

export function obtenerFragmento(): string {
    return window.location.hash;
}

export function leerParametrosBusqueda(): URLSearchParams {
    return new URLSearchParams(window.location.search);
}

export function reemplazarUrl(url: string): void {
    window.history.replaceState({}, '', url);
}

export function redirigir(url: string): void {
    window.location.href = url;
}

export function protocoloSocket(): 'wss:' | 'ws:' {
    return window.location.protocol === 'https:' ? 'wss:' : 'ws:';
}
