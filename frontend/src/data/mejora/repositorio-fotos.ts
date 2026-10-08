// Repositorio de fotos mejoradas: única puerta de entrada.
// - La copia `original` es inmutable y no depende del input de subida.
// - La `mejorada` la escribe solo el backend tras procesar.
// - Migración suave: importa `inmueble.fotos` (dataURL) como originales.

import type { Inmueble } from '../../domain/inmueble';
import { nuevoId, type ConfigMejora, type FotoMejora, type ParcheFoto } from '../../domain/foto-mejora';
import { CLAVE_CONFIG_MEJORA, CONFIG_DEFECTO, normalizarConfig } from '../../domain/foto-mejora';
import {
  CLAVE_HISTORIAL_MEJORA,
  MAX_HISTORIAL,
  crearEvento,
  normalizarHistorial,
  type EventoMejora,
  type TipoEventoMejora,
} from '../../domain/historial-mejora';
import { LocalStorageAdapter } from '../sesion/storage';
import { idbBorrarMejora, idbBorrarPorInmueble, idbGuardar, idbListar } from './idb';
import { subirMejorada } from '../inmuebles/api';

const storage = new LocalStorageAdapter();

function ahora(): string {
  return new Date().toISOString();
}

/* Importa las fotos actuales de cada inmueble como originales.
 * Acepta dataURL (formulario) y URL http(s) del servidor: las segundas se
 * guardan tal cual (sin descargar MBs); la conversión a dataURL ocurre al
 * enviar a mejora (`cliente-mejora`). Idempotente: no duplica la misma foto
 * aunque cambie su orden (p. ej. tras quitar una intermedia en Editar). */
export async function importarOriginales(inmuebles: Inmueble[], existentes: FotoMejora[]): Promise<FotoMejora[]> {
  const creadas: FotoMejora[] = [];
  for (const inmueble of inmuebles) {
    inmueble.fotos.forEach((foto, orden) => {
      if (!foto.startsWith('data:image/') && !foto.startsWith('http')) return;
      const ya = existentes.some(
        (f) =>
          f.inmuebleId === inmueble.id &&
          (f.original === foto || (f.orden === orden && f.original.length === foto.length)),
      );
      if (ya) return;
      creadas.push({
        id: nuevoId(),
        inmuebleId: inmueble.id,
        orden,
        original: foto,
        mejorada: null,
        estado: 'pendiente',
        intentos: 0,
        jobId: null,
        error: null,
        createdAt: ahora(),
        updatedAt: ahora(),
      });
    });
  }
  for (const f of creadas) {
    try {
      await idbGuardar(f);
    } catch {
      // Si IndexedDB falla, se avisa en el hook; no se pierde el inmueble.
      break;
    }
  }
  return creadas;
}

/* Funde las mejoras ya guardadas en el servidor (`inmueble.mejoradasServidor`,
 * emparejadas por `orden`) en el store local. El servidor manda en el pareo:
 * si la mejora local difiere de la del servidor (renumerado tras un borrado,
 * dupe eliminada), se converge a la del servidor; si ya no hay mejora en ese
 * `orden`, se limpia la local para que la vista caiga al original. Sin esto
 * una IndexedDB vieja sombrea para siempre el pareo reparado, porque la
 * vista prefiere lo local (`mejoradaDeLista ?? mejoradasServidor`). Las
 * dataURL (trabajo en curso aún no subido) nunca se pisan. Después rescata
 * las mejoras solo-navegador (entradas `lista` con `mejorada` dataURL, de
 * antes del auto-guardado al completar): las sube al servidor y apunta la
 * entrada a la URL resultante. Devuelve la lista actualizada (misma
 * referencia de entrada más los parches aplicados). */
