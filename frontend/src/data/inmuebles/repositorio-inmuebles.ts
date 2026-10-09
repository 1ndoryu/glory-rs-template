import type { Inmueble } from '../../domain/inmueble';
import {
  ErrorApi,
  actualizarRemoto,
  crearRemoto,
  eliminarRemoto,
  fijarPublicado,
  listarRemoto,
  sincronizarFotos,
} from './api';
import { borrarFotosDeInmueble } from '../mejora/repositorio-fotos';

export type ResultadoGuardado = { ok: true } | { ok: false; motivo: string };

// Repositorio: única puerta de entrada a los inmuebles. La API es la única
// fuente (nada local): cada mutación escribe primero el registro y luego
// iguala las fotos con `sincronizarFotos`. Errores explícitos, nunca
// fallos silenciosos.
function motivo(error: unknown, defecto: string): string {
  return error instanceof Error ? error.message : defecto;
}

/* Carga la lista del servidor. Lanza `ErrorApi` si no hay sesión o la API
 * no responde: el hook lo muestra en el banner (sin lista vacía muda). */
export async function cargarInmuebles(): Promise<{ lista: Inmueble[]; fotosOk: boolean }> {
  return { lista: await listarRemoto(), fotosOk: true };
}

/* Crear: primero el registro (devuelve id y slug), luego las fotos. Si la
 * subida falla, el registro queda creado sin fotos y se informa: nada se
 * pierde porque el modal conserva el borrador con sus fotos. */
export async function crearInmueble(
  inmueble: Inmueble,
  actuales: Inmueble[],
): Promise<{ lista: Inmueble[]; resultado: ResultadoGuardado }> {
  let creado: Inmueble;
  try {
    creado = await crearRemoto(inmueble);
  } catch (error) {
    return { lista: actuales, resultado: { ok: false, motivo: motivo(error, 'No se pudo crear el inmueble.') } };
  }
  if (inmueble.fotos.length > 0) {
    try {
      const sync = await sincronizarFotos(creado.id, inmueble.fotos);
      creado = { ...creado, fotos: sync.fotos };
      // Registro nuevo: no hay copias de mejora previas que invalidar.
    } catch (error) {
      return {
        lista: [creado, ...actuales],
        resultado: {
          ok: false,
          motivo: `Inmueble creado sin fotos: ${motivo(error, 'falló la subida')}. Vuelve a editarlo para subirlas.`,
        },
      };
    }
  }
  return { lista: [creado, ...actuales], resultado: { ok: true } };
}

export async function actualizarInmueble(
  actualizado: Inmueble,
  actuales: Inmueble[],
): Promise<{ lista: Inmueble[]; resultado: ResultadoGuardado }> {
  let guardado: Inmueble;
  try {
    guardado = await actualizarRemoto(actualizado);
  } catch (error) {
    return { lista: actuales, resultado: { ok: false, motivo: motivo(error, 'No se pudo guardar el inmueble.') } };
  }
  try {
    const sync = await sincronizarFotos(guardado.id, actualizado.fotos);
    /* Se adoptan fotos Y mejoradas del sync: tras re-subir, las URLs
     * anteriores ya no existen en el servidor (conservarlas dibuja la
     * imagen publicitaria vieja). */
    guardado = { ...guardado, fotos: sync.fotos, mejoradasServidor: sync.mejoradas };
    if (sync.cambiaron) {
      /* Las entradas de mejora describían fotos que ya no existen (posición
       * o contenido distinto): se purgan para no cementar `orden` viejos ni
       * mostrar mejoradas huérfanas. Las mejoradas que siguen vivas se
       * reimportan del servidor al recargar la lista. */
      try {
        await borrarFotosDeInmueble(guardado.id);
      } catch {
        /* IDB inaccesible: la purga de huérfanas lo intentará de nuevo. */
      }
    }
  } catch (error) {
    return {
      lista: actuales.map((i) => (i.id === guardado.id ? guardado : i)),
      resultado: { ok: false, motivo: `Datos guardados, pero las fotos no: ${motivo(error, 'falló la subida')}.` },
    };
  }
  return { lista: actuales.map((i) => (i.id === guardado.id ? guardado : i)), resultado: { ok: true } };
}

export async function eliminarInmueble(
  id: string,
  actuales: Inmueble[],
): Promise<{ lista: Inmueble[]; resultado: ResultadoGuardado }> {
  try {
    await eliminarRemoto(id);
  } catch (error) {
    if (error instanceof ErrorApi && error.estado === 404) {
      // Ya no existe en el servidor: se saca de la lista local y se avisa.
      return { lista: actuales.filter((i) => i.id !== id), resultado: { ok: false, motivo: 'Ya no existía en el servidor.' } };
    }
    return { lista: actuales, resultado: { ok: false, motivo: motivo(error, 'No se pudo eliminar.') } };
  }
  return { lista: actuales.filter((i) => i.id !== id), resultado: { ok: true } };
}

/* Publicar/retirar de la web pública. Devuelve la lista con el inmueble
 * actualizado o el motivo del fallo. */
export async function publicarInmueble(
  id: string,
  publicado: boolean,
  actuales: Inmueble[],
): Promise<{ lista: Inmueble[]; resultado: ResultadoGuardado }> {
  try {
    const guardado = await fijarPublicado(id, publicado);
    return { lista: actuales.map((i) => (i.id === id ? guardado : i)), resultado: { ok: true } };
  } catch (error) {
    return { lista: actuales, resultado: { ok: false, motivo: motivo(error, 'No se pudo cambiar la publicación.') } };
  }
}

/* [09AA-19 F7c] Vinculación manual hilo→inmueble (vista de huérfanos): fija
 * `marketplaceId` con el PUT completo (`actualizarRemoto` ya lo envía como
 * `marketplace_id`). Hasta F7a el backend lo ignora (queda preparado); el
 * `avisoConocido` futuro lo confirmará al siguiente borrador. */
export async function vincularMarketplace(
  inmueble: Inmueble,
  marketplaceId: string | null,
  actuales: Inmueble[],
): Promise<{ lista: Inmueble[]; resultado: ResultadoGuardado }> {
  return actualizarInmueble({ ...inmueble, marketplaceId }, actuales);
}
