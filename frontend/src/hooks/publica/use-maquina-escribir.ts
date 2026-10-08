import { useEffect, useState } from 'react';
import { repetirCada, prefiereMovimientoReducido } from '../../platform/ventana';

/* Palabras que rotan en el titular con efecto máquina de escribir. */
export const PALABRAS_ROTATIVAS = ['hogar', 'apart.', 'local', 'terreno'] as const;

/* Ritmo del efecto: una letra por tick, pausa larga con la palabra
 * completa y pausa corta al cambiar de palabra. */
const TICK_MS = 90;
const PAUSA_COMPLETA_TICKS = 18;
const PAUSA_CAMBIO_TICKS = 3;

/* Máquina de escribir: muestra la palabra, la sostiene, la borra y pasa
 * a la siguiente en bucle. Un solo intervalo con limpieza; con
 * movimiento reducido devuelve la primera palabra sin animar. */
export function useMaquinaEscribir() {
  const [salida, setSalida] = useState<string>(PALABRAS_ROTATIVAS[0]);
  useEffect(() => {
    if (prefiereMovimientoReducido()) return;
    let indice = 0;
    let posicion = PALABRAS_ROTATIVAS[0].length;
    let borrando = false;
    let espera = PAUSA_COMPLETA_TICKS;
    /* [08AA-22] Intervalo vía plataforma (sin `window` directo aquí). */
    return repetirCada(TICK_MS, () => {
      if (espera > 0) {
        espera -= 1;
        return;
      }
      let actual = PALABRAS_ROTATIVAS[indice];
      if (borrando) {
        posicion -= 1;
        if (posicion <= 0) {
          indice = (indice + 1) % PALABRAS_ROTATIVAS.length;
          actual = PALABRAS_ROTATIVAS[indice];
          posicion = 0;
          borrando = false;
          espera = PAUSA_CAMBIO_TICKS;
        }
      } else {
        posicion += 1;
        if (posicion >= actual.length) {
          posicion = actual.length;
          borrando = true;
          espera = PAUSA_COMPLETA_TICKS;
        }
      }
      setSalida(actual.slice(0, posicion));
    });
  }, []);
  return salida;
}
