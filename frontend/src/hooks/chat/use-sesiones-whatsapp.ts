import { useCallback, useEffect, useState } from 'react';
import { descargarQr, sesionesWhatsapp, type SesionWhatsapp } from '@/data/chat/cliente-duena';
import { ErrorApi } from '@/data/inmuebles/api';
import { repetirCada } from '@/platform/ventana';

/* [08AA-32] Estado y recarga de sesiones WhatsApp + QRs (regla 8 /
 * componente-sin-hook): lista, QR por vía (con revocación de object URLs),
 * error y recarga con reintento suave. El componente
 * (`sesiones-whatsapp.tsx`) queda con solo JSX + etiqueta estática. */

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado de sesiones WhatsApp.';
}

export function useSesionesWhatsapp() {
  const [sesiones, setSesiones] = useState<SesionWhatsapp[] | null>(null);
  const [qrPorVia, setQrPorVia] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  const recargar = useCallback(async () => {
    try {
      const lista = await sesionesWhatsapp();
      setSesiones(lista);
      setError(null);
      const qr: Record<string, string> = {};
      for (const s of lista) {
        if (s.estado === 'esperando_qr') {
          const url = await descargarQr(s.via);
          if (url) qr[s.via] = url;
        }
      }
      setQrPorVia((prev) => {
        for (const u of Object.values(prev)) URL.revokeObjectURL(u);
        return qr;
      });
    } catch (e) {
      setError(mensajeError(e));
    }
  }, []);

  useEffect(() => {
    void recargar();
  }, [recargar]);

  /* Reintento suave mientras falte alguna por vincular; al vincularse
   * todas, el intervalo se detiene solo. */
  useEffect(() => {
    if (!sesiones || sesiones.every((s) => s.estado === 'abierta')) return;
    /* [08AA-22] Intervalo vía plataforma (sin `window` directo aquí). */
    return repetirCada(20000, () => void recargar());
  }, [sesiones, recargar]);

  useEffect(
    () => () => {
      for (const u of Object.values(qrPorVia)) URL.revokeObjectURL(u);
    },
    [qrPorVia],
  );

  return { sesiones, qrPorVia, error, recargar };
}
