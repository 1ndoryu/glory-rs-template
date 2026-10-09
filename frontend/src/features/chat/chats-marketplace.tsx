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

/* [08AA-31] Lado por marca de texto: lo propio viaja como `Tú:`/`Tu:`/`You:`
 * (backend 08AA-29); lo demás es del cliente (viaja sin etiqueta). Sin esta
 * separación el excerpt se veía pegado en un solo bloque (hilo angelv). */
function esLadoPropio(linea: string): boolean {
  const marca = linea.trim().toLowerCase();
  return marca.startsWith('tú:') || marca.startsWith('tu:') || marca.startsWith('you:');
}

/* [08AA-31] Quita la marca de lado (`Tú: msg` → `msg`): el Badge ya dice el
 * lado, repetirlo en el texto duplica. Solo se usa con `esLadoPropio`. */
function textoSinMarca(linea: string): string {
  const i = linea.indexOf(':');
  const resto = i < 0 ? '' : linea.slice(i + 1).trim();
  return resto === '' ? linea.trim() : resto;
}

export function ChatsMarketplace() {
  const { lista, seleccion, error, recargar, elegir, limpiar } = useChatsMarketplace();

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
        <div className="mt-2 flex gap-2">
          <Button variant="outline" size="sm" onClick={() => void recargar()}>
            Recargar
          </Button>
          {/* [08AA-39] Limpieza total al lado de Recargar, con confirmación. */}
          <Button variant="destructive" size="sm" onClick={() => void limpiar()}>
            Limpiar
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
              {/* [08AA-5] La conversación se muestra una sola vez: las filas
               * vienen recientes-primero y cada snapshot trae el hilo
               * completo, así que solo la primera pinta su excerpt.
               * [08AA-31] Como chat real: una línea por mensaje con su Badge
               * (`Tú` lo propio, `Cliente` lo de ella/él) en vez del bloque
               * pegado en un solo `span`. */}
              {i === 0 && f.excerpt_texto && (
                <span className="block space-y-1 border-l-2 border-primary/40 pl-2 text-muted-foreground">
                  {f.excerpt_texto.split('\n').map((linea, j) => {
                    const texto = linea.trim();
                    if (texto === '') return null;
                    const propia = esLadoPropio(texto);
                    return (
                      <span key={j} className="flex flex-wrap items-center gap-1">
                        <Badge variant={propia ? 'default' : 'secondary'} className="text-[10px]">
                          {propia ? 'Tú' : 'Cliente'}
                        </Badge>
                        <span className="min-w-0 flex-1 break-words">
                          {propia ? textoSinMarca(texto) : texto}
                        </span>
                      </span>
                    );
                  })}
                </span>
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
