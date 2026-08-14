/* 138A-5 — Debounce de regeneración en tiempo real del Constructor de mundo.
 * Agrupa cambios rápidos de controles (slider/input/select) en UNA
 * regeneración tras `delayMs`; la última opción enviada gana. No depende de
 * Three/DOM y se puede testear con timers fake. */

import type { TerrainOptions } from '../../../game-core';

export interface DebouncedRegenerator {
  /** Programa una regeneración con las últimas opciones (reemplaza la pendiente). */
  readonly schedule: (options: TerrainOptions) => void;
  /** Cancela la regeneración pendiente sin disparar el callback. */
  readonly cancel: () => void;
  /** Cancela y marca el objeto como inservible (teardown). */
  readonly dispose: () => void;
}

export function createDebouncedRegenerator(
  delayMs: number,
  regenerate: (options: TerrainOptions) => void,
): DebouncedRegenerator {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let latest: TerrainOptions | null = null;
  let disposed = false;

  const run = (): void => {
    timer = null;
    const options = latest;
    latest = null;
    if (options !== null) regenerate(options);
  };

  const cancel = (): void => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
    latest = null;
  };

  return {
    schedule(options) {
      if (disposed) return;
      latest = options;
      if (timer !== null) clearTimeout(timer);
      timer = setTimeout(run, Math.max(0, delayMs));
    },
    cancel,
    dispose() {
      cancel();
      disposed = true;
    },
  };
}
