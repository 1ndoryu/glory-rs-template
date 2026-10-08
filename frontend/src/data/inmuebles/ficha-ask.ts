// Cliente de la ficha /ask (279A-3): GET+PUT a `/api/admin/inmuebles/:id/ficha`.
// Reutiliza sesión+JWT+401 de `api.ts`. `precio_minimo` viaja aparte del
// mapa `extras` (columna privada, nunca a la web pública).

import { actualizarRemoto, apiFetch } from './api';
import type { ExtrasAsk } from '../../domain/ficha-ask';
import { pasoRespondido, pasosPara, type ColumnaNumericaAsk, type ValorUbicacion } from '../../domain/pasos-ask';
import type { Inmueble } from '../../domain/inmueble';

export interface FichaAsk {
  extras: ExtrasAsk;
  precioMinimo: number | null;
}

interface FichaAskRemota {
  extras: ExtrasAsk;
  precio_minimo: number | null;
}

export async function obtenerFicha(id: string): Promise<FichaAsk> {
  const r = await apiFetch<FichaAskRemota>(`/api/admin/inmuebles/${encodeURIComponent(id)}/ficha`);
  return { extras: r.extras ?? {}, precioMinimo: r.precio_minimo ?? null };
}

export async function guardarFicha(id: string, ficha: FichaAsk): Promise<FichaAsk> {
  const r = await apiFetch<FichaAskRemota>(`/api/admin/inmuebles/${encodeURIComponent(id)}/ficha`, {
    method: 'PUT',
    body: JSON.stringify({ extras: ficha.extras, precio_minimo: ficha.precioMinimo }),
  });
  return { extras: r.extras ?? {}, precioMinimo: r.precio_minimo ?? null };
}

/* Guardado de columnas desde /ask (279A-3 F2): la ubicación+residencia y
 * los numéricos condicionales rellenan el inmueble de verdad vía el PUT
 * admin existente (reemplazo completo, sin tocar backend). */
export async function guardarUbicacionAsk(actual: Inmueble, valor: ValorUbicacion): Promise<Inmueble> {
  return actualizarRemoto({
    ...actual,
    ubicacion: valor.ubicacion.trim(),
    residencia: valor.residencia.trim(),
  });
}

export async function guardarColumnaAsk(
  actual: Inmueble,
  columna: ColumnaNumericaAsk,
  valor: number,
): Promise<Inmueble> {
  return actualizarRemoto({ ...actual, [columna]: valor });
}

/* Primera propiedad con algo que preguntar (279A-4): candidatas en orden
 * aleatorio, trae la ficha una a una y devuelve la primera con algún paso
 * sin responder. `null` = al día (o lista vacía). Las fichas que
 * fallan se saltan en silencio: ya habrá otra candidata. */
export async function buscarPendiente(
  lista: Inmueble[],
  excluirId?: string | null,
): Promise<{ inmueble: Inmueble; ficha: FichaAsk } | null> {
  const candidatas = lista.filter((i) => i.id !== excluirId);
  for (let i = candidatas.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [candidatas[i], candidatas[j]] = [candidatas[j], candidatas[i]];
  }
  for (const candidata of candidatas) {
    try {
      const ficha = await obtenerFicha(candidata.id);
      const pendientes = pasosPara(candidata.tipo, candidata).some(
        (p) => !pasoRespondido(p, candidata, ficha.extras, ficha.precioMinimo),
      );
      if (pendientes) return { inmueble: candidata, ficha };
    } catch {
      continue;
    }
  }
  return null;
}
