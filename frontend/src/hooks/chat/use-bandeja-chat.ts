// Bandeja staff del chat (169A-5): sesiones + hilo abierto con refresco,
// responder (toma el hilo), soltar/tomar IA y cerrar. Un solo intervalo
// (5 s) refresca bandeja e hilo; con limpieza al desmontar.
// [309A-3] El hilo lo lleva `useHiloPaginado` (última página al abrir,
// fusión al refrescar, anteponer al subir): aquí solo sesiones y acciones.

import { useCallback, useEffect, useState } from 'react';
import {
  actualizarSesion,
  listarSesiones,
  responderSesion,
  type EstadoSesionChat,
  type ResumenSesion,
} from '../../data/chat/cliente-admin';
import { ErrorApi } from '../../data/inmuebles/api';
import { repetirCada } from '../../platform/ventana';
import { useHiloPaginado } from './use-hilo-paginado';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado del chat.';
}

export function useBandejaChat() {
  const [sesiones, setSesiones] = useState<ResumenSesion[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filtro, setFiltro] = useState<'todas' | EstadoSesionChat>('todas');
  const [seleccionada, setSeleccionada] = useState<string | null>(null);
  const [respondiendo, setRespondiendo] = useState(false);
  const hilo = useHiloPaginado();
  /* Estable entre renders (el objeto `hilo` no): evita recrear `recargar`
   * y re-disparar el efecto en cada render. */
  const { refrescarHilo } = hilo;

  const recargar = useCallback(async (sesionId: string | null, conFiltro: typeof filtro) => {
    try {
      const lista = await listarSesiones(conFiltro === 'todas' ? undefined : conFiltro);
      setSesiones(lista);
      setError(null);
      /* El refresco fusiona (no resetea páginas viejas); al seleccionar,
       * `seleccionar` ya abrió el hilo desde cero y esto solo duplica la
       * misma página (fusión idempotente por id). */
      await refrescarHilo(sesionId);
    } catch (e) {
      setError(mensajeError(e));
    }
  }, [refrescarHilo]);

  /* `cargando` solo cubre la primera carga (va en `true` inicial):
   * filtro/selección refrescan sobre los datos visibles, sin pantallazo. */
  useEffect(() => {
    let viva = true;
    void recargar(seleccionada, filtro).finally(() => {
      if (viva) setCargando(false);
    });
    /* [08AA-23] Intervalo vía plataforma (sin `window` directo aquí). */
    const detener = repetirCada(5000, () => {
      if (viva) void recargar(seleccionada, filtro);
    });
    return () => {
      viva = false;
      detener();
    };
  }, [recargar, seleccionada, filtro]);

  /* Cambiar de hilo lo abre desde cero (última página). */
  function seleccionar(id: string | null) {
    setSeleccionada(id);
    void hilo.abrirHilo(id);
  }

  async function responder(texto: string): Promise<boolean> {
    if (!seleccionada || !texto.trim()) return false;
    setRespondiendo(true);
    try {
      await responderSesion(seleccionada, texto.trim());
      await hilo.refrescarHilo(seleccionada);
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
    error: error ?? hilo.errorHilo,
    filtro,
    ponerFiltro: setFiltro,
    seleccionada: sesionActual,
    hilo: hilo.hilo,
    hayMas: hilo.hayMas,
    cargandoMas: hilo.cargandoMas,
    cargarAnteriores: () => hilo.cargarAnteriores(seleccionada),
    seleccionar,
    responder,
    respondiendo,
    cambiarSesion,
  };
}
