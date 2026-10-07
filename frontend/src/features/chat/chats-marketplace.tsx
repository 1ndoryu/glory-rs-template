// Panel por chat del asistente Marketplace (07AA-7): una fila por hilo
// (conversación + borradores + usos + vigencia) y detalle con la foto de
// la conversación (`excerpt_texto`) al lado del texto guardado.

import { useChatsMarketplace } from './use-chats-marketplace';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

function fechaCorta(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString('es-VE', { dateStyle: 'short', timeStyle: 'short' });
}

export function ChatsMarketplace() {
  const { lista, seleccion, error, recargar, elegir } = useChatsMarketplace();

  return (
    <div className="grid gap-4 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
      {error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive md:col-span-2">
          {error}
        </p>
      )}
      <section>
        <h3 className="mb-2 text-sm font-medium">Chats con borradores ({lista.length})</h3>
        <ul className="space-y-2">
          {lista.map((c) => (
            <li key={c.thread_id}>
              <button
                type="button"
                onClick={() => void elegir(c.thread_id)}
                className={`w-full rounded-md border px-3 py-2 text-left text-xs hover:bg-muted/50 ${
                  seleccion?.hilo === c.thread_id ? 'border-primary' : ''
                }`}
              >
                <span className="block break-all font-medium">{c.thread_id}</span>
                <span className="mt-0.5 block text-muted-foreground">
                  {c.borradores} borrador{c.borradores === 1 ? '' : 'es'} · {c.usos} uso{c.usos === 1 ? '' : 's'} ·{' '}
                  {fechaCorta(c.ultimo)}
                </span>
              </button>
            </li>
          ))}
          {lista.length === 0 && (
            <li className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
              Sin chats todavía. Aparecen cuando el puente genera borradores.
            </li>
          )}
        </ul>
        <div className="mt-2">
          <Button variant="outline" size="sm" onClick={() => void recargar()}>
            Recargar
          </Button>
        </div>
      </section>
      <section>
        <h3 className="mb-2 text-sm font-medium">
          {seleccion ? `Hilo: ${seleccion.hilo}` : 'Elige un chat para ver sus borradores'}
        </h3>
        {seleccion?.cargando && <p className="text-sm text-muted-foreground">Cargando borradores…</p>}
        <ul className="space-y-2">
          {(seleccion?.filas ?? []).map((f, i) => (
            <li key={`${seleccion?.hilo}-${i}`} className="rounded-md border px-3 py-2 text-xs">
              {f.excerpt_texto && (
                <span className="block border-l-2 border-primary/40 pl-2 text-muted-foreground">{f.excerpt_texto}</span>
              )}
              <span className="mt-1 block">{f.respuesta}</span>
              <span className="mt-1 flex flex-wrap items-center gap-2 text-muted-foreground">
                {f.corregida && (
                  <Badge variant="secondary" className="text-[10px]">
                    corregida por la dueña
                  </Badge>
                )}
                <span>
                  {f.usos} uso{f.usos === 1 ? '' : 's'} · vigente hasta {fechaCorta(f.valida_hasta)}
                </span>
              </span>
            </li>
          ))}
        </ul>
        {seleccion && !seleccion.cargando && seleccion.filas.length === 0 && (
          <p className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
            Este hilo ya no tiene borradores vigentes.
          </p>
        )}
      </section>
    </div>
  );
}
