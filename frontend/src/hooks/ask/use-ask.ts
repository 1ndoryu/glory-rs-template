import { useCallback, useEffect, useRef, useState } from 'react';
import { listarRemoto } from '../../data/inmuebles/api';
import {
  buscarPendiente,
  guardarColumnaAsk,
  guardarFicha,
  guardarUbicacionAsk,
} from '../../data/inmuebles/ficha-ask';
import { extrasTrasRespuesta, mazclarPasos, pasosPara } from '../../domain/pasos-ask';
import type { Inmueble } from '../../domain/inmueble';
import {
  FICHA_VACIA,
  esUbicacion,
  trasFicha,
  trasInmueble,
  type EstadoAsk,
  type ValorAsk,
} from './estado-ask';

/* Máquina /ask (279A-3 F2 + 279A-4): sin lista de propiedades — al entrar
 * (y al pedir otra) elige sola una propiedad con algo que preguntar, en
 * orden aleatorio, y mezcla sus pasos. Guardado por tipo de paso: ficha a
 * `extras`, ubicación y numéricos a columnas (PUT admin existente).
 * Saltar avanza sin guardar; anterior deshace para corregir. */

export function useAsk() {
  const [estado, setEstado] = useState<EstadoAsk>({
    inmuebles: [],
    seleccionado: null,
    ficha: FICHA_VACIA,
    pasos: [],
    indice: 0,
    cargando: true,
    guardando: false,
    error: null,
    aviso: null,
    terminado: false,
  });

  /* Elige sola una propiedad con pendientes (279A-4): al cargar la lista y
   * cada vez que la dueña pide otra. Sin lista visible: solo preguntas. */
  const iniciarEn = useCallback(async (lista: Inmueble[], excluirId?: string | null) => {
    setEstado((e) => ({
      ...e,
      inmuebles: lista,
      seleccionado: null,
      ficha: FICHA_VACIA,
      pasos: [],
      indice: 0,
      cargando: true,
      error: null,
      aviso: null,
      terminado: false,
    }));
    try {
      const hallada = await buscarPendiente(lista, excluirId);
      if (!hallada) {
        setEstado((e) => ({ ...e, cargando: false, terminado: true }));
        return;
      }
      /* Orden aleatorio por sesión: no es una lista fija. */
      const pasos = mazclarPasos(pasosPara(hallada.inmueble.tipo, hallada.inmueble));
      setEstado((e) => ({ ...e, seleccionado: hallada.inmueble, ficha: hallada.ficha, pasos, cargando: false }));
    } catch (e: unknown) {
      setEstado((s) => ({ ...s, cargando: false, error: e instanceof Error ? e.message : 'No se pudo cargar la ficha.' }));
    }
  }, []);

  const iniciarEnRef = useRef(iniciarEn);
  iniciarEnRef.current = iniciarEn;

  useEffect(() => {
    let vivo = true;
    listarRemoto()
      .then((lista) => {
        if (vivo) void iniciarEnRef.current(lista);
      })
      .catch((e: unknown) => {
        if (vivo) setEstado((s) => ({ ...s, cargando: false, error: e instanceof Error ? e.message : 'No se pudo cargar la lista.' }));
      });
    return () => {
      vivo = false;
    };
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

  /* Otra propiedad con pendientes (excluye la actual para no repetirla). */
  const siguiente = useCallback(() => {
    const { inmuebles, seleccionado } = estadoRef.current;
    void iniciarEn(inmuebles, seleccionado?.id ?? null);
  }, [iniciarEn]);

  return { ...estado, responder, saltar, anterior, siguiente };
}