export async function importarMejoradasServidor(
  inmuebles: Inmueble[],
  existentes: FotoMejora[],
): Promise<FotoMejora[]> {
  let lista = existentes;
  for (const inmueble of inmuebles) {
    if (inmueble.mejoradasServidor.length === 0) continue;
    /* [08AA-4] Convergencia local→servidor por `orden` (ver comentario). */
    const delServidor = new Map(inmueble.mejoradasServidor.map((m) => [m.orden, m.url]));
    for (const base of lista.filter((f) => f.inmuebleId === inmueble.id)) {
      if (base.mejorada?.startsWith('data:image/')) continue;
      const urlServidor = delServidor.get(base.orden);
      if (urlServidor !== undefined && base.mejorada !== urlServidor) {
        try {
          const actualizada = await marcarEstado(base, { mejorada: urlServidor, estado: 'lista', error: null });
          lista = lista.map((f) => (f.id === base.id ? actualizada : f));
        } catch {
          break;
        }
      } else if (
        urlServidor === undefined &&
        typeof base.mejorada === 'string' &&
        base.mejorada.startsWith('http')
      ) {
        try {
          const actualizada = await marcarEstado(base, { mejorada: null });
          lista = lista.map((f) => (f.id === base.id ? actualizada : f));
        } catch {
          break;
        }
      }
    }
  }
  for (const inmueble of inmuebles) {
    const locales = lista.filter(
      (f) => f.inmuebleId === inmueble.id && f.estado === 'lista' && f.mejorada?.startsWith('data:image/'),
    );
    for (const base of locales) {
      try {
        const url = await subirMejorada(inmueble.id, base.mejorada as string, base.orden);
        const actualizada = await marcarEstado(base, { mejorada: url });
        lista = lista.map((f) => (f.id === base.id ? actualizada : f));
      } catch {
        /* Sin sesión o sin red: se queda local; el próximo arranque lo
         * reintenta (sigue siendo dataURL). */
      }
    }
  }
  return lista;
}

/* Misma foto con distinta base (absoluta/relativa): se compara por ruta. */
function mismaRuta(a: string, b: string): boolean {
  if (a === b) return true;
  try {
    return new URL(a).pathname === new URL(b).pathname;
  } catch {
    return false;
  }
}

/* Purga única tras el respaldo: antes del fix, `inmueble.fotos` mezclaba
 * originales y mejoradas y estas últimas se importaron como originales
 * (duplicadas junto a las de verdad, con `orden` desplazado). Se elimina
 * la entrada cuya URL es una mejorada del servidor y se repara el `orden`
 * de las originales supervivientes. Las dataURL (formulario) no se tocan. */
export async function purgarMejoradasDuplicadas(
  inmuebles: Inmueble[],
  existentes: FotoMejora[],
): Promise<FotoMejora[]> {
  let lista = existentes;
  for (const inmueble of inmuebles) {
    const delInmueble = lista.filter((f) => f.inmuebleId === inmueble.id);
    if (delInmueble.length === 0) continue;
    for (const entrada of delInmueble) {
      if (!entrada.original.startsWith('http')) continue;
      if (inmueble.mejoradasServidor.some((m) => mismaRuta(m.url, entrada.original))) {
        try {
          await idbBorrarMejora(entrada.id);
        } catch {
          break;
        }
        lista = lista.filter((f) => f.id !== entrada.id);
        continue;
      }
      const indice = inmueble.fotos.findIndex((foto) => mismaRuta(foto, entrada.original));
      if (indice >= 0 && entrada.orden !== indice) {
        try {
          const actualizada = await marcarEstado(entrada, { orden: indice });
          lista = lista.map((f) => (f.id === entrada.id ? actualizada : f));
        } catch {
          break;
        }
      }
    }
  }
  return lista;
}

/* Purga huérfanas con original muerta: la foto del servidor se borró o se
 * re-subió con otra URL y la entrada local apunta a un 404 (tarjeta con la
 * original en negro). Se conserva la entrada si su mejorada sigue viva en
 * el servidor (aunque ya no se renderiza, evita re-encolarla). La foto
 * vigente, si la hay, ya tiene su propia entrada o la crea el importador.
 * Corre tras `purgarMejoradasDuplicadas` y antes de `importarOriginales`. */
