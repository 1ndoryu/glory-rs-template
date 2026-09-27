import type { FichaAsk } from '../../data/inmuebles/ficha-ask';
import type { PasoAsk, ValorUbicacion } from '../../domain/pasos-ask';
import type { Inmueble } from '../../domain/inmueble';

/* Estado y updaters puros de /ask (279A-4): el hook conserva solo el cableado. */

export type ValorAsk = string | number | boolean | null | ValorUbicacion;

export interface EstadoAsk {
  inmuebles: Inmueble[];
  seleccionado: Inmueble | null;
  ficha: FichaAsk;
  pasos: PasoAsk[];
  indice: number;
  cargando: boolean;
  guardando: boolean;
  error: string | null;
  aviso: string | null;
  terminado: boolean;
}

export const FICHA_VACIA: FichaAsk = { extras: {}, precioMinimo: null };

export function esUbicacion(v: ValorAsk): v is ValorUbicacion {
  return typeof v === 'object' && v !== null;
}

export function trasFicha(guardada: FichaAsk): (e: EstadoAsk) => EstadoAsk {
  return (e) => ({ ...e, ficha: guardada, guardando: false, indice: Math.min(e.indice + 1, e.pasos.length) });
}

export function trasInmueble(guardado: Inmueble): (e: EstadoAsk) => EstadoAsk {
  return (e) => ({
    ...e,
    seleccionado: guardado,
    inmuebles: e.inmuebles.map((i) => (i.id === guardado.id ? guardado : i)),
    guardando: false,
    indice: Math.min(e.indice + 1, e.pasos.length),
  });
}
