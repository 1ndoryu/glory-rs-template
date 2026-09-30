// Hilo paginado del panel staff (309A-3): abre con la última página, el
// refresco fusiona sin perder páginas viejas y el scroll arriba antepone.
// Estado separado de la bandeja (regla: hooks de ≤120 líneas).

import { useCallback, useState } from 'react';
import { historialSesion } from '../../data/chat/cliente-admin';
import type { MensajeServidor } from '../../data/chat/cliente-chat';
import { ErrorApi } from '../../data/inmuebles/api';
import {
  PAGINA_HILO,
  anteponerPagina,
  cursorAnterior,
  fusionarHilo,
  recortarPagina,
} from '../../data/chat/paginacion-hilo';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado del chat.';
}

export function useHiloPaginado() {
  const [hilo, setHilo] = useState<MensajeServidor[]>([]);
  const [hayMas, setHayMas] = useState(false);
  const [cargandoMas, setCargandoMas] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /* Abre un hilo desde cero (última página). */
  const abrir = useCallback(async (sesionId: string | null) => {
    setHilo([]);
    setHayMas(false);
    if (!sesionId) return;
    try {
      const llegados = await historialSesion(sesionId, { limit: PAGINA_HILO + 1 });
      const { pagina, hayMas } = recortarPagina(llegados);
      setHilo(pagina);
      setHayMas(hayMas);
      setError(null);
    } catch (e) {
      setError(mensajeError(e));
    }
  }, []);

  /* Refresco periódico: fusiona por id (conserva páginas viejas y
   * actualiza cuerpos editados: descripción de foto, transcripción).
   * No toca `hayMas`: la página fresca es la última y nada dice de si
   * queda historia vieja sin cargar; solo abrir/anteponer lo saben. Si
   * lo tocara, el poll resucitaría el botón tras agotar la historia. */
  const refrescar = useCallback(async (sesionId: string | null) => {
    if (!sesionId) return;
    try {
      const llegados = await historialSesion(sesionId, { limit: PAGINA_HILO + 1 });
      const { pagina } = recortarPagina(llegados);
      setHilo((prev) => fusionarHilo(prev, pagina));
      setError(null);
    } catch (e) {
      setError(mensajeError(e));
    }
  }, []);

  /* Antepone la página anterior al cursor mínimo; el componente preserva
   * el scroll (compensa el crecimiento hacia arriba). */
  const cargarAnteriores = useCallback(async (sesionId: string | null) => {
    if (!sesionId || !hayMas || cargandoMas) return;
    const cursor = cursorAnterior(hilo);
    if (cursor === null) return;
    setCargandoMas(true);
    try {
      const llegados = await historialSesion(sesionId, { limit: PAGINA_HILO + 1, beforeSeq: cursor });
      const { pagina, hayMas: mas } = recortarPagina(llegados);
      setHayMas(mas);
      setHilo((prev) => anteponerPagina(prev, pagina));
      setError(null);
    } catch (e) {
      setError(mensajeError(e));
    } finally {
      setCargandoMas(false);
    }
  }, [hilo, hayMas, cargandoMas]);

  return { hilo, hayMas, cargandoMas, errorHilo: error, abrirHilo: abrir, refrescarHilo: refrescar, cargarAnteriores };
}
