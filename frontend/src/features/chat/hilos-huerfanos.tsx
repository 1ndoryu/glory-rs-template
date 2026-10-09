// Vista de hilos huérfanos (09AA-19 F7c): chats sin `aviso_conocido` con
// vínculo manual a inmueble. Reutiliza los datos de `useChatsMarketplace`
// (vía props desde `ChatsMarketplace`, sin segundo fetch): la lista y el
// detalle del hilo; el catálogo y la acción viven en `useVincularHilo`.

import { useState } from 'react';
import { normalizarMarketplaceId } from '@/domain/inmueble';
import { useVincularHilo } from '@/hooks/chat/use-vincular-hilo';
import type { ChatResumen } from '../../data/chat/marketplace-chats';
import type { DetalleChat } from './use-chats-marketplace';
import { VincularHilo } from './vincular-hilo';

/* El hilo es `nombre|aviso`: si el aviso son dígitos (o URL) se sugiere
 * como ID; si es un título aproximado no hay nada que sugerir. */
function avisoSugeridoDe(threadId: string): string {
  const parte = threadId.split('|')[1]?.trim() ?? '';
  return normalizarMarketplaceId(parte);
}

export function HilosHuerfanos({
  hilos,
  seleccion,
  alElegir,
}: {
  hilos: ChatResumen[];
  seleccion: DetalleChat | null;
  alElegir: (hilo: string) => void;
}) {
  const [hiloElegido, setHiloElegido] = useState<string | null>(null);
  const { inmuebles, cargando, error, vinculando, aviso, vincular } = useVincularHilo();

  /* El backend aún no envía `aviso_conocido` (llega con F7a): sin dato se
   * muestran todos como pendientes de revisión, con aviso. */
  const huerfanos = hilos.filter((h) => h.aviso_conocido !== true);
  const sinDato = hilos.length > 0 && hilos.every((h) => h.aviso_conocido === undefined);
  const actual = huerfanos.find((h) => h.thread_id === hiloElegido) ?? null;

  function elegir(threadId: string): void {
    setHiloElegido(threadId);
    void alElegir(threadId);
  }

  return (
    <div className="space-y-2">
      <p className="text-xs text-muted-foreground">
        Hilos sin ficha exacta ({huerfanos.length}): vincúlalos a mano a su inmueble.
        {sinDato && ' El backend aún no confirma el vínculo: se listan todos.'}
      </p>
      {huerfanos.length === 0 ? (
        <p className="text-sm text-muted-foreground">Sin hilos huérfanos: todo hilo tiene su ficha.</p>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          <ul className="space-y-1">
            {huerfanos.map((h) => (
              <li key={h.thread_id}>
                <button
                  type="button"
                  onClick={() => elegir(h.thread_id)}
                  className={`w-full rounded-md border px-3 py-2 text-left text-xs hover:bg-muted ${hiloElegido === h.thread_id ? 'border-primary' : ''}`}
                >
                  <span className="block break-all font-medium">{h.thread_id}</span>
                  <span className="text-muted-foreground">
                    {h.borradores} borradores · {h.usos} usos
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div className="space-y-2">
            {!actual ? (
              <p className="text-sm text-muted-foreground">Elige un hilo para vincularlo.</p>
            ) : (
              <>
                {seleccion?.hilo === actual.thread_id && !seleccion.cargando && (
                  <ul className="space-y-1">
                    {seleccion.filas.slice(0, 3).map((f, i) => (
                      <li key={`${actual.thread_id}-${i}`} className="rounded-md border px-3 py-2 text-xs">
                        {f.excerpt_texto.split('\n')[0]}
                      </li>
                    ))}
                  </ul>
                )}
                {cargando ? (
                  <p className="text-sm text-muted-foreground">Cargando inmuebles…</p>
                ) : (
                  <VincularHilo
                    key={actual.thread_id}
                    inmuebles={inmuebles}
                    sugerido={avisoSugeridoDe(actual.thread_id)}
                    vinculando={vinculando}
                    alVincular={(inmuebleId, marketplaceId) => void vincular(inmuebleId, marketplaceId)}
                  />
                )}
              </>
            )}
            {aviso && <p className="text-xs text-emerald-700">{aviso}</p>}
            {error && <p className="text-xs text-destructive">{error}</p>}
          </div>
        </div>
      )}
    </div>
  );
}
