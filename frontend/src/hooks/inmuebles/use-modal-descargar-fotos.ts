import { useState } from 'react';
import type { Inmueble } from '@/domain/inmueble';
import { descargarDataUrl, descargarUrl, esperar } from '@/platform/descarga';
import { extensionDeFoto, slugificar } from '@/hooks/inmuebles/use-modal-ver-inmueble';

/* [08AA-32] Selección y descargas del modal (regla 8 / componente-sin-hook):
 * estado de selección/ocupado/fallo + descargas individual, todas y
 * selección. El componente (`modal-descargar-fotos.tsx`) queda con solo
 * JSX + helpers puros de presentación. */

export type Ocupado = 'todas' | 'seleccion' | null;

/* URL a descargar de la foto `indice`: la mejorada del servidor cuando
 * existe, si no el original. */
export function urlDescargaDe(inmueble: Inmueble, indice: number): string {
  return inmueble.mejoradasServidor.find((m) => m.orden === indice)?.url ?? inmueble.fotos[indice]!;
}

export function tieneMejorada(inmueble: Inmueble, indice: number): boolean {
  return inmueble.mejoradasServidor.some((m) => m.orden === indice);
}

export function useModalDescargarFotos(inmueble: Inmueble | null, alCerrar: () => void) {
  const [seleccion, setSeleccion] = useState<number[]>([]);
  const [unoDescargando, setUnoDescargando] = useState<number | null>(null);
  const [ocupado, setOcupado] = useState<Ocupado>(null);
  const [fallo, setFallo] = useState<string | null>(null);
  const abierto = inmueble !== null;
  const total = inmueble?.fotos.length ?? 0;

  const alternar = (indice: number) => {
    setSeleccion((previa) =>
      previa.includes(indice) ? previa.filter((i) => i !== indice) : [...previa, indice],
    );
  };

  async function descargarIndices(inmuebleActual: Inmueble, indices: number[]) {
    const base = slugificar(inmuebleActual.titulo);
    for (let n = 0; n < indices.length; n++) {
      const indice = indices[n]!;
      const url = urlDescargaDe(inmuebleActual, indice);
      const variante = tieneMejorada(inmuebleActual, indice) ? 'mejorada' : 'foto';
      const nombre = `${base}-${variante}-${indice + 1}.${extensionDeFoto(url)}`;
      if (url.startsWith('data:')) {
        descargarDataUrl(url, nombre);
      } else {
        await descargarUrl(url, nombre);
      }
      if (n < indices.length - 1) await esperar(600);
    }
  }

  const descargarUna = async (indice: number) => {
    if (!inmueble || ocupado !== null || unoDescargando !== null) return;
    setUnoDescargando(indice);
    setFallo(null);
    try {
      await descargarIndices(inmueble, [indice]);
    } catch (e) {
      setFallo(e instanceof Error ? e.message : 'No se pudo descargar la foto.');
    } finally {
      setUnoDescargando(null);
    }
  };

  const descargarTodas = async () => {
    if (!inmueble || ocupado !== null) return;
    setOcupado('todas');
    setFallo(null);
    try {
      await descargarIndices(
        inmueble,
        inmueble.fotos.map((_, indice) => indice),
      );
    } catch (e) {
      setFallo(e instanceof Error ? e.message : 'No se pudieron descargar las fotos.');
    } finally {
      setOcupado(null);
    }
  };

  const descargarSeleccion = async () => {
    if (!inmueble || ocupado !== null || seleccion.length === 0) return;
    setOcupado('seleccion');
    setFallo(null);
    try {
      await descargarIndices(inmueble, [...seleccion].sort((a, b) => a - b));
    } catch (e) {
      setFallo(e instanceof Error ? e.message : 'No se pudieron descargar las fotos.');
    } finally {
      setOcupado(null);
    }
  };

  const cerrar = (abrir: boolean) => {
    if (!abrir) {
      setSeleccion([]);
      setFallo(null);
      alCerrar();
    }
  };

  return {
    seleccion,
    unoDescargando,
    ocupado,
    fallo,
    abierto,
    total,
    alternar,
    descargarUna,
    descargarTodas,
    descargarSeleccion,
    cerrar,
  };
}
