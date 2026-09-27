import { useCallback, useEffect, useRef, useState } from 'react';
import { listarRemoto } from '../../data/inmuebles/api';
import {
  buscarPendiente,
  guardarColumnaAsk,
  guardarFicha,
  guardarUbicacionAsk,
} from '../../data/inmuebles/ficha-ask';
import { assertNunca, extrasTrasRespuesta, marcaNoSePaso, mazclarPasos, pasosPara } from '../../domain/pasos-ask';
import { NO_SE, type ExtrasAsk } from '../../domain/ficha-ask';
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
   * cada vez que se pide otra. Sin lista visible: solo preguntas. */
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
     * (se salta o se corrige el valor). */
    if (valor === null && paso.kind !== 'ficha') return;
    setEstado((e) => ({ ...e, guardando: true, error: null }));
    /* «No lo sé» en pasos de columna (279A-7, todas las preguntas lo
     * llevan): la columna no lo puede guardar, así que se guarda la marca
     * `*_nose` en la ficha y el dato queda vacío. */
    const guardarMarcaNoSe = async (): Promise<void> => {
      const marca = marcaNoSePaso(paso);
      if (!marca) {
        setEstado((e) => ({ ...e, guardando: false }));
        return;
      }
      const guardada = await guardarFicha(seleccionado.id, {
        extras: { ...ficha.extras, [marca]: NO_SE },
        precioMinimo: ficha.precioMinimo,
      });
      setEstado(trasFicha(guardada));
    };
    /* Al responder de verdad se borra la marca «no sé» en local (el
     * backend la borra en BD al mismo tiempo: `update`/`set_ficha`). */
    const sinMarca = (extras: ExtrasAsk): ExtrasAsk => {
      const marca = marcaNoSePaso(paso);
      if (!marca || extras[marca] === undefined) return extras;
      const resto = { ...extras };
      delete resto[marca];
      return resto;
    };
    /* Guardado por tipo de paso (ramas exhaustivas: un `kind` nuevo
     * falla en compilación aquí, en el render y en `pasoRespondido`). */
    try {
      if (paso.kind === 'ficha') {
        if (esUbicacion(valor)) setEstado((e) => ({ ...e, guardando: false }));
        else {
          const guardada = await guardarFicha(seleccionado.id, extrasTrasRespuesta(ficha, paso.pregunta, valor));
          setEstado(trasFicha(guardada));
        }
      } else if (paso.kind === 'ubicacion') {
        if (valor === NO_SE) await guardarMarcaNoSe();
        else if (!esUbicacion(valor)) setEstado((e) => ({ ...e, guardando: false }));
        else {
          const guardado = await guardarUbicacionAsk(seleccionado, valor);
          setEstado((e) => {
            const base = trasInmueble(guardado)(e);
            return { ...base, ficha: { ...base.ficha, extras: sinMarca(base.ficha.extras) } };
          });
        }
      } else if (paso.kind === 'columna') {
        if (valor === NO_SE) await guardarMarcaNoSe();
        else if (typeof valor !== 'number' || !Number.isFinite(valor) || valor < 0) {
          setEstado((e) => ({ ...e, guardando: false }));
        } else {
          const guardado = await guardarColumnaAsk(seleccionado, paso.columna, valor);
          setEstado((e) => {
            const base = trasInmueble(guardado)(e);
            return { ...base, ficha: { ...base.ficha, extras: sinMarca(base.ficha.extras) } };
          });
        }
      } else {
        assertNunca(paso);
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
