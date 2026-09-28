// Sesiones WhatsApp (289A-1): vincular los números desde la consola.
// Muestra estado por sesión (A/B) y el QR pendiente para escanearlo con el
// teléfono de ese número (WhatsApp > Dispositivos vinculados > Vincular).
// El QR caduca en ~1 min: el botón Actualizar trae el vigente. Sin polling
// agresivo: recarga al montar, cada 20 s solo si falta alguna por vincular
// y a mano con el botón.

import { useCallback, useEffect, useState } from 'react';
import { descargarQr, sesionesWhatsapp, type SesionWhatsapp } from '../../data/chat/cliente-duena';
import { ErrorApi } from '../../data/inmuebles/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado de sesiones WhatsApp.';
}

const ETIQUETA_ESTADO: Record<string, string> = {
  iniciando: 'Iniciando…',
  esperando_qr: 'Esperando QR',
  abierta: 'Vinculada',
  cerrada: 'Desconectada',
};

export function SesionesWhatsapp() {
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
    const t = window.setInterval(() => void recargar(), 20000);
    return () => window.clearInterval(t);
  }, [sesiones, recargar]);

  useEffect(
    () => () => {
      for (const u of Object.values(qrPorVia)) URL.revokeObjectURL(u);
    },
    [qrPorVia],
  );

  return (
    <div>
      {error && (
        <p className="mb-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error} (¿gateway Baileys arrancado? ver `gateway/iniciar-pruebas.ps1`)
        </p>
      )}
      <div className="mb-3 flex items-center gap-2">
        <p className="text-sm text-muted-foreground">
          Escanea cada QR con el teléfono de ese número para vincular la sesión.
        </p>
        <Button className="ml-auto" variant="outline" size="sm" onClick={() => void recargar()}>
          Actualizar
        </Button>
      </div>
      {!sesiones ? (
        <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
          Cargando sesiones…
        </p>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {sesiones.map((s) => (
            <div key={s.via} className="rounded-lg border px-4 py-3">
              <div className="flex items-center gap-2">
                <span className="text-sm font-semibold">
                  Sesión {s.nombre} · {s.numero}
                </span>
                <Badge
                  className="ml-auto"
                  variant={s.estado === 'abierta' ? 'default' : 'secondary'}
                >
                  {ETIQUETA_ESTADO[s.estado] ?? s.estado}
                </Badge>
              </div>
              {s.estado === 'esperando_qr' &&
                (qrPorVia[s.via] ? (
                  <img
                    src={qrPorVia[s.via]}
                    alt={`QR para vincular la sesión ${s.nombre}`}
                    className="mx-auto mt-3 h-64 w-64"
                  />
                ) : (
                  <p className="mt-3 text-sm text-muted-foreground">
                    Generando QR… pulsa Actualizar si tarda.
                  </p>
                ))}
              {s.estado === 'abierta' && (
                <p className="mt-3 text-sm text-emerald-700">
                  Vinculada: los mensajes a este número ya llegan al modo que le toca.
                </p>
              )}
              {(s.estado === 'iniciando' || s.estado === 'cerrada') && (
                <p className="mt-3 text-sm text-muted-foreground">
                  Sin QR todavía (gateway arrancando o sesión caída). Pulsa Actualizar.
                </p>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
