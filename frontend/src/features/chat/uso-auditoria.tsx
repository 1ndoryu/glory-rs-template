// Uso y auditoría de la dueña (279A-2 F5): tokens por día×remitente
// (exactos del núcleo F0 en `ai`, estima en el resto) y últimas tomas
// humanas con contexto. Tablas simples, sin N+1 (una consulta cada una).

import { useEffect, useState } from 'react';
import { leerAuditoria, leerUso, type AuditoriaFila, type UsoDia } from '../../data/chat/cliente-duena';
import { ErrorApi } from '../../data/inmuebles/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado.';
}

export function UsoAuditoria() {
  const [uso, setUso] = useState<UsoDia[]>([]);
  const [auditoria, setAuditoria] = useState<AuditoriaFila[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [cargando, setCargando] = useState(true);

  useEffect(() => {
    let viva = true;
    Promise.all([leerUso(7), leerAuditoria(50)])
      .then(([u, a]) => {
        if (!viva) return;
        setUso(u);
        setAuditoria(a);
        setError(null);
      })
      .catch((e: unknown) => viva && setError(mensajeError(e)))
      .finally(() => viva && setCargando(false));
    return () => {
      viva = false;
    };
  }, []);

  if (cargando) {
    return (
      <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
        Cargando uso y auditoría…
      </p>
    );
  }

  return (
    <div className="grid gap-4">
      {error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}
      <section>
        <h3 className="mb-2 text-sm font-medium">Uso (7 días, por remitente)</h3>
        <div className="overflow-x-auto rounded-md border">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b bg-muted/50">
                <th className="px-2 py-1">Día</th>
                <th className="px-2 py-1">Remitente</th>
                <th className="px-2 py-1 text-right">Mensajes</th>
                <th className="px-2 py-1 text-right">Est.</th>
                <th className="px-2 py-1 text-right">In</th>
                <th className="px-2 py-1 text-right">Out</th>
              </tr>
            </thead>
            <tbody>
              {uso.map((u, i) => (
                <tr key={`${u.dia}-${u.remitente}-${i}`} className="border-b last:border-0">
                  <td className="px-2 py-1">{u.dia ?? '—'}</td>
                  <td className="px-2 py-1">{u.remitente ?? '—'}</td>
                  <td className="px-2 py-1 text-right">{u.mensajes ?? 0}</td>
                  <td className="px-2 py-1 text-right">{u.tokens_est ?? 0}</td>
                  <td className="px-2 py-1 text-right">{u.tokens_in ?? 0}</td>
                  <td className="px-2 py-1 text-right">{u.tokens_out ?? 0}</td>
                </tr>
              ))}
              {uso.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-2 py-4 text-center text-muted-foreground">
                    Sin uso registrado.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
      <section>
        <h3 className="mb-2 text-sm font-medium">Auditoría (últimas tomas humanas)</h3>
        <ul className="space-y-2">
          {auditoria.map((a) => (
            <li key={a.session_id} className="rounded-md border px-3 py-2 text-xs">
              <span className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{a.nombre || a.telefono || a.session_id.slice(0, 8)}</span>
                {a.estado_atencion && (
                  <Badge variant={a.estado_atencion === 'delegada' ? 'destructive' : 'secondary'} className="text-[10px]">
                    {a.estado_atencion}
                  </Badge>
                )}
                {a.ai_enabled === false && (
                  <Badge variant="outline" className="text-[10px]">
                    manual
                  </Badge>
                )}
                <span className="ml-auto text-muted-foreground">
                  IA {a.ia ?? 0} · humano {a.humano ?? 0} · visitante {a.visitante ?? 0}
                </span>
              </span>
              {a.extracto && <span className="mt-0.5 block truncate text-muted-foreground">{a.extracto}</span>}
            </li>
          ))}
          {auditoria.length === 0 && (
            <li className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
              Sin tomas humanas.
            </li>
          )}
        </ul>
      </section>
      <div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setCargando(true);
            Promise.all([leerUso(7), leerAuditoria(50)])
              .then(([u, a]) => {
                setUso(u);
                setAuditoria(a);
                setError(null);
              })
              .catch((e: unknown) => setError(mensajeError(e)))
              .finally(() => setCargando(false));
          }}
        >
          Recargar
        </Button>
      </div>
    </div>
  );
}
