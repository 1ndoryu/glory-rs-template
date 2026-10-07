// Estado del panel por chat (07AA-7): lista de hilos + detalle del elegido.
// El hilo es la clave de ventana del puente (trae nombre+aviso: solo-admin).

import { useCallback, useEffect, useState } from 'react';
import { leerChat, listarChats, type ChatFila, type ChatResumen } from '../../data/chat/marketplace-chats';
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

  return { lista, seleccion, error, recargar, elegir };
}
