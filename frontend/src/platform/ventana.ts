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

/* [08AA-23] ¿El sistema pide tema oscuro? (sin `matchMedia` fuera). */
export function sistemaQuiereOscuro(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

/* [08AA-23] ¿El sistema pide movimiento reducido? (sin `matchMedia` fuera). */
export function prefiereMovimientoReducido(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/* [08AA-23] Observa cambios del tema del sistema; devuelve función para
 * soltarlo (sin `MediaQueryList` fuera). */
export function observarTemaSistema(fn: () => void): () => void {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  mq.addEventListener('change', fn);
  return () => mq.removeEventListener('change', fn);
}

/* [08AA-23] Emite un evento de ventana por nombre (sin `Event` fuera). */
export function emitirEvento(nombre: string): void {
  window.dispatchEvent(new Event(nombre));
}

/* [08AA-23] Abre URL externa en pestaña nueva aislada (sin `opener`). */
export function abrirExterna(url: string): void {
  window.open(url, '_blank', 'noopener,noreferrer');
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