export async function purgarHuerfanas(
  inmuebles: Inmueble[],
  existentes: FotoMejora[],
): Promise<FotoMejora[]> {
  let lista = existentes;
  for (const entrada of existentes) {
    if (!entrada.original.startsWith('http')) continue; // formulario: aún no subidas
    const inmueble = inmuebles.find((i) => i.id === entrada.inmuebleId);
    /* Inmueble aún no cargado (primer arranque con lista vacía) NO es
     * huérfana: se conserva. Las de inmuebles borrados de verdad las purga
     * el hook (`idsHuerfanos`), que sí distingue vacío de eliminado. */
    if (!inmueble) continue;
    const vigente =
      inmueble.fotos.some((foto) => mismaRuta(foto, entrada.original)) ||
      inmueble.mejoradasServidor.some(
        (m) => mismaRuta(m.url, entrada.original) || (entrada.mejorada !== null && mismaRuta(m.url, entrada.mejorada)),
      );
    if (vigente) continue;
    try {
      await idbBorrarMejora(entrada.id);
    } catch {
      break;
    }
    lista = lista.filter((f) => f.id !== entrada.id);
  }
  return lista;
}

export async function listarFotos(): Promise<FotoMejora[]> {
  try {
    const todas = await idbListar();
    // Migración suave: registros anteriores a F13 no tienen `jobId`.
    for (const t of todas) {
      if (typeof t.jobId !== 'string') t.jobId = null;
    }
    return [...todas].sort((a, b) =>
      a.inmuebleId === b.inmuebleId ? a.orden - b.orden : a.createdAt.localeCompare(b.createdAt),
    );
  } catch {
    return [];
  }
}

export async function marcarEstado(foto: FotoMejora, parche: ParcheFoto): Promise<FotoMejora> {
  const actualizada: FotoMejora = { ...foto, ...parche, updatedAt: ahora() };
  await idbGuardar(actualizada);
  return actualizada;
}

export async function reintentarFoto(foto: FotoMejora): Promise<FotoMejora> {
  return marcarEstado(foto, { estado: 'pendiente', error: null });
}

export async function borrarFotosDeInmueble(inmuebleId: string): Promise<void> {
  await idbBorrarPorInmueble(inmuebleId);
}

export function leerConfig(): ConfigMejora {
  return normalizarConfig(storage.leer<unknown>(CLAVE_CONFIG_MEJORA));
}

export function guardarConfig(config: ConfigMejora): void {
  storage.escribir(CLAVE_CONFIG_MEJORA, normalizarConfig(config));
}

export function configPorDefecto(): ConfigMejora {
  return { ...CONFIG_DEFECTO };
}

/* Historial visible de mejoras (localStorage, máx 200 entradas).
 * Es solo lectura para revisar: nunca afecta a las fotos ni a la cola. */
export function listarHistorial(): EventoMejora[] {
  return normalizarHistorial(storage.leer<unknown>(CLAVE_HISTORIAL_MEJORA)).reverse();
}

export function registrarEvento(
  fotoId: string,
  inmuebleId: string,
  tipo: TipoEventoMejora,
  detalle: string | null,
): EventoMejora[] {
  const previos = normalizarHistorial(storage.leer<unknown>(CLAVE_HISTORIAL_MEJORA));
  const siguientes = [...previos, crearEvento(fotoId, inmuebleId, tipo, detalle)].slice(-MAX_HISTORIAL);
  try {
    storage.escribir(CLAVE_HISTORIAL_MEJORA, siguientes);
  } catch {
    // Si el storage está lleno, el historial se pierde pero las fotos no.
  }
  return [...siguientes].reverse();
}

export function limpiarHistorial(): void {
  try {
    storage.escribir(CLAVE_HISTORIAL_MEJORA, []);
  } catch {
    // Sin storage no hay nada que limpiar.
  }
}
