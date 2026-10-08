import { useCallback, useState } from 'react';
import type { EstadoInmueble } from '@/domain/inmueble';

/* [08AA-33] Máquina del botón "Esta propiedad se vendió": abre un modal
 * pequeño centrado (doble confirmación contra toques accidentales) y, al
 * confirmar, delega el marcado a `alMarcar` (devuelve `true` si guardó).
 * El error queda visible dentro del modal; cerrar limpia el estado. */

export function useVendida(alMarcar: (id: string, estado: EstadoInmueble) => Promise<boolean>) {
  const [abierto, setAbierto] = useState(false);
  const [marcando, setMarcando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const abrir = useCallback(() => {
    setError(null);
    setAbierto(true);
  }, []);

  const cerrar = useCallback(() => {
    setError(null);
    setAbierto(false);
  }, []);

  const confirmar = useCallback(
    async (id: string, estado: EstadoInmueble) => {
      setMarcando(true);
      setError(null);
      try {
        const ok = await alMarcar(id, estado);
        if (ok) setAbierto(false);
        else setError('No se pudo marcar. Inténtalo de nuevo.');
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : 'No se pudo marcar.');
      } finally {
        setMarcando(false);
      }
    },
    [alMarcar],
  );

  return { abierto, marcando, error, abrir, cerrar, confirmar };
}
