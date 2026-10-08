import { useCallback, useEffect, useState } from 'react';
import { aplicarTema, guardarTema, leerTemaGuardado, type Tema } from '../../app/tema';
import { observarTemaSistema } from '../../platform/ventana';

const ORDEN: Tema[] = ['sistema', 'claro', 'oscuro'];

// Tema global: persiste la elección y reacciona al cambio del sistema
// cuando el modo es "sistema".
export function useTema() {
  const [tema, setTemaState] = useState<Tema>(() => leerTemaGuardado());

  useEffect(() => {
    aplicarTema(tema);
  }, [tema]);

  useEffect(() => {
    if (tema !== 'sistema') return;
    /* [08AA-23] Suscripción vía plataforma (sin `window` directo aquí). */
    return observarTemaSistema(() => aplicarTema('sistema'));
  }, [tema]);

  const setTema = useCallback((t: Tema) => {
    guardarTema(t);
    setTemaState(t);
  }, []);

  const ciclar = useCallback(() => {
    setTemaState((actual) => {
      const siguiente = ORDEN[(ORDEN.indexOf(actual) + 1) % ORDEN.length];
      guardarTema(siguiente);
      return siguiente;
    });
  }, []);

  return { tema, setTema, ciclar };
}
