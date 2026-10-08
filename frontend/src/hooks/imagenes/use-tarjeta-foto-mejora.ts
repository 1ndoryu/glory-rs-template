import { useState } from 'react';
import type { FotoMejora } from '@/domain/foto-mejora';
import type { InfoReintento } from '@/hooks/mejora/use-cola-mejora';
import { descargarUrl } from '@/platform/descarga';
import { confirmar } from '@/platform/ventana';

/* [08AA-32] Estado interactivo de la tarjeta (regla 8 /
 * componente-sin-hook): ampliada, restaurar (con confirmación, es borrado
 * real) y descarga vía blob. El componente (`tarjeta-foto-mejora.tsx`)
 * queda con solo JSX + helpers puros de presentación. */

/* Nombre seguro para la descarga: último segmento de la URL sin query,
 * saneado porque `:` (p. ej. en blob:) es inválido en Windows. */
function nombreArchivo(url: string, defecto: string): string {
  const base = url.split('/').pop()?.split('?')[0]?.trim() || defecto;
  const limpio = base.replace(/[^A-Za-z0-9._-]/g, '_');
  return limpio || `${defecto}.jpg`;
}

export function useTarjetaFotoMejora(params: {
  foto: FotoMejora;
  alRestaurar: (() => Promise<void>) | null;
  idServidor: string | null;
  reintento?: InfoReintento | null;
}) {
  const { foto, alRestaurar, idServidor, reintento } = params;
  const [ampliada, setAmpliada] = useState<'original' | 'mejorada' | null>(null);
  const [restaura, setRestaura] = useState<{ enCurso: boolean; error: string | null }>({
    enCurso: false,
    error: null,
  });
  const [descarga, setDescarga] = useState<{ enCurso: boolean; error: string | null }>({
    enCurso: false,
    error: null,
  });
  /* Objeto cuando hay reintento visible, null si no: el componente lo usa
   * como guarda (`conReintento && …`) y TS lo estrecha a no-nulo dentro. */
  const conReintento: InfoReintento | null =
    foto.estado === 'procesando' && reintento && (reintento.motivo || (reintento.enSeg ?? 0) > 0)
      ? reintento
      : null;
  const urlAmpliada = ampliada === 'mejorada' ? (foto.mejorada ?? foto.original) : foto.original;
  const puedeRestaurar = alRestaurar !== null && idServidor !== null && foto.estado !== 'procesando';

  /* Restaurar descarta la mejorada del servidor y deja el original
   * vigente; el llamador limpia además la copia local (queda pendiente
   * para mejorarla de nuevo). Con confirmación: es un borrado real. */
  async function restaurar(): Promise<void> {
    if (!alRestaurar || restaura.enCurso) return;
    if (
      !confirmar(
        'Descartar la mejorada del servidor y volver al original? La foto quedará pendiente para mejorarla de nuevo.',
      )
    ) {
      return;
    }
    setRestaura({ enCurso: true, error: null });
    try {
      await alRestaurar();
      setRestaura({ enCurso: false, error: null });
    } catch (e) {
      setRestaura({ enCurso: false, error: e instanceof Error ? e.message : 'No se pudo restaurar el original.' });
    }
  }

  /* Descarga original o mejorada como archivo (vía blob temporal; el
   * atributo `download` se ignora entre orígenes). El fallo queda visible
   * en la tarjeta, nunca silenciado. */
  async function descargar(cual: 'original' | 'mejorada'): Promise<void> {
    const url = cual === 'mejorada' ? foto.mejorada : foto.original;
    if (!url || descarga.enCurso) return;
    setDescarga({ enCurso: true, error: null });
    try {
      await descargarUrl(url, nombreArchivo(url, cual));
      setDescarga({ enCurso: false, error: null });
    } catch (e) {
      setDescarga({ enCurso: false, error: e instanceof Error ? e.message : 'No se pudo descargar la foto.' });
    }
  }

  return {
    ampliada,
    setAmpliada,
    restaura,
    descarga,
    conReintento,
    urlAmpliada,
    puedeRestaurar,
    restaurar,
    descargar,
  };
}
