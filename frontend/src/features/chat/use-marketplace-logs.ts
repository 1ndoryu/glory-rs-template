// Estado de la tab de Logs (09AA-5): eventos del puente con filtro por
// nivel y auto-refresh cada 5s (pausable). Sin PII en lo que se muestra.

import { useCallback, useEffect, useState } from 'react';
import { listarLogs, type LogEvento, type NivelLog } from '../../data/chat/marketplace-logs';
import { ErrorApi } from '../../data/inmuebles/api';

export type FiltroNivel = NivelLog | 'todos';

export function useMarketplaceLogs() {
  const [eventos, setEventos] = useState<LogEvento[]>([]);
  const [nivel, setNivel] = useState<FiltroNivel>('todos');
  const [pausado, setPausado] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const recargar = useCallback(
    async (ctrl?: AbortController) => {
      try {
        const lista = await listarLogs(
          nivel === 'todos' ? undefined : nivel,
        );
        if (!ctrl?.signal.aborted) {
          setEventos(lista);
          setError(null);
        }
      } catch (e: unknown) {
        if (ctrl?.signal.aborted) return;
        setError(e instanceof ErrorApi ? e.message : 'Fallo inesperado.');
      }
    },
    [nivel],
  );

  useEffect(() => {
    const ctrl = new AbortController();
    void recargar(ctrl);
    if (pausado) return () => ctrl.abort();
    const cada = window.setInterval(() => void recargar(ctrl), 5000);
    return () => {
      window.clearInterval(cada);
      ctrl.abort();
    };
  }, [recargar, pausado]);

  return { eventos, nivel, setNivel, pausado, setPausado, error, recargar: () => void recargar() };
}
