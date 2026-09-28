// Bandeja staff del chat (169A-5): sesiones + hilo abierto con refresco,
// responder (toma el hilo), soltar/tomar IA y cerrar. Un solo intervalo
// (5 s) refresca bandeja e hilo; con limpieza al desmontar.

import { useCallback, useEffect, useState } from 'react';
import {
  actualizarSesion,
  historialSesion,
  listarSesiones,
  responderSesion,
  type EstadoSesionChat,
  type ResumenSesion,
} from '../../data/chat/cliente-admin';
import type { MensajeServidor } from '../../data/chat/cliente-chat';
import { ErrorApi } from '../../data/inmuebles/api';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado del chat.';
}

export function useBandejaChat() {
  const [sesiones, setSesiones] = useState<ResumenSesion[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filtro, setFiltro] = useState<'todas' | EstadoSesionChat>('todas');
  const [seleccionada, setSeleccionada] = useState<string | null>(null);
  const [hilo, setHilo] = useState<MensajeServidor[]>([]);
  const [respondiendo, setRespondiendo] = useState(false);

  const recargar = useCallback(async (sesionId: string | null, conFiltro: typeof filtro) => {
    try {
      const lista = await listarSesiones(conFiltro === 'todas' ? undefined : conFiltro);
      setSesiones(lista);
      setError(null);
      if (sesionId) {
        const mensajes = await historialSesion(sesionId);
        /* [289A-7] Nuevos arriba, viejos abajo: la conversación crece
         * hacia arriba. Orden explícito, no implícito. */
        mensajes.sort((a, b) => b.sequence_num - a.sequence_num);
        setHilo(mensajes);
      }
    } catch (e) {
      setError(mensajeError(e));
    }
  }, []);

  /* `cargando` solo cubre la primera carga (va en `true` inicial):
   * filtro/selección refrescan sobre los datos visibles, sin pantallazo. */
  useEffect(() => {
    let viva = true;
    void recargar(seleccionada, filtro).finally(() => {
      if (viva) setCargando(false);
    });
    const temporizador = window.setInterval(() => {
      if (viva) void recargar(seleccionada, filtro);
    }, 5000);
    return () => {
      viva = false;
      window.clearInterval(temporizador);
    };
  }, [recargar, seleccionada, filtro]);

  async function responder(texto: string): Promise<boolean> {
    if (!seleccionada || !texto.trim()) return false;
    setRespondiendo(true);
    try {
      await responderSesion(seleccionada, texto.trim());
      await recargar(seleccionada, filtro);
      return true;
    } catch (e) {
      setError(mensajeError(e));
      return false;
    } finally {
      setRespondiendo(false);
    }
  }

  /* Soltar la IA (`true`), tomarla a mano (`false`) o cerrar el hilo. */
  async function cambiarSesion(cambio: { aiEnabled?: boolean; status?: EstadoSesionChat }): Promise<void> {
    if (!seleccionada) return;
    try {
      await actualizarSesion(seleccionada, cambio);
      await recargar(seleccionada, filtro);
    } catch (e) {
      setError(mensajeError(e));
    }
  }

  const sesionActual = sesiones.find((s) => s.id === seleccionada) ?? null;
  return {
    sesiones,
    cargando,
    error,
    filtro,
    ponerFiltro: setFiltro,
    seleccionada: sesionActual,
    hilo,
    seleccionar: setSeleccionada,
    responder,
    respondiendo,
    cambiarSesion,
  };
}
