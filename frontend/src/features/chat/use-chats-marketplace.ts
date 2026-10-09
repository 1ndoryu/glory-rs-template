// Estado del panel por chat (07AA-7): lista de hilos + detalle del elegido.
// El hilo es la clave de ventana del puente (trae nombre+aviso: solo-admin).

import { useCallback, useEffect, useState } from 'react';
import { leerChat, limpiarChats, listarChats, regenerarTodo, type ChatFila, type ChatResumen, type RegenerarTodoResumen } from '../../data/chat/marketplace-chats';
import { ErrorApi } from '../../data/inmuebles/api';

export interface DetalleChat {
  hilo: string;
  filas: ChatFila[];
  cargando: boolean;
}

export function useChatsMarketplace() {
  const [lista, setLista] = useState<ChatResumen[]>([]);
  const [seleccion, setSeleccion] = useState<DetalleChat | null>(null);
  const [error, setError] = useState<string | null>(null);

  const recargar = useCallback(async () => {
    try {
      setLista(await listarChats());
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof ErrorApi ? e.message : 'Fallo inesperado.');
    }
  }, []);

  useEffect(() => {
    let viva = true;
    listarChats()
      .then((c) => viva && (setLista(c), setError(null)))
      .catch((e: unknown) => viva && setError(e instanceof ErrorApi ? e.message : 'Fallo inesperado.'));
    return () => {
      viva = false;
    };
  }, []);

  const elegir = useCallback(async (hilo: string) => {
    setSeleccion({ hilo, filas: [], cargando: true });
    try {
      const filas = await leerChat(hilo);
      setSeleccion({ hilo, filas, cargando: false });
    } catch (e: unknown) {
      setError(e instanceof ErrorApi ? e.message : 'Fallo inesperado.');
      setSeleccion(null);
    }
  }, []);

  /* [08AA-39] Limpieza total con confirmación: vacía caché + panel. */
  const limpiar = useCallback(async () => {
    if (!window.confirm('¿Borrar todos los chats y borradores? No se puede deshacer.')) return;
    try {
      await limpiarChats();
      setLista([]);
      setSeleccion(null);
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof ErrorApi ? e.message : 'Fallo inesperado.');
    }
  }, []);

  /* [09AA-3] Regeneración masiva con confirmación: refresca toda la
   * lista al terminar y muestra el resumen por hilo. Tarda ~15s por
   * chat (una pasada en serie contra la IA). */
  const [regenerando, setRegenerando] = useState(false);
  const [resumenRegen, setResumenRegen] = useState<RegenerarTodoResumen | null>(null);
  const regenerarTodoPanel = useCallback(async () => {
    if (!window.confirm('¿Regenerar todos los borradores? Tarda ~15s por chat y la última que escribe gana.')) return;
    setRegenerando(true);
    setResumenRegen(null);
    try {
      const resumen = await regenerarTodo();
      setResumenRegen(resumen);
      setLista(await listarChats());
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof ErrorApi ? e.message : 'Fallo inesperado.');
    } finally {
      setRegenerando(false);
    }
  }, []);

  return { lista, seleccion, error, recargar, elegir, limpiar, regenerando, resumenRegen, regenerarTodoPanel };
}
