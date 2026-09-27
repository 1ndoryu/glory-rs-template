import { useEffect, useState } from 'react';
import type { ExtrasAsk } from '@/domain/ficha-ask';
import type { Inmueble } from '@/domain/inmueble';

/* Ficha /ask en el modal admin (279A-3 F2): estado de `extras` + mínimo y
 * fusión al guardar. Todo lo que /ask pregunta se edita aquí y pisa lo
 * guardado (vacío = sin responder y borra la clave). */

export interface FichaEdicion {
  extras: ExtrasAsk;
  minimo: string;
}

export function useFichaInmueble(abierto: boolean, editando: Inmueble | null) {
  const [extras, setExtras] = useState<ExtrasAsk>({});
  const [minimo, setMinimo] = useState('');

  useEffect(() => {
    if (abierto) {
      setExtras({ ...(editando?.extras ?? {}) });
      setMinimo(editando?.precioMinimo != null ? String(editando.precioMinimo) : '');
    }
  }, [abierto, editando]);

  const cambiarExtra = (clave: string, valor: string | number | boolean | null) => {
    setExtras((antes) => {
      const copia = { ...antes };
      if (valor === null || valor === '') delete copia[clave];
      else copia[clave] = valor;
      return copia;
    });
  };

  /* Fusión al guardar: mínimo a número o null (privado, columna aparte). */
  const conFicha = (inmueble: Inmueble): Inmueble => {
    const n = Number(minimo.replace(',', '.'));
    return { ...inmueble, extras: { ...extras }, precioMinimo: minimo.trim() && Number.isFinite(n) && n >= 0 ? n : null };
  };

  return { extras, minimo, setMinimo, cambiarExtra, conFicha };
}
