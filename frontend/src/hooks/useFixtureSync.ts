/* [154A-2] Hook para gestionar la sincronización de fixtures de contenido desde el panel admin.
 * Carga el status actual de fixtures y expone la función sync para disparar la sincronización. */

import {useState, useEffect, useCallback} from 'react';
import {apiGetFixtureStatus, apiTriggerFixtureSync} from '../api/admin-fixtures';
import type {FixtureStatusResponse, FixtureSyncResult} from '../api/admin-fixtures';

interface UseFixtureSyncReturn {
    statusData: FixtureStatusResponse | null;
    syncResult: FixtureSyncResult | null;
    cargando: boolean;
    error: string | null;
    sync: () => Promise<void>;
}

export function useFixtureSync(): UseFixtureSyncReturn {
    const [statusData, setStatusData] = useState<FixtureStatusResponse | null>(null);
    const [syncResult, setSyncResult] = useState<FixtureSyncResult | null>(null);
    const [cargando, setCargando] = useState(false);
    const [error, setError] = useState<string | null>(null);

    /* Cargar status inicial de fixtures */
    useEffect(() => {
        const ctrl = new AbortController();
        apiGetFixtureStatus()
            .then(data => {
                if (!ctrl.signal.aborted) setStatusData(data);
            })
            .catch(() => {
                if (!ctrl.signal.aborted) {
                    /* Si la tabla _glory_fixtures no existe aún, simplemente no mostramos status */
                }
            });
        return () => ctrl.abort();
    }, []);

    const sync = useCallback(async () => {
        setCargando(true);
        setError(null);
        setSyncResult(null);
        try {
            const result = await apiTriggerFixtureSync();
            setSyncResult(result);
            /* Recargar status después del sync para mostrar conteos actualizados */
            const status = await apiGetFixtureStatus();
            setStatusData(status);
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : 'Error al sincronizar fixtures';
            setError(msg);
        } finally {
            setCargando(false);
        }
    }, []);

    return {statusData, syncResult, cargando, error, sync};
}
