// Paginación del hilo staff (309A-3): helpers puros para pedir de a una
// página y fusionar sin duplicar. El backend pagina por cursor
// (`before_seq`); el panel pide `PAGINA + 1` y si llegan todas hay más.

import type { MensajeServidor } from './cliente-chat';

/* Mensajes visibles por página en el hilo. */
export const PAGINA_HILO = 50;

function ordenar(hilo: MensajeServidor[]): MensajeServidor[] {
  return [...hilo].sort((a, b) => a.sequence_num - b.sequence_num);
}

/* Recorta la página recién llegada: si vino llena (`PAGINA + 1`), sobra la
 * más vieja y `hayMas` queda en true. Devuelve página lista + `hayMas`. */
export function recortarPagina(llegados: MensajeServidor[]): { pagina: MensajeServidor[]; hayMas: boolean } {
  const ordenados = ordenar(llegados);
  if (ordenados.length <= PAGINA_HILO) return { pagina: ordenados, hayMas: false };
  return { pagina: ordenados.slice(1), hayMas: true };
}

/* Fusiona la página fresca (lo último) con lo ya visible: conserva páginas
 * viejas, actualiza cuerpos editados (descripción de foto, transcripción) y
 * agrega lo nuevo. Por id, nunca duplica. Si nada cambió devuelve el mismo
 * arreglo (evita re-renders del poll que robarían la compensación de
 * scroll pendiente en `HiloMensajes`). */
export function fusionarHilo(previo: MensajeServidor[], frescos: MensajeServidor[]): MensajeServidor[] {
  if (previo.length === 0) return ordenar(frescos);
  const cuerpos = new Map(previo.map((m) => [m.id, m.body]));
  const intacto = frescos.every((f) => cuerpos.get(f.id) === f.body);
  if (intacto) return previo;
  const ids = new Set(frescos.map((m) => m.id));
  return ordenar([...previo.filter((m) => !ids.has(m.id)), ...frescos]);
}

/* Antepone una página anterior (sin duplicar por id). */
export function anteponerPagina(previo: MensajeServidor[], anteriores: MensajeServidor[]): MensajeServidor[] {
  if (anteriores.length === 0) return previo;
  const ids = new Set(previo.map((m) => m.id));
  return ordenar([...anteriores.filter((m) => !ids.has(m.id)), ...previo]);
}

/* Secuencia mínima visible: cursor para pedir la página anterior. */
export function cursorAnterior(hilo: MensajeServidor[]): number | null {
  if (hilo.length === 0) return null;
  return Math.min(...hilo.map((m) => m.sequence_num));
}
