// Clientes de la dueña (279A-2 F5): buscar, alta, ficha editable,
// sesiones del cliente y envío manual por WhatsApp. Sin polling: recarga
// tras cada mutación.

import { useCallback, useEffect, useState } from 'react';
import { temporizar } from '../../platform/ventana';
import {
  actualizarCliente,
  crearCliente,
  enviarManual,
  listarClientes,
  sesionesDeCliente,
  type ClienteDuena,
  type SesionCliente,
} from '../../data/chat/cliente-duena';
import { ErrorApi } from '../../data/inmuebles/api';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado de clientes.';
}

export function useClientes() {
  const [clientes, setClientes] = useState<ClienteDuena[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busqueda, setBusqueda] = useState('');
  const [seleccionado, setSeleccionado] = useState<string | null>(null);
  const [sesiones, setSesiones] = useState<SesionCliente[]>([]);
  const [aviso, setAviso] = useState<string | null>(null);

  const recargar = useCallback(async (q: string) => {
    try {
      setClientes(await listarClientes(q));
      setError(null);
    } catch (e) {
      setError(mensajeError(e));
    }
  }, []);

  useEffect(() => {
    let viva = true;
    /* [08AA-23] Debounce vía plataforma (sin `window` directo aquí). */
    const cancelar = temporizar(300, () => {
      if (viva) void recargar(busqueda).finally(() => viva && setCargando(false));
    });
    return () => {
      viva = false;
      cancelar();
    };
  }, [recargar, busqueda]);

  /* Al elegir cliente trae sus hilos; al soltarlo limpia. */
  useEffect(() => {
    if (!seleccionado) {
      setSesiones([]);
      return;
    }
    let viva = true;
    sesionesDeCliente(seleccionado)
      .then((s) => viva && setSesiones(s))
      .catch((e: unknown) => viva && setError(mensajeError(e)));
    return () => {
      viva = false;
    };
  }, [seleccionado]);

  async function alta(nombre: string, telefono: string): Promise<void> {
    try {
      await crearCliente({ nombre: nombre.trim() || undefined, telefono: telefono.trim() });
      setAviso(null);
      await recargar(busqueda);
    } catch (e) {
      setError(mensajeError(e));
    }
  }

  async function guardarFicha(id: string, ficha: Record<string, string>): Promise<void> {
    try {
      await actualizarCliente(id, ficha);
      setAviso(null);
      await recargar(busqueda);
    } catch (e) {
      setError(mensajeError(e));
    }
  }

  /* «Dime y lo envío»: encola en outbox al hilo del cliente, con foto
   * opcional (F4-parcial: URL http(s) que valida el backend). */
  async function enviar(clienteId: string, texto: string, media: string): Promise<void> {
    try {
      const r = await enviarManual({
        cliente_id: clienteId,
        texto: texto.trim(),
        media_url: media.trim() || undefined,
      });
      setAviso(`Encolado a ${r.destino} (outbox ${r.outbox_id.slice(0, 8)}).`);
    } catch (e) {
      setError(mensajeError(e));
    }
  }

  const actual = clientes.find((c) => c.id === seleccionado) ?? null;
  return {
    clientes,
    cargando,
    error,
    busqueda,
    ponerBusqueda: setBusqueda,
    seleccionado: actual,
    elegir: setSeleccionado,
    sesiones,
    aviso,
    alta,
    guardarFicha,
    enviar,
  };
}
