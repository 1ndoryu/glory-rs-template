// Vinculación manual hilo→inmueble (09AA-19 F7c): catálogo de inmuebles y
// acción de vínculo para la vista de hilos huérfanos. La lista de hilos la
// aporta `useChatsMarketplace` (fuente única, sin segundo fetch); aquí solo
// el destino del vínculo.

import { useCallback, useEffect, useState } from 'react';
import { ErrorApi } from '../../data/inmuebles/api';
import { cargarInmuebles, vincularMarketplace } from '../../data/inmuebles/repositorio-inmuebles';
import { normalizarMarketplaceId, type Inmueble } from '../../domain/inmueble';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado de vinculación.';
}

export function useVincularHilo() {
  const [inmuebles, setInmuebles] = useState<Inmueble[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [vinculando, setVinculando] = useState(false);
  const [aviso, setAviso] = useState<string | null>(null);

  useEffect(() => {
    let vivo = true;
    cargarInmuebles()
      .then((lista) => {
        if (vivo) {
          setInmuebles(lista);
          setError(null);
        }
      })
      .catch((e: unknown) => vivo && setError(mensajeError(e)))
      .finally(() => vivo && setCargando(false));
    return () => {
      vivo = false;
    };
  }, []);

  /* Fija `marketplaceId` en el inmueble con el PUT completo (ya envía
   * `marketplace_id`; el backend lo ignora hasta la migración F7a y el
   * `avisoConocido` futuro lo confirmará al siguiente borrador). */
  const vincular = useCallback(
    async (inmuebleId: string, marketplaceId: string): Promise<boolean> => {
      const inmueble = inmuebles.find((i) => i.id === inmuebleId);
      const normalizado = normalizarMarketplaceId(marketplaceId);
      if (!inmueble || !normalizado) return false;
      setVinculando(true);
      try {
        const { lista, resultado } = await vincularMarketplace(inmueble, normalizado, inmuebles);
        setInmuebles(lista);
        if (!resultado.ok) {
          setError(resultado.motivo ?? 'No se pudo vincular.');
          return false;
        }
        setAviso(`Hilo vinculado a «${inmueble.titulo}» (aviso ${normalizado}).`);
        return true;
      } catch (e: unknown) {
        setError(mensajeError(e));
        return false;
      } finally {
        setVinculando(false);
      }
    },
    [inmuebles],
  );

  return { inmuebles, cargando, error, vinculando, aviso, vincular };
}
