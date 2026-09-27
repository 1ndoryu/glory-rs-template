/* [044A-38 Fase 6] Hook para el panel de entregables.
 * [074A-53] Modal de entrega con notas opcionales y adjuntos opcionales. */

import { useState, useCallback } from 'react';
import { useDeliverables } from './useDeliverables';
/* [259A-5] Creacion de enlace de descarga via platform/dom (boundary sentinel). */
import {crearElemento} from '../platform/dom';
import type { PhaseDeliverable } from '../api/deliverables';

export function useEntregablesPanel(orderId: string, phaseNumber: number) {
    const {deliverables, cargando, entregarFase, entregando, descargar} = useDeliverables(orderId, phaseNumber);
    const [error, setError] = useState('');
    const [modalAbierto, setModalAbierto] = useState(false);
    const [notas, setNotas] = useState('');
    const [archivos, setArchivos] = useState<File[]>([]);

    const abrirModal = useCallback(() => {
        setNotas('');
        setArchivos([]);
        setError('');
        setModalAbierto(true);
    }, []);

    const cerrarModal = useCallback(() => {
        setModalAbierto(false);
    }, []);

    /* [074A-53] Entregar con notas y archivos opcionales */
    const handleDeliver = useCallback(async () => {
        try {
            setError('');
            await entregarFase({
                files: archivos.length > 0 ? archivos : undefined,
                notes: notas.trim() || undefined,
            });
            setModalAbierto(false);
        } catch {
            setError('Error al entregar. Intenta de nuevo.');
        }
    }, [entregarFase, archivos, notas]);

    const agregarArchivos = useCallback((nuevos: FileList | null) => {
        if (nuevos) setArchivos(prev => [...prev, ...Array.from(nuevos)]);
    }, []);

    const quitarArchivo = useCallback((index: number) => {
        setArchivos(prev => prev.filter((_, i) => i !== index));
    }, []);

    const handleDownload = useCallback(async (deliverableId: string, fileName: string) => {
        try {
            const blobUrl = await descargar(deliverableId);
            const a = crearElemento('a');
            a.href = blobUrl;
            a.download = fileName;
            a.click();
            URL.revokeObjectURL(blobUrl);
        } catch {
            setError('Error al descargar archivo');
        }
    }, [descargar]);

    /* Agrupar entregables por revisión (descendente) */
    const revisionGroups = deliverables.reduce<Record<number, PhaseDeliverable[]>>((acc, d) => {
        if (!acc[d.revision_number]) acc[d.revision_number] = [];
        acc[d.revision_number].push(d);
        return acc;
    }, {});
    const sortedRevisions = Object.keys(revisionGroups).map(Number).sort((a, b) => b - a);

    return {
        error,
        cargando,
        entregando,
        handleDeliver,
        handleDownload,
        sortedRevisions,
        revisionGroups,
        modalAbierto,
        abrirModal,
        cerrarModal,
        notas,
        setNotas,
        archivos,
        agregarArchivos,
        quitarArchivo,
    };
}
