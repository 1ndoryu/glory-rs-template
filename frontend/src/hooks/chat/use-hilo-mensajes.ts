import { useEffect, useRef, useState, type UIEvent } from 'react';
import type { MensajeServidor } from '@/data/chat/cliente-chat';

/* [08AA-32] Lógica del hilo (regla 8 / componente-sin-hook): estado del
 * borrador, refs de scroll, efecto post-commit de scroll/paginación y
 * handlers de carga y envío. El componente (`hilo-mensajes.tsx`) queda con
 * solo JSX + helpers puros de presentación. */

export function useHiloMensajes(params: {
  hilo: MensajeServidor[];
  hayMas: boolean;
  cargandoMas: boolean;
  alCargarAnteriores: () => Promise<void>;
  alResponder: (texto: string) => Promise<boolean>;
}) {
  const { hilo, hayMas, cargandoMas, alCargarAnteriores, alResponder } = params;
  const [texto, setTexto] = useState('');
  const cajaRef = useRef<HTMLDivElement>(null);
  const previoRef = useRef(0);
  /* [309A-3] Distancia al fondo a restaurar tras anteponer página: se
   * consume en el efecto post-commit (único punto con DOM ya crecido;
   * medir/ajustar desde el handler async pierde contra el render). */
  const pendienteRef = useRef<number | null>(null);
  /* Candado local: `cargandoMas` tarda un render en propagarse y el scroll
   * dispara varias veces en la misma subida. */
  const pidiendoRef = useRef(false);

  /* [289A-9] Orden normal: el hilo crece hacia abajo; acompaña al fondo
   * solo si ya estabas cerca de él (no roba el scroll al releer arriba).
   * [309A-3] Al abrir (vacío → con mensajes) salta al fondo: lo nuevo es
   * lo último; sin esto la página abría arriba en lo más viejo. Tras
   * anteponer restaura la distancia guardada en `pendienteRef`, pero solo
   * cuando el hilo creció y la carga terminó: el poll fusiona sin crecer
   * y no debe robar la compensación pendiente. */
  useEffect(() => {
    const caja = cajaRef.current;
    if (!caja) return;
    if (pendienteRef.current !== null && !cargandoMas) {
      if (hilo.length > previoRef.current) {
        caja.scrollTop = Math.max(0, caja.scrollHeight - pendienteRef.current);
      }
      pendienteRef.current = null;
    } else if (pendienteRef.current === null && previoRef.current === 0 && hilo.length > 0) {
      caja.scrollTop = caja.scrollHeight;
    } else if (pendienteRef.current === null) {
      const cercaFondo = caja.scrollHeight - caja.scrollTop - caja.clientHeight < 160;
      if (cercaFondo) caja.scrollTop = caja.scrollHeight;
    }
    previoRef.current = hilo.length;
  }, [hilo, cargandoMas]);

  /* Subir hasta arriba carga la página anterior; el efecto post-commit
   * compensa el crecimiento hacia arriba con la distancia guardada aquí. */
  async function cargarPagina(caja: HTMLDivElement): Promise<void> {
    if (!hayMas || pidiendoRef.current) return;
    pidiendoRef.current = true;
    try {
      pendienteRef.current = caja.scrollHeight - caja.scrollTop;
      await alCargarAnteriores();
    } finally {
      pidiendoRef.current = false;
    }
  }

  function alSubir(e: UIEvent<HTMLDivElement>): void {
    const caja = e.currentTarget;
    if (caja.scrollTop <= 120) void cargarPagina(caja);
  }

  async function enviar(): Promise<void> {
    if (await alResponder(texto)) setTexto('');
  }

  return { texto, setTexto, cajaRef, alSubir, cargarPagina, enviar };
}
