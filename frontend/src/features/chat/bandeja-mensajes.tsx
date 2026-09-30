// Bandeja (169A-5): lista de sesiones con último mensaje y avisos +
// hilo abierto. En móvil alterna lista/hilo; en escritorio lado a lado.

import { ArrowLeft } from 'lucide-react';
import { useBandejaChat } from '../../hooks/chat/use-bandeja-chat';
import type { EstadoSesionChat } from '../../data/chat/cliente-admin';
import { HiloMensajes } from './hilo-mensajes';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

const FILTROS: { id: 'todas' | EstadoSesionChat; texto: string }[] = [
  { id: 'todas', texto: 'Todas' },
  { id: 'open', texto: 'Abiertas' },
  { id: 'escalated', texto: 'Escaladas' },
  { id: 'closed', texto: 'Cerradas' },
];

export function BandejaMensajes() {
  const b = useBandejaChat();

  return (
    <div>
      <div className="mb-3 flex flex-wrap gap-2">
        {FILTROS.map((f) => (
          <Button
            key={f.id}
            variant={b.filtro === f.id ? 'default' : 'outline'}
            size="sm"
            onClick={() => b.ponerFiltro(f.id)}
          >
            {f.texto}
          </Button>
        ))}
      </div>
      {b.error && (
        <p className="mb-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {b.error}
        </p>
      )}
      {b.cargando ? (
        <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
          Cargando conversaciones…
        </p>
      ) : (
        <div className="flex flex-col gap-3 lg:flex-row">
          {/* Lista (en móvil se oculta al abrir un hilo). */}
          <ul className={cn('space-y-2 lg:w-80 lg:shrink-0', b.seleccionada && 'hidden lg:block')}>
            {b.sesiones.length === 0 && (
              <li className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
                Sin conversaciones.
              </li>
            )}
            {b.sesiones.map((s) => (
              <li key={s.id}>
                <button
                  type="button"
                  onClick={() => b.seleccionar(s.id)}
                  className={cn(
                    'w-full rounded-md border px-3 py-2 text-left text-sm hover:bg-muted',
                    b.seleccionada?.id === s.id && 'border-primary bg-muted',
                  )}
                >
                  <span className="flex items-center gap-2">
                    <span className="font-medium">{s.visitor_name ?? s.contact ?? s.telefono ?? s.id.slice(0, 8)}</span>
                    {s.status !== 'open' && (
                      <Badge variant={s.status === 'escalated' ? 'destructive' : 'secondary'} className="text-[10px]">
                        {s.status}
                      </Badge>
                    )}
                    {(s.alertas ?? 0) > 0 && (
                      <Badge variant="destructive" className="ml-auto text-[10px]">
                        {s.alertas} aviso{(s.alertas ?? 0) === 1 ? '' : 's'}
                      </Badge>
                    )}
                    {!s.ai_enabled && (
                      <Badge variant="outline" className="ml-auto text-[10px]">
                        manual
                      </Badge>
                    )}
                  </span>
                  {/* [289A-2] Número del cliente: identifica la conversación de WhatsApp. */}
                  {s.telefono && <span className="mt-0.5 block text-xs font-medium text-muted-foreground">{s.telefono}</span>}
                  {s.last_body && <span className="mt-0.5 block truncate text-xs text-muted-foreground">{s.last_body}</span>}
                </button>
              </li>
            ))}
          </ul>
          {/* Hilo (en móvil con botón volver). [309A-2] Altura acotada: sin
              `max-h` la página crecía sin fin con hilos largos; el scroll vive
              en la caja de mensajes (`overflow-y-auto` en `HiloMensajes`). */}
          <div className={cn('min-h-[50dvh] max-h-[75dvh] flex-1 flex-col overflow-hidden rounded-md border', b.seleccionada ? 'flex' : 'hidden lg:flex')}>
            {!b.seleccionada ? (
              <p className="m-auto px-6 py-16 text-center text-sm text-muted-foreground">Elige una conversación.</p>
            ) : (
              <>
                <div className="border-b px-2 py-1 lg:hidden">
                  <Button variant="ghost" size="sm" onClick={() => b.seleccionar(null)}>
                    <ArrowLeft className="h-3.5 w-3.5" /> Bandeja
                  </Button>
                </div>
                <HiloMensajes
                  sesion={b.seleccionada}
                  hilo={b.hilo}
                  hayMas={b.hayMas}
                  cargandoMas={b.cargandoMas}
                  alCargarAnteriores={b.cargarAnteriores}
                  respondiendo={b.respondiendo}
                  alResponder={b.responder}
                  alCambiar={b.cambiarSesion}
                />
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
