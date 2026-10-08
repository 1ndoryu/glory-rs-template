/* Adaptador de plataforma (window): temporizadores y aviso de cierre.
 * Cualquier acceso directo a `window` vive aquí, nunca en hooks/componentes.
 * Sentinel (window-reference-outside-platform). [08AA-22] `nombreHost` y
 * `repetirCada` absorben los dos últimos usos directos fuera de aquí. */

/* Ejecuta `fn` tras `ms`; devuelve función para cancelarlo (debounce). */
export function temporizar(ms: number, fn: () => void): () => void {
  const t = window.setTimeout(fn, ms);
  return () => window.clearTimeout(t);
}

/* Aviso nativo al recargar/cerrar la pestaña mientras `debeAvisar()` sea
 * verdad; devuelve función para retirar el aviso. */
export function avisarAlCerrar(debeAvisar: () => boolean): () => void {
  const handler = (e: BeforeUnloadEvent) => {
    if (debeAvisar()) e.preventDefault();
  };
  window.addEventListener('beforeunload', handler);
  return () => window.removeEventListener('beforeunload', handler);
}

/* Ruta actual sin query (decide pública vs admin). */
export function rutaActual(): string {
  return window.location.pathname;
}

/* Dominio actual (p. ej. detectar entorno local sin tocar `window` fuera). */
export function nombreHost(): string {
  return window.location.hostname;
}

/* Repite `fn` cada `ms`; devuelve función para detenerlo (polling suave). */
export function repetirCada(ms: number, fn: () => void): () => void {
  const t = window.setInterval(fn, ms);
  return () => window.clearInterval(t);
}

/* Navegación completa a `ruta` (pública ↔ admin son árboles distintos). */
export function irA(ruta: string): void {
  window.location.assign(ruta);
}

/* Confirmación nativa (descartes y reinicios destructivos). */
export function confirmar(mensaje: string): boolean {
  return window.confirm(mensaje);
}

/* Suscribe `fn` a un evento de ventana; devuelve función para soltarlo. */
export function suscribirEvento(nombre: string, fn: () => void): () => void {
  window.addEventListener(nombre, fn);
  return () => window.removeEventListener(nombre, fn);
}
