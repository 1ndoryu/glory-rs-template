import { useCallback, useEffect, useRef, useState } from 'react';
import { listarRemoto } from '../../data/inmuebles/api';
import {
  guardarColumnaAsk,
  guardarFicha,
  guardarUbicacionAsk,
  obtenerFicha,
  type FichaAsk,
} from '../../data/inmuebles/ficha-ask';
import { extrasTrasRespuesta, mazclarPasos, pasosPara, type PasoAsk, type ValorUbicacion } from '../../domain/pasos-ask';
import type { Inmueble } from '../../domain/inmueble';

/* Máquina /ask (279A-3 F2): foto de pasos al elegir (ubicación + ficha +
 * numéricos que faltan, en orden aleatorio) y guardado por tipo de paso —
 * ficha a `extras`, ubicación y numéricos a columnas (PUT admin existente).
 * Saltar avanza sin guardar; anterior deshace para corregir. */

export type ValorAsk = string | number | boolean | null | ValorUbicacion;

interface EstadoAsk {
  inmuebles: Inmueble[];
  seleccionado: Inmueble | null;
  ficha: FichaAsk;
  pasos: PasoAsk[];
  indice: number;
  cargando: boolean;
  guardando: boolean;
  error: string | null;
  aviso: string | null;
}

const VACIA: FichaAsk = { extras: {}, precioMinimo: null };

function esUbicacion(v: ValorAsk): v is ValorUbicacion {
  return typeof v === 'object' && v !== null;
}

/* Updaters puros (sin hook) para no engordar el componente. */
function trasFicha(guardada: FichaAsk): (e: EstadoAsk) => EstadoAsk {
  return (e) => ({ ...e, ficha: guardada, guardando: false, indice: Math.min(e.indice + 1, e.pasos.length) });
}

function trasInmueble(guardado: Inmueble): (e: EstadoAsk) => EstadoAsk {
  return (e) => ({
    ...e,
    seleccionado: guardado,
    inmuebles: e.inmuebles.map((i) => (i.id === guardado.id ? guardado : i)),
    guardando: false,
    indice: Math.min(e.indice + 1, e.pasos.length),
  });
}

export function useAsk() {
  const [estado, setEstado] = useState<EstadoAsk>({
    inmuebles: [],
    seleccionado: null,
    ficha: VACIA,
    pasos: [],
    indice: 0,
    cargando: true,
    guardando: false,
    error: null,
    aviso: null,
  });

  useEffect(() => {
    let vivo = true;
    listarRemoto()
      .then((lista) => {
        if (vivo) setEstado((e) => ({ ...e, inmuebles: lista, cargando: false }));
      })
      .catch((e: unknown) => {
        if (vivo) setEstado((s) => ({ ...s, cargando: false, error: e instanceof Error ? e.message : 'No se pudo cargar la lista.' }));
      });
    return () => {
      vivo = false;
    };
  }, []);

  const elegir = useCallback(async (inmueble: Inmueble) => {
    setEstado((e) => ({ ...e, seleccionado: inmueble, cargando: true, error: null, aviso: null, indice: 0, pasos: [] }));
    try {
      const ficha = await obtenerFicha(inmueble.id);
      /* Orden aleatorio por sesión: no es una lista fija. */
      setEstado((e) => ({ ...e, ficha, pasos: mazclarPasos(pasosPara(inmueble.tipo, inmueble)), cargando: false }));
    } catch (e: unknown) {
      setEstado((s) => ({ ...s, cargando: false, error: e instanceof Error ? e.message : 'No se pudo cargar la ficha.' }));
    }
  }, []);

  const estadoRef = useRef(estado);
  estadoRef.current = estado;

  const responder = useCallback(async (valor: ValorAsk) => {
    const { seleccionado, ficha, pasos, indice } = estadoRef.current;
    const paso = pasos[indice];
    if (!seleccionado || !paso) return;
    /* `null` = no aplica: en ficha borra la clave; en columnas no existe
     * (la dueña salta o corrige el valor). */
    if (valor === null && paso.kind !== 'ficha') return;
    setEstado((e) => ({ ...e, guardando: true, error: null }));
    try {
      if (paso.kind === 'ficha' && !esUbicacion(valor)) {
        const guardada = await guardarFicha(seleccionado.id, extrasTrasRespuesta(ficha, paso.pregunta, valor));
        setEstado(trasFicha(guardada));
      } else if (paso.kind === 'ubicacion' && esUbicacion(valor)) {
        setEstado(trasInmueble(await guardarUbicacionAsk(seleccionado, valor)));
      } else if (paso.kind === 'columna' && typeof valor === 'number' && Number.isFinite(valor) && valor >= 0) {
        setEstado(trasInmueble(await guardarColumnaAsk(seleccionado, paso.columna, valor)));
      } else {
        setEstado((e) => ({ ...e, guardando: false }));
      }
    } catch (e: unknown) {
      setEstado((s) => ({ ...s, guardando: false, error: e instanceof Error ? e.message : 'No se pudo guardar.' }));
    }
  }, []);

  const saltar = useCallback(() => {
    setEstado((e) => ({ ...e, indice: Math.min(e.indice + 1, e.pasos.length) }));
  }, []);

  const anterior = useCallback(() => {
    setEstado((e) => ({ ...e, indice: Math.max(0, e.indice - 1) }));
  }, []);

  const volver = useCallback(() => {
    setEstado((e) => ({ ...e, seleccionado: null, ficha: VACIA, pasos: [], indice: 0, error: null, aviso: null }));
  }, []);

  return { ...estado, elegir, responder, saltar, anterior, volver };
}
