import { useCallback, useEffect, useRef, useState } from 'react';
import { listarRemoto } from '../../data/inmuebles/api';
import { guardarFicha, obtenerFicha, type FichaAsk } from '../../data/inmuebles/ficha-ask';
import { CHECKLIST_ASK, type ExtrasAsk } from '../../domain/ficha-ask';
import type { Inmueble } from '../../domain/inmueble';

/* Máquina del cuestionario /ask (279A-3): lista, ficha, pregunta actual y
 * guardado respuesta por respuesta. Una pregunta por vez; saltar y
 * no-aplica nunca bloquean (el % es solo aviso visual). */

interface EstadoAsk {
  inmuebles: Inmueble[];
  seleccionado: Inmueble | null;
  ficha: FichaAsk;
  indice: number;
  cargando: boolean;
  guardando: boolean;
  error: string | null;
  aviso: string | null;
}

const VACIA: FichaAsk = { extras: {}, precioMinimo: null };

export function useAsk() {
  const [estado, setEstado] = useState<EstadoAsk>({
    inmuebles: [],
    seleccionado: null,
    ficha: VACIA,
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
    setEstado((e) => ({ ...e, seleccionado: inmueble, cargando: true, error: null, aviso: null, indice: 0 }));
    try {
      const ficha = await obtenerFicha(inmueble.id);
      setEstado((e) => ({ ...e, ficha, cargando: false }));
    } catch (e: unknown) {
      setEstado((s) => ({ ...s, cargando: false, error: e instanceof Error ? e.message : 'No se pudo cargar la ficha.' }));
    }
  }, []);

  /* Ref mutable para `responder` sin re-crearla por cada tecla. */
  const estadoRef = useRef(estado);
  estadoRef.current = estado;

  /* Guarda la respuesta de la pregunta actual y avanza (PUT real por
   * respuesta: sin llamadas = sin persistencia, regla 6). */
  const responder = useCallback(async (valor: string | number | boolean | null) => {
    const { seleccionado, ficha, indice } = estadoRef.current;
    if (!seleccionado) return;
    const preguntas = CHECKLIST_ASK[seleccionado.tipo] ?? [];
    const pregunta = preguntas[indice];
    if (!pregunta) return;
    setEstado((e) => ({ ...e, guardando: true, error: null }));
    try {
      const extras: ExtrasAsk = { ...ficha.extras };
      let precioMinimo = ficha.precioMinimo;
      if (valor === null) {
        if (pregunta.clave === 'precio_minimo') precioMinimo = null;
        else delete extras[pregunta.clave];
      } else if (pregunta.clave === 'precio_minimo') {
        precioMinimo = typeof valor === 'number' ? valor : Number(valor);
      } else {
        extras[pregunta.clave] = typeof valor === 'string' ? valor.trim() : valor;
      }
      const guardada = await guardarFicha(seleccionado.id, { extras, precioMinimo });
      setEstado((e) => ({ ...e, ficha: guardada, guardando: false, indice: Math.min(e.indice + 1, preguntas.length), aviso: null }));
    } catch (e: unknown) {
      setEstado((s) => ({ ...s, guardando: false, error: e instanceof Error ? e.message : 'No se pudo guardar.' }));
    }
  }, []);

  const saltar = useCallback(() => {
    setEstado((e) => {
      const total = CHECKLIST_ASK[e.seleccionado?.tipo ?? 'apartamento']?.length ?? 0;
      return { ...e, indice: Math.min(e.indice + 1, total) };
    });
  }, []);

  const volver = useCallback(() => {
    setEstado((e) => ({ ...e, seleccionado: null, ficha: VACIA, indice: 0, error: null, aviso: null }));
  }, []);

  return { ...estado, elegir, responder, saltar, volver };
}
