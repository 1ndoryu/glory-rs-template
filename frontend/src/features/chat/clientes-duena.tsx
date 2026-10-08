// Clientes de la dueña (279A-2 F5): buscar, alta manual, ficha comercial
// editable, hilos del cliente y «dime y lo envío» por WhatsApp.
// En móvil alterna lista/ficha; en escritorio lado a lado.

import { useState } from 'react';
import { useClientes } from '../../hooks/chat/use-clientes';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';

const CAMPOS_FICHA = ['nombre', 'interes', 'presupuesto', 'zona', 'notas'] as const;

export function ClientesDuena() {
  const c = useClientes();
  const sel = c.seleccionado;
  const [alta, setAlta] = useState({ nombre: '', telefono: '' });
  const [ficha, setFicha] = useState<Record<string, string>>({});
  /* F4-parcial: texto + pie de foto opcional (URL http(s)) en un solo objeto
   * (regla 8: max 3 `useState`). El backend valida y el worker la adjunta al
   * enviar. Audios sin transcripción: fuera de alcance (ver plan 279A-2). */
  const [envio, setEnvio] = useState({ texto: '', media: '' });
  function fichaDe(actual: NonNullable<typeof sel>): Record<string, string> {
    return Object.fromEntries(
      CAMPOS_FICHA.map((k) => [k, ficha[k] ?? (actual[k] as string | null) ?? '']),
    );
  }

  return (
    <div>
      {c.error && (
        <p className="mb-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {c.error}
        </p>
      )}
      {c.aviso && (
        <p className="mb-3 rounded-md border border-emerald-500/40 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-700">
          {c.aviso}
        </p>
      )}
      <div className="mb-3 flex flex-col gap-2 sm:flex-row">
        <Input
          placeholder="Buscar por nombre o teléfono…"
          value={c.busqueda}
          onChange={(e) => c.ponerBusqueda(e.target.value)}
          className="sm:max-w-xs"
        />
        <div className="flex flex-1 gap-2">
          <Input placeholder="Nombre (opcional)" value={alta.nombre} onChange={(e) => setAlta((a) => ({ ...a, nombre: e.target.value }))} />
          <Input placeholder="Teléfono" value={alta.telefono} onChange={(e) => setAlta((a) => ({ ...a, telefono: e.target.value }))} />
          <Button
            size="sm"
            disabled={!alta.telefono.trim()}
            onClick={() => {
              /* sentinel-disable-next-line promise-sin-catch -- [08AA-26] catch-interno: alta() captura en use-clientes.ts y expone c.error (renderizado arriba); la promesa nunca rechaza. */
              void c.alta(alta.nombre, alta.telefono).then(() => {
                setAlta({ nombre: '', telefono: '' });
              });
            }}
          >
            Alta
          </Button>
        </div>
      </div>
      {c.cargando ? (
        <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
          Cargando clientes…
        </p>
      ) : (
        <div className="flex flex-col gap-3 lg:flex-row">
          <ul className={cn('space-y-2 lg:w-80 lg:shrink-0', sel && 'hidden lg:block')}>
            {c.clientes.length === 0 && (
              <li className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
                Sin clientes.
              </li>
            )}
            {c.clientes.map((k) => (
              <li key={k.id}>
                <button
                  type="button"
                  onClick={() => {
                    c.elegir(k.id);
                    setFicha({});
                    setEnvio({ texto: '', media: '' });
                  }}
                  className={cn(
                    'w-full rounded-md border px-3 py-2 text-left text-sm hover:bg-muted',
                    sel?.id === k.id && 'border-primary bg-muted',
                  )}
                >
                  <span className="flex items-center gap-2">
                    <span className="font-medium">{k.nombre || k.telefono}</span>
                    <Badge variant="outline" className="text-[10px]">
                      {k.origen}
                    </Badge>
                    {(k.sesiones ?? 0) > 0 && (
                      <Badge variant="secondary" className="ml-auto text-[10px]">
                        {k.sesiones} hilo{(k.sesiones ?? 0) === 1 ? '' : 's'}
                      </Badge>
                    )}
                  </span>
                  <span className="mt-0.5 block truncate text-xs text-muted-foreground">
                    {[k.interes, k.presupuesto, k.zona].filter(Boolean).join(' · ') || k.telefono}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div className={cn('min-h-[40dvh] flex-1 flex-col gap-3 rounded-md border p-3', sel ? 'flex' : 'hidden lg:flex')}>
            {!sel ? (
              <p className="m-auto px-6 py-16 text-center text-sm text-muted-foreground">Elige un cliente.</p>
            ) : (
              <>
                <Button variant="ghost" size="sm" className="self-start lg:hidden" onClick={() => c.elegir(null)}>
                  ← Clientes
                </Button>
                <div className="grid gap-2 sm:grid-cols-2">
                  {CAMPOS_FICHA.map((k) => (
                    <label key={k} className="grid gap-1 text-xs text-muted-foreground">
                      {k}
                      <Input value={fichaDe(sel)[k]} onChange={(e) => setFicha((f) => ({ ...f, [k]: e.target.value }))} />
                    </label>
                  ))}
                </div>
                <div>
                  <Button
                    size="sm"
                    onClick={() => {
                      const actual = fichaDe(sel);
                      const cambio = Object.fromEntries(
                        CAMPOS_FICHA.map((k) => [k, actual[k].trim()]).filter(([, v]) => v),
                      );
                      void c.guardarFicha(sel.id, cambio);
                    }}
                  >
                    Guardar ficha
                  </Button>
                </div>
                <div className="space-y-1">
                  <p className="text-xs font-medium text-muted-foreground">Hilos ({c.sesiones.length})</p>
                  {c.sesiones.map((s) => (
                    <p key={s.session_id} className="truncate rounded border px-2 py-1 text-xs">
                      <Badge variant="outline" className="mr-1 text-[10px]">
                        {s.canal ?? '?'}
                      </Badge>
                      {s.last_body ?? 'sin mensajes'}
                    </p>
                  ))}
                </div>
                <div className="grid gap-2">
                  <Textarea
                    placeholder="«Dime y lo envío» por WhatsApp…"
                    value={envio.texto}
                    onChange={(e) => setEnvio((v) => ({ ...v, texto: e.target.value }))}
                    rows={2}
                  />
                  <div className="flex gap-2">
                    <Input
                      placeholder="URL de foto (opcional)"
                      value={envio.media}
                      onChange={(e) => setEnvio((v) => ({ ...v, media: e.target.value }))}
                    />
                    <Button
                      size="sm"
                      disabled={!envio.texto.trim()}
                      onClick={() => {
                        /* sentinel-disable-next-line promise-sin-catch -- [08AA-26] catch-interno: enviar() captura en use-clientes.ts y expone c.error (renderizado arriba); la promesa nunca rechaza. */
                        void c.enviar(sel.id, envio.texto, envio.media).then(() => setEnvio({ texto: '', media: '' }));
                      }}
                    >
                      Enviar
                    </Button>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
